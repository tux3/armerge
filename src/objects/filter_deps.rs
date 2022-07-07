use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::MergeError;
use rayon::iter::IntoParallelIterator;
use rayon::prelude::*;
use regex::Regex;
use tracing::{event_enabled, info, Level};

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
) -> Result<HashMap<PathBuf, ObjectSyms>, MergeError> {
    let mut object_syms = objects
        .into_par_iter()
        .map(|obj_path| {
            Ok::<_, MergeError>((
                obj_path.to_owned(),
                ObjectSyms::new(obj_path, keep_regexes)?,
            ))
        })
        .collect::<Result<HashMap<PathBuf, ObjectSyms>, _>>()?;
    ObjectSyms::check_dependencies(&mut object_syms);

    let mut required_objs = HashSet::new();
    for (obj_path, obj) in object_syms.iter() {
        if obj.has_exported_symbols {
            if event_enabled!(Level::INFO) {
                let filename = obj_path.file_name().unwrap().to_string_lossy();
                let name_parts = filename.rsplitn(3, '.').collect::<Vec<_>>();
                info!(
                    "Will merge {:?} and its dependencies, as it contains global kept symbols",
                    name_parts[2],
                );
            }
            required_objs.insert(obj_path.clone());
            add_deps_recursive(&mut required_objs, &object_syms, obj);
        }
    }

    if event_enabled!(Level::INFO) {
        for obj in object_syms.keys() {
            if !required_objs.contains(obj) {
                let filename = obj.file_name().unwrap().to_string_lossy();
                let name_parts = filename.rsplitn(3, '.').collect::<Vec<_>>();
                info!(
                    "`{}` is not used by any kept objects, it will be skipped",
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
