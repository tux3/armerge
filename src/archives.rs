use crate::arbuilder::ArBuilder;
use crate::objects::ObjectTempDir;
use ar::Archive;
use rand::distributions::{Alphanumeric, DistString};
use rand::thread_rng;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::from_utf8;
use tempdir::TempDir;

pub fn extract_objects(archives: &[PathBuf]) -> Result<ObjectTempDir, Box<dyn Error>> {
    let dir = TempDir::new("armerge")?;
    let mut objects = Vec::new();

    for archive_path in archives {
        let mut archive = Archive::new(File::open(archive_path)?);
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

            let mut file = File::create(&obj_path)?;
            std::io::copy(&mut entry, &mut file).unwrap();
            objects.push(obj_path);
        }
    }

    Ok(ObjectTempDir { dir, objects })
}

#[cfg(not(target_os = "macos"))]
pub fn create_index(archive_path: &std::path::Path, verbose: bool) -> Result<(), Box<dyn Error>> {
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

pub fn merge(mut output: impl ArBuilder, archives: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    let objects_dir = extract_objects(archives)?;

    for obj_path in objects_dir.objects {
        output.append_obj(obj_path)?;
    }

    output.close()?;
    Ok(())
}
