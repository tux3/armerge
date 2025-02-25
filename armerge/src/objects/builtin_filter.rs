use crate::{ArmergeKeepOrRemove, MergeError};
use regex::Regex;
use std::path::{Path, PathBuf};

use crate::objects::merge;

#[cfg(feature = "objpoke_symbols")]
pub fn merge_required_objects(
    _obj_dir: &Path,
    merged_path: &Path,
    objects: &[PathBuf],
    keep_or_remove: ArmergeKeepOrRemove,
    regexes: &[Regex],
) -> Result<(), MergeError> {
    if keep_or_remove == ArmergeKeepOrRemove::RemoveSymbols {
        unimplemented!("--remove-symbols not yet supported with builtin filter")
    }

    // The merging part is still not builtin, it has to be done by a real linker
    merge::create_merged_object(merged_path, &[], objects, false)?;

    // Filtering the symbols is faster in pure Rust, compared to calling the system's objcopy
    let merged_elf = std::fs::read(merged_path)?;
    let filtered_elf =
        objpoke::elf::localize_elf_symbols(merged_elf, regexes).map_err(|e| MergeError::InternalError(e.into()))?;

    // If a symbol we localize is in a COMDAT section group, we also want to turn it into a regular
    // section group. Otherwise the local symbol is not really local, because the containing section
    // could later get COMDAT-folded with other (potentially incompatible) object files.
    let filtered_elf =
        objpoke::elf::demote_comdat_groups(filtered_elf, regexes).map_err(|e| MergeError::InternalError(e.into()))?;

    std::fs::write(merged_path, filtered_elf)?;
    Ok(())
}
