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
) -> Result<(), Box<dyn Error>> {
    // The merging part is still not builtin, it has to be done by a real linker
    merge::create_merged_object(merged_path, &[], objects.keys(), verbose)?;

    // Filtering the symbols is faster in pure Rust, compared to calling the system's objcopy
    let merged_elf = std::fs::read(merged_path)?;
    let filtered_elf = objpoke::elf::localize_elf_symbols(merged_elf, keep_regexes)?;
    std::fs::write(merged_path, filtered_elf)?;
    Ok(())
}
