use goblin::{peek_bytes, Hint};
use std::{
    collections::HashSet,
    ffi::OsString,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use crate::{objects::merge::create_merged_object, ArmergeKeepOrRemove, MergeError};
use object::{Object, ObjectSymbol, SymbolKind};
use regex::Regex;
use std::fs::File;
use tracing::info;

pub fn create_filtered_merged_object(
    merged_path: &Path,
    objects: impl IntoIterator<Item = impl AsRef<Path>>,
    filter_list: &Path,
) -> Result<(), MergeError> {
    create_merged_object(merged_path, &[], objects, false)?;
    filter_symbols(merged_path, filter_list)?;

    Ok(())
}

fn create_filtered_merged_macho_object(
    merged_path: &Path,
    objects: impl IntoIterator<Item = impl AsRef<Path>>,
    filter_list: &Path,
) -> Result<(), MergeError> {
    let extra_args = &["-unexported_symbols_list".as_ref(), filter_list.as_os_str()];
    let merged_firstpass_path = merged_path.parent().unwrap().join("merged_firstpass.o");
    create_merged_object(&merged_firstpass_path, extra_args, objects, false)?;
    create_merged_object(merged_path, &[], [&merged_firstpass_path], true)?;

    Ok(())
}

pub fn create_symbol_filter_list(
    object_dir: &Path,
    objects: impl IntoIterator<Item = impl AsRef<Path>>,
    keep_or_remove: ArmergeKeepOrRemove,
    regexes: &[Regex],
) -> Result<PathBuf, MergeError> {
    let filter_path = object_dir.join("localize.syms");
    let mut filter_syms = HashSet::new();
    let mut kept_count = 0;

    for object_path in objects.into_iter() {
        let object_path = object_path.as_ref();
        let data = std::fs::read(object_path)?;
        let file = object::File::parse(data.as_slice())
            .map_err(|e| MergeError::InvalidObject { path: object_path.to_owned(), inner: e })?;
        'next_symbol: for sym in file.symbols() {
            if keep_or_remove == ArmergeKeepOrRemove::KeepSymbols
                && (!sym.is_global()
                    || sym.is_undefined()
                    || (sym.kind() != SymbolKind::Text
                        && sym.kind() != SymbolKind::Data
                        && sym.kind() != SymbolKind::Unknown/* ASM functions often end up unknown */))
            {
                continue;
            }
            if let Ok(name) = sym.name() {
                for regex in regexes {
                    if regex.is_match(name) {
                        if keep_or_remove == ArmergeKeepOrRemove::KeepSymbols {
                            kept_count += 1;
                        } else {
                            filter_syms.insert(name.to_owned());
                        }
                        continue 'next_symbol;
                    }
                }

                if keep_or_remove == ArmergeKeepOrRemove::KeepSymbols {
                    filter_syms.insert(name.to_owned());
                } else {
                    kept_count += 1;
                }
            }
        }
    }
    info!(
        "Localizing {} symbols, keeping {} globals",
        filter_syms.len(),
        kept_count
    );

    let mut filter_file = File::create(&filter_path)?;
    for sym_name in filter_syms {
        filter_file.write_all(sym_name.as_bytes())?;
        filter_file.write_all(b"\n")?;
    }

    Ok(filter_path)
}

fn filter_symbols(object_path: &Path, filter_list_path: &Path) -> Result<(), MergeError> {
    let objcopy_path = if let Some(var) = std::env::var_os("OBJCOPY") {
        var
    } else {
        OsString::from_str("llvm-objcopy").unwrap()
    };

    let args = vec![
        OsString::from("--localize-symbols"),
        filter_list_path.as_os_str().to_owned(),
        object_path.as_os_str().to_owned(),
    ];
    info!(
        "{} {}",
        objcopy_path.to_string_lossy(),
        args.iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>().join(" ")
    );

    let output = Command::new(&objcopy_path)
        .args(&args)
        .output()
        .map_err(|e| MergeError::ExternalToolLaunchError {
            tool: objcopy_path.to_string_lossy().to_string(),
            inner: e,
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(MergeError::ExternalToolError {
            reason: "Failed to filter symbols".to_string(),
            tool: objcopy_path.to_string_lossy().to_string(),
            args,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

pub fn merge_required_macho_objects(
    obj_dir: &Path,
    merged_path: &Path,
    objects: &[PathBuf],
    keep_or_remove: ArmergeKeepOrRemove,
    regexes: &[Regex],
) -> Result<(), MergeError> {
    let filter_path = create_symbol_filter_list(obj_dir, objects, keep_or_remove, regexes)?;
    create_filtered_merged_macho_object(merged_path, objects, &filter_path)
}

pub fn merge_required_objects(
    obj_dir: &Path,
    merged_path: &Path,
    objects: &[PathBuf],
    keep_or_remove: ArmergeKeepOrRemove,
    regexes: &[Regex],
) -> Result<(), MergeError> {
    let filter_path = create_symbol_filter_list(obj_dir, objects, keep_or_remove, regexes)?;
    create_filtered_merged_object(merged_path, objects, &filter_path)?;

    // If a symbol we localize is in a COMDAT section group, we also want to turn it into a regular
    // section group. Otherwise the local symbol is not really local, because the containing section
    // could later get COMDAT-folded with other (potentially incompatible) object files.
    demote_elf_comdats(merged_path, regexes)
}

fn demote_elf_comdats(merged_path: &Path, keep_regexes: &[Regex]) -> Result<(), MergeError> {
    let mut file = File::open(merged_path)?;
    let hint_bytes = &mut [0u8; 16];
    file.read_exact(hint_bytes)?;
    file.seek(SeekFrom::Start(0))?;

    let new_data = {
        match peek_bytes(hint_bytes) {
            Ok(Hint::Elf(_)) => {
                info!(
                    "Automatically demoting ELF COMDAT section groups in {}",
                    merged_path.display()
                );

                let mut data = Vec::new();
                file.read_to_end(&mut data)?;
                objpoke::elf::demote_comdat_groups(data, keep_regexes)
                    .map_err(|e| MergeError::InternalError(e.into()))?
            },
            // We don't know about needing to demote any COMDATs in PE/Mach-O files
            Ok(Hint::Mach(_) | Hint::MachFat(_)) => return Ok(()),
            Ok(Hint::PE) => return Ok(()),
            Ok(_) => return Ok(()),
            Err(_) => return Ok(()), // Goblin probably just doesn't understand this format
        }
    };

    drop(file);
    std::fs::write(merged_path, new_data)?;
    Ok(())
}
