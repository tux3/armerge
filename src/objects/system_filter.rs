use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use object::{Object, ObjectSymbol, SymbolKind};
use regex::Regex;

use crate::objects::merge::create_merged_object;
use crate::objects::syms::ObjectSyms;

#[cfg(not(target_os = "macos"))]
pub fn create_filtered_merged_object(
    merged_path: &Path,
    objects: impl IntoIterator<Item = impl AsRef<Path>>,
    filter_list: &Path,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    create_merged_object(&merged_path, &[], objects, verbose)?;
    filter_symbols(&merged_path, &filter_list, verbose)?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn create_filtered_merged_object(
    merged_path: &Path,
    objects: impl IntoIterator<Item = impl AsRef<Path>>,
    filter_list: &Path,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let extra_args = &[
        "-unexported_symbols_list".to_owned(),
        filter_list.to_str().unwrap().to_owned(),
    ];
    let merged_firstpass_path = merged_path.parent().unwrap().join("merged_firstpass.o");
    create_merged_object(&merged_firstpass_path, extra_args, objects, verbose)?;
    create_merged_object(&merged_path, &[], &[&merged_firstpass_path], false)?;

    Ok(())
}

pub fn create_symbol_filter_list(
    object_dir: &Path,
    objects: impl IntoIterator<Item = impl AsRef<Path>>,
    keep_regexes: &[Regex],
    verbose: bool,
) -> Result<PathBuf, Box<dyn Error>> {
    let filter_path = object_dir.join("localize.syms");
    let mut filter_syms = HashSet::new();
    let mut kept_count = 0;

    for object_path in objects.into_iter() {
        let data = std::fs::read(object_path)?;
        let file = object::File::parse(&data)?;
        'next_symbol: for sym in file.symbols() {
            if !sym.is_global()
                || sym.is_undefined()
                || (sym.kind() != SymbolKind::Text && sym.kind() != SymbolKind::Data)
            {
                continue;
            }
            if let Ok(name) = sym.name() {
                for regex in keep_regexes {
                    if regex.is_match(name) {
                        kept_count += 1;
                        continue 'next_symbol;
                    }
                }

                filter_syms.insert(name.to_owned());
            }
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

#[cfg(not(target_os = "macos"))]
fn filter_symbols(
    object_path: &Path,
    filter_list_path: &Path,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let objcopy_path = if let Some(var) = std::env::var_os("OBJCOPY") {
        var
    } else {
        OsString::from_str("objcopy").unwrap()
    };

    let args = vec![
        OsString::from("--localize-symbols"),
        filter_list_path.as_os_str().to_owned(),
        object_path.as_os_str().to_owned(),
    ];
    if verbose {
        println!(
            "{} {}",
            objcopy_path.to_string_lossy(),
            args.iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    let output = Command::new(objcopy_path).args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        std::io::stdout().write_all(&output.stdout).unwrap();
        std::io::stderr().write_all(&output.stderr).unwrap();
        panic!("Failed to filter symbols with objcopy")
    }
}

pub fn merge_required_objects(
    obj_dir: &Path,
    merged_path: &Path,
    objects: &HashMap<PathBuf, ObjectSyms>,
    keep_regexes: &[Regex],
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let filter_path = create_symbol_filter_list(obj_dir, objects.keys(), &keep_regexes, verbose)?;
    create_filtered_merged_object(&merged_path, objects.keys(), &filter_path, verbose)
}
