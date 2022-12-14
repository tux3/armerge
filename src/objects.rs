mod filter_deps;
mod merge;
mod syms;

#[cfg(feature = "objpoke_symbols")]
mod builtin_filter;
mod system_filter;

use crate::arbuilder::ArBuilder;
use crate::objects::syms::ObjectSyms;
use crate::{ArchiveContents, ArmergeKeepOrRemove, MergeError};
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct ObjectTempDir {
    pub dir: TempDir,
    pub objects: Vec<PathBuf>,
}

pub fn merge_required_objects(
    contents_type: ArchiveContents,
    obj_dir: &Path,
    merged_path: &Path,
    objs: &HashMap<PathBuf, ObjectSyms>,
    keep_or_remove: ArmergeKeepOrRemove,
    regexes: &[Regex],
) -> Result<(), MergeError> {
    #[allow(clippy::if_same_then_else)] // Clippy can't see both [cfg] at once
    if contents_type == ArchiveContents::Elf {
        #[cfg(feature = "objpoke_symbols")]
        builtin_filter::merge_required_objects(obj_dir, merged_path, objs, keep_or_remove, keeps)?;
        #[cfg(not(feature = "objpoke_symbols"))]
        system_filter::merge_required_objects(obj_dir, merged_path, objs, keep_or_remove, regexes)?;
    } else if contents_type == ArchiveContents::MachO {
        system_filter::merge_required_macho_objects(
            obj_dir,
            merged_path,
            objs,
            keep_or_remove,
            regexes,
        )?;
    } else {
        system_filter::merge_required_objects(obj_dir, merged_path, objs, keep_or_remove, regexes)?;
    }
    Ok(())
}

pub fn merge(
    mut output: Box<dyn ArBuilder>,
    contents_type: ArchiveContents,
    objects: ObjectTempDir,
    keep_or_remove: ArmergeKeepOrRemove,
    mut regexes: Vec<Regex>,
) -> Result<(), MergeError> {
    let merged_name = "merged.o";
    let mut merged_path = objects.dir.path().to_owned();
    merged_path.push(merged_name);

    if keep_or_remove == ArmergeKeepOrRemove::KeepSymbols {
        // When filtering symbols to keep just the public API visible,
        // we must make an exception for the unwind symbols (if linked statically)
        regexes.push(Regex::new("^_?_Unwind_.*").expect("Failed to compile Regex"));
    }

    let required_objects =
        filter_deps::filter_required_objects(&objects.objects, keep_or_remove, &regexes)?;

    if required_objects.is_empty() {
        return Err(MergeError::NoObjectsLeft);
    }

    if keep_or_remove == ArmergeKeepOrRemove::KeepSymbols {
        // When filtering symbols to keep just the public API visible,
        // we must make an exception for the unwind symbols (if linked statically)
        // However, some symbols are not indicative of the fact that we need to keep an object file
        regexes.push(Regex::new("_?__g.._personality_.*").expect("Failed to compile Regex"));
    }

    merge_required_objects(
        contents_type,
        objects.dir.path(),
        &merged_path,
        &required_objects,
        keep_or_remove,
        &regexes,
    )?;

    output.append_obj(&merged_path)?;
    output.close()?;

    Ok(())
}
