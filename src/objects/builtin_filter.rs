use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::objects::merge;
use crate::objects::syms::ObjectSyms;

pub fn merge_required_objects(
    _obj_dir: &Path,
    merged_path: &Path,
    objects: &HashMap<PathBuf, ObjectSyms>,
    keep_regexes: &[Regex],
    verbose: bool,
) -> Result<()> {
    // The merging part is still not builtin, it has to be done by a real linker
    merge::create_merged_object(merged_path, &[], objects.keys(), verbose)?;

    // Filtering the symbols is faster in pure Rust, compared to calling the system's objcopy
    let merged_elf = std::fs::read(merged_path)?;
    let filtered_elf = objpoke::elf::localize_elf_symbols(merged_elf, keep_regexes)?;

    // If a symbol we localize is in a COMDAT section group, we also want to turn it into a regular
    // section group. Otherwise the local symbol is not really local, because the containing section
    // could later get COMDAT-folded with other (potentially incompatible) object files.
    let filtered_elf = objpoke::elf::demote_comdat_groups(filtered_elf, keep_regexes)?;

    std::fs::write(merged_path, filtered_elf)?;
    Ok(())
}
