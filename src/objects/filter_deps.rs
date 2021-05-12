use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use rayon::iter::IntoParallelIterator;
use rayon::prelude::*;
use regex::Regex;

use crate::objects::syms::ObjectSyms;

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

pub fn filter_required_objects(
    objects: &[PathBuf],
    keep_regexes: &[Regex],
    verbose: bool,
) -> HashMap<PathBuf, ObjectSyms> {
    let mut object_syms = objects
        .into_par_iter()
        .map(|obj_path| {
            (
                obj_path.to_owned(),
                ObjectSyms::new(&obj_path, keep_regexes)
                    .map_err(|e| {
                        Box::new(format!(
                            "Failed to open object {}: {}",
                            obj_path.display(),
                            e
                        ))
                    })
                    .unwrap(),
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

    object_syms
        .into_iter()
        .filter(|(obj_path, _)| required_objs.contains(obj_path))
        .collect()
}
