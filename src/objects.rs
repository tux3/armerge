mod filter_deps;
mod merge;
mod syms;

#[cfg(all(feature = "objpoke", any(target_os = "linux", target_os = "android")))]
mod builtin_filter;
#[cfg(all(feature = "objpoke", any(target_os = "linux", target_os = "android")))]
use crate::objects::builtin_filter::merge_required_objects;

#[cfg(not(all(feature = "objpoke", any(target_os = "linux", target_os = "android"))))]
mod system_filter;
#[cfg(not(all(feature = "objpoke", any(target_os = "linux", target_os = "android"))))]
use crate::objects::system_filter::merge_required_objects;

use crate::arbuilder::ArBuilder;
use regex::Regex;
use std::error::Error;
use std::path::PathBuf;
use tempdir::TempDir;

pub struct ObjectTempDir {
    pub dir: TempDir,
    pub objects: Vec<PathBuf>,
}

pub fn merge(
    mut output: impl ArBuilder,
    objects: ObjectTempDir,
    keep_regexes: Vec<String>,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let merged_name = "merged.o";
    let mut merged_path = objects.dir.path().to_owned();
    merged_path.push(merged_name);

    let mut keep_regexes = keep_regexes
        .into_iter()
        .map(|r| Regex::new(&r))
        .collect::<Result<Vec<_>, _>>()?;

    // When filtering symbols to keep just the public API visible,
    // we must make an exception for the unwind symbols (if linked statically)
    keep_regexes.push(Regex::new("^_?_Unwind_.*")?);

    let required_objects =
        filter_deps::filter_required_objects(&objects.objects, &keep_regexes, verbose);

    if required_objects.is_empty() {
        panic!("Zero objects left after filtering! Make sure to keep at least one public symbol.");
    }

    // When filtering symbols to keep just the public API visible,
    // we must make an exception for the unwind symbols (if linked statically)
    // However, some symbols are not indicative of the fact that we need to keep an object file
    keep_regexes.push(Regex::new("_?__g.._personality_.*")?);

    merge_required_objects(
        objects.dir.path(),
        &merged_path,
        &required_objects,
        &keep_regexes,
        verbose,
    )?;

    output.append_obj(merged_path)?;
    output.close()?;

    Ok(())
}
