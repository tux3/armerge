use crate::arbuilder::ArBuilder;
use crate::input_library::InputLibrary;
use crate::objects::ObjectTempDir;
use crate::MergeError::ExternalToolLaunchError;
use crate::{MergeError, ProcessInputError};
use ar::Archive;
use goblin::{peek_bytes, Hint};
use rand::distributions::{Alphanumeric, DistString};
use rand::thread_rng;
use rayon::prelude::*;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{Read, Write};
use tracing::info;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ArchiveContents {
    /// Only ELF files
    Elf,
    /// Only Mach-O files
    MachO,
    /// Only unsupported files (e.g. PE/COFF)
    Other,

    /// Archives contain a mix of file types
    Mixed,
    /// No contents
    Empty,
}

impl ArchiveContents {
    pub(crate) fn merge(a: ArchiveContents, b: ArchiveContents) -> ArchiveContents {
        #[allow(clippy::if_same_then_else)] // Two of the cases return `a`, that's okay
        if a == ArchiveContents::Mixed || b == ArchiveContents::Mixed {
            ArchiveContents::Mixed
        } else if a == ArchiveContents::Empty {
            b
        } else if b == ArchiveContents::Empty {
            a
        } else if a == b {
            a
        } else {
            ArchiveContents::Mixed
        }
    }
}

pub struct ExtractedArchive {
    pub object_dir: ObjectTempDir,
    pub contents_type: ArchiveContents,
}

impl Debug for ExtractedArchive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractedArchive")
            .field("object_dir", &self.object_dir.dir.path())
            .field("contents_type", &self.contents_type)
            .finish()
    }
}

fn archive_object_type(object_header: &[u8; 16]) -> ArchiveContents {
    match peek_bytes(object_header) {
        Ok(Hint::Elf(_)) => ArchiveContents::Elf,
        Ok(Hint::Mach(_) | Hint::MachFat(_)) => ArchiveContents::MachO,
        Ok(_) => ArchiveContents::Other,
        Err(_) => ArchiveContents::Other, // Malformed input
    }
}

pub fn extract_objects<I: IntoParallelIterator<Item = InputLibrary<R>>, R: Read>(
    input_libraries: I,
) -> Result<ExtractedArchive, ProcessInputError> {
    let dir = tempfile::Builder::new()
        .prefix("armerge.")
        .tempdir()
        .map_err(ProcessInputError::TempDir)?;

    let (objects, archive_contents) = input_libraries
        .into_par_iter()
        .try_fold(
            || (Vec::new(), ArchiveContents::Empty),
            |(mut objects, mut archive_contents), input_lib| {
                let mut archive = Archive::new(input_lib.reader);
                while let Some(entry_result) = archive.next_entry() {
                    let mut entry =
                        entry_result.map_err(|e| ProcessInputError::ReadingArchive {
                            name: input_lib.name.clone(),
                            inner: e,
                        })?;

                    let rnd: String = Alphanumeric.sample_string(&mut thread_rng(), 8);
                    let mut obj_path = dir.path().to_owned();
                    obj_path.push(format!(
                        "{}@{}.{}.o",
                        input_lib.name,
                        String::from_utf8_lossy(entry.header().identifier()),
                        &rnd
                    ));

                    let hint_bytes = &mut [0u8; 16];
                    entry.read_exact(hint_bytes).map_err(|e| {
                        ProcessInputError::ReadingArchive {
                            name: input_lib.name.clone(),
                            inner: e,
                        }
                    })?;
                    let obj_type = archive_object_type(hint_bytes);
                    archive_contents = ArchiveContents::merge(archive_contents, obj_type);

                    let mut file = File::create(&obj_path).map_err(|e| {
                        ProcessInputError::ExtractingObject {
                            path: obj_path.to_owned(),
                            inner: e,
                        }
                    })?;
                    file.write_all(hint_bytes).map_err(|e| {
                        ProcessInputError::ExtractingObject {
                            path: obj_path.to_owned(),
                            inner: e,
                        }
                    })?;
                    std::io::copy(&mut entry, &mut file).map_err(|e| {
                        ProcessInputError::ExtractingObject {
                            path: obj_path.to_owned(),
                            inner: e,
                        }
                    })?;
                    objects.push(obj_path);
                }

                Ok((objects, archive_contents))
            },
        )
        .try_reduce(
            || (Vec::new(), ArchiveContents::Empty),
            |(mut objs_a, contents_a), (mut objs_b, contents_b)| {
                objs_a.append(&mut objs_b);
                Ok((objs_a, ArchiveContents::merge(contents_a, contents_b)))
            },
        )?;

    Ok(ExtractedArchive {
        object_dir: ObjectTempDir { dir, objects },
        contents_type: archive_contents,
    })
}

pub fn create_index(archive_path: &std::path::Path) -> Result<(), MergeError> {
    use std::process::Command;

    info!("ranlib {}", archive_path.to_string_lossy());

    let output = Command::new("ranlib")
        .args(vec![archive_path])
        .output()
        .map_err(|e| ExternalToolLaunchError {
            tool: "ranlib".to_string(),
            inner: e,
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(MergeError::ExternalToolError {
            reason: "Failed to create archive index with `ranlib`".to_string(),
            tool: "ranlib".to_string(),
            args: archive_path.iter().map(|p| p.to_owned()).collect(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

pub fn merge(mut output: Box<dyn ArBuilder>, objects_dir: ObjectTempDir) -> Result<(), MergeError> {
    for obj_path in objects_dir.objects {
        output.append_obj(obj_path.as_path())?;
    }
    output.close()?;
    Ok(())
}
