use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
};

use crate::{ArmergeKeepOrRemove, MergeError};
use rayon::prelude::*;
use regex::Regex;
use tracing::{event_enabled, info, Level};

use crate::{archives::get_object_name_from_path, objects::syms::ObjectSyms};

fn add_deps_recursive(objs_set: &mut HashSet<PathBuf>, syms: &BTreeMap<PathBuf, ObjectSyms>, obj: &ObjectSyms) {
    for dep in &obj.deps {
        if objs_set.insert(dep.to_owned()) {
            add_deps_recursive(objs_set, syms, syms.get(dep).unwrap());
        }
    }
}

pub fn filter_required_objects(
    objects: &[PathBuf],
    keep_or_remove: ArmergeKeepOrRemove,
    regexes: &[Regex],
) -> Result<BTreeMap<PathBuf, ObjectSyms>, MergeError> {
    let mut object_syms = objects
        .into_par_iter()
        .map(|obj_path| Ok::<_, MergeError>((obj_path.to_owned(), ObjectSyms::new(obj_path, keep_or_remove, regexes)?)))
        .collect::<Result<BTreeMap<PathBuf, ObjectSyms>, _>>()?;
    ObjectSyms::check_dependencies(&mut object_syms);

    let mut required_objs = HashSet::new();
    for (obj_path, obj) in object_syms.iter() {
        if obj.has_exported_symbols {
            if event_enabled!(Level::INFO) {
                info!(
                    "Will merge {:?} and its dependencies, as it contains global kept symbols",
                    get_object_name_from_path(obj_path),
                );
            }
            required_objs.insert(obj_path.clone());
            add_deps_recursive(&mut required_objs, &object_syms, obj);
        }
    }

    if event_enabled!(Level::INFO) {
        for obj in object_syms.keys() {
            if !required_objs.contains(obj) {
                info!(
                    "`{}` is not used by any kept objects, it will be skipped",
                    get_object_name_from_path(obj)
                )
            }
        }
    }

    Ok(object_syms
        .into_iter()
        .filter(|(obj_path, _)| required_objs.contains(obj_path))
        .collect())
}
