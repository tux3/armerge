use crate::object_syms::ObjectSyms;
use ar::{Builder, Header};
use object::{Object, SymbolKind};
use rayon::prelude::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use tempdir::TempDir;

pub struct ObjectTempDir {
    pub dir: TempDir,
    pub objects: Vec<PathBuf>,
}

fn add_deps_recursive(
    objs_set: &mut HashSet<PathBuf>,
    syms: &HashMap<PathBuf, ObjectSyms>,
    obj: &ObjectSyms,
) {
    for dep in &obj.deps {
        if objs_set.insert(dep.to_owned()) {
            add_deps_recursive(objs_set, syms, syms.get(dep).unwrap());
        }
    }
}

fn filter_required_objects(
    objects: &[PathBuf],
    keep_regexes: &[Regex],
    verbose: bool,
) -> Result<HashMap<PathBuf, ObjectSyms>, Box<dyn Error>> {
    let mut object_syms = objects
        .into_par_iter()
        .map(|obj_path| {
            (
                obj_path.to_owned(),
                ObjectSyms::new(&obj_path, keep_regexes).unwrap(),
            )
        })
        .collect::<HashMap<PathBuf, ObjectSyms>>();
    ObjectSyms::check_dependencies(&mut object_syms);

    let mut required_objs = HashSet::new();
    for (obj_path, obj) in object_syms.iter() {
        if obj.has_exported_symbols {
            if verbose {
                let filename = obj_path.file_name().unwrap().to_string_lossy();
                let name_parts = filename.rsplitn(3, '.').collect::<Vec<_>>();
                println!(
                    "Will merge {:?} and its dependencies, as it contains global kept symbols",
                    name_parts[2],
                );
            }
            required_objs.insert(obj_path.clone());
            add_deps_recursive(&mut required_objs, &object_syms, obj);
        }
    }

    if verbose {
        for obj in object_syms.keys() {
            if !required_objs.contains(obj) {
                let filename = obj.file_name().unwrap().to_string_lossy();
                let name_parts = filename.rsplitn(3, '.').collect::<Vec<_>>();
                println!(
                    "note: `{}` is not used by any kept objects, it will be skipped",
                    name_parts[2]
                )
            }
        }
    }

    Ok(object_syms
        .into_iter()
        .filter(|(obj_path, _)| required_objs.contains(obj_path))
        .collect())
}

fn create_merged_object(
    merged_path: &Path,
    objects: impl IntoIterator<Item = impl AsRef<Path>>,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let ld_path = if let Some(ld_var) = std::env::var_os("LD") {
        ld_var
    } else {
        OsString::from_str("ld").unwrap()
    };
    let mut args = [
        OsString::from("-r"),
        OsString::from("-o"),
        merged_path.as_os_str().to_owned(),
    ]
    .to_vec();
    let mut count = 0;
    args.extend(
        objects
            .into_iter()
            .inspect(|_| count += 1)
            .map(|p| p.as_ref().as_os_str().into()),
    );
    if verbose {
        println!("Merging {} objects", count);
    }

    Command::new(&ld_path)
        .args(args)
        .status()
        .unwrap_or_else(|_| {
            panic!(
                "Failed to merged object files with `{}`",
                ld_path.to_string_lossy()
            )
        });

    Ok(())
}

fn create_filter_list(
    object_path: &Path,
    keep_regexes: &[Regex],
    verbose: bool,
) -> Result<PathBuf, Box<dyn Error>> {
    let filter_path = object_path.parent().unwrap().join("localize.syms");
    let mut filter_syms = HashSet::new();

    let data = std::fs::read(object_path)?;
    let file = object::File::parse(&data)?;
    let mut kept_count = 0;
    'next_symbol: for (_idx, sym) in file.symbols() {
        if !sym.is_global()
            || sym.is_weak()
            || sym.is_undefined()
            || (sym.kind() != SymbolKind::Text && sym.kind() != SymbolKind::Data)
        {
            continue;
        }
        if let Some(name) = sym.name() {
            for regex in keep_regexes {
                if regex.is_match(name) {
                    kept_count += 1;
                    continue 'next_symbol;
                }
            }

            filter_syms.insert(name);
        }
    }
    if verbose {
        println!(
            "Localizing {} symbols, keeping {} globals",
            filter_syms.len(),
            kept_count
        );
    }

    let mut filter_file = std::fs::File::create(&filter_path)?;
    for sym_name in filter_syms {
        filter_file.write_all(sym_name.as_bytes())?;
        filter_file.write_all(b"\n")?;
    }

    Ok(filter_path)
}

fn filter_symbols(object_path: &Path, filter_list_path: &Path) -> Result<(), Box<dyn Error>> {
    let args = vec![
        OsString::from("--localize-symbols"),
        filter_list_path.as_os_str().to_owned(),
        object_path.as_os_str().to_owned(),
    ];
    Command::new("llvm-objcopy")
        .args(args)
        .status()
        .expect("Failed to filter symbols with `llvm-objcopy`");

    Ok(())
}

pub fn merge<T: Write>(
    mut output: Builder<T>,
    objects: ObjectTempDir,
    keep_regexes: Vec<String>,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let merged_name = "merged.o";
    let mut merged_path = objects.dir.path().to_owned();
    merged_path.push(merged_name);

    let keep_regexes = keep_regexes
        .into_iter()
        .map(|r| Regex::new(&r))
        .collect::<Result<Vec<_>, _>>()?;

    let required_objects = filter_required_objects(&objects.objects, &keep_regexes, verbose)?;
    create_merged_object(&merged_path, required_objects.keys(), verbose)?;

    let filter_path = create_filter_list(&merged_path, &keep_regexes, verbose)?;
    filter_symbols(&merged_path, &filter_path)?;

    let obj = File::open(&merged_path)?;
    let header = Header::new(
        merged_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .as_bytes()
            .to_vec(),
        obj.metadata()?.len(),
    );
    output.append(&header, &obj)?;

    Ok(())
}
