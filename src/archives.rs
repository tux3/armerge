use crate::arbuilder::ArBuilder;
use crate::objects::ObjectTempDir;
use anyhow::{Context, Result};
use ar::{Archive, Entry};
use goblin::{peek_bytes, Hint};
use rand::distributions::{Alphanumeric, DistString};
use rand::thread_rng;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::str::from_utf8;
use tempdir::TempDir;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ArchiveContents {
    Elf,
    MachO,
    Other, // E.g. PE files

    Empty,
    Mixed,
}

pub struct ExtractedArchive {
    pub object_dir: ObjectTempDir,
    pub contents_type: ArchiveContents,
}

fn archive_object_type(object: &mut Entry<File>) -> Result<ArchiveContents> {
    let hint_bytes = &mut [0u8; 16];
    object.read_exact(hint_bytes)?;
    object.seek(SeekFrom::Start(0))?;

    Ok(match peek_bytes(hint_bytes)? {
        Hint::Elf(_) => ArchiveContents::Elf,
        Hint::Mach(_) | Hint::MachFat(_) => ArchiveContents::MachO,
        _ => ArchiveContents::Other,
    })
}

pub fn extract_objects(archives: &[PathBuf]) -> Result<ExtractedArchive> {
    let dir = TempDir::new("armerge")?;
    let mut objects = Vec::new();
    let mut archive_contents = ArchiveContents::Empty;

    for archive_path in archives {
        let mut archive =
            Archive::new(File::open(archive_path).with_context(|| {
                format!("Failed to open input file '{}'", archive_path.display())
            })?);
        let archive_name = archive_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .replace('/', "_");
        while let Some(entry_result) = archive.next_entry() {
            let mut entry = entry_result?;

            let rnd: String = Alphanumeric.sample_string(&mut thread_rng(), 8);
            let mut obj_path = dir.path().to_owned();
            obj_path.push(format!(
                "{}@{}.{}.o",
                archive_name,
                from_utf8(entry.header().identifier())?,
                &rnd
            ));

            let obj_type = archive_object_type(&mut entry)?;
            if archive_contents == ArchiveContents::Empty {
                archive_contents = obj_type;
            } else if archive_contents != obj_type {
                archive_contents = ArchiveContents::Mixed
            }

            let mut file = File::create(&obj_path)?;
            std::io::copy(&mut entry, &mut file).unwrap();
            objects.push(obj_path);
        }
    }

    Ok(ExtractedArchive {
        object_dir: ObjectTempDir { dir, objects },
        contents_type: archive_contents,
    })
}

pub fn create_index(archive_path: &std::path::Path, verbose: bool) -> Result<()> {
    use std::process::Command;

    if verbose {
        println!("ranlib {}", archive_path.to_string_lossy());
    }

    let output = Command::new("ranlib").args(vec![archive_path]).output()?;
    if output.status.success() {
        Ok(())
    } else {
        std::io::stdout().write_all(&output.stdout).unwrap();
        std::io::stderr().write_all(&output.stderr).unwrap();
        panic!("Failed to create archive index with `ranlib`")
    }
}

pub fn merge(mut output: Box<dyn ArBuilder>, objects_dir: ObjectTempDir) -> Result<()> {
    for obj_path in objects_dir.objects {
        output.append_obj(obj_path.as_path())?;
    }
    output.close()?;
    Ok(())
}
