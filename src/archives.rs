use crate::arbuilder::ArBuilder;
use crate::objects::ObjectTempDir;
use ar::Archive;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::str::from_utf8;
use tempdir::TempDir;

pub fn extract_objects(archives: &[PathBuf]) -> Result<ObjectTempDir, Box<dyn Error>> {
    let dir = TempDir::new("armerge")?;
    let mut objects = Vec::new();

    for archive_path in archives {
        let mut archive = Archive::new(File::open(archive_path)?);
        while let Some(entry_result) = archive.next_entry() {
            let mut entry = entry_result?;

            let rnd: String = thread_rng().sample_iter(&Alphanumeric).take(8).collect();
            let mut obj_path = dir.path().to_owned();
            obj_path.push(from_utf8(entry.header().identifier())?.to_string() + "." + &rnd + ".o");

            let mut file = File::create(&obj_path)?;
            std::io::copy(&mut entry, &mut file).unwrap();
            objects.push(obj_path);
        }
    }

    Ok(ObjectTempDir { dir, objects })
}

#[cfg(not(target_os = "macos"))]
pub fn create_index(archive_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
    use std::process::Command;

    Command::new("ranlib")
        .args(vec![archive_path])
        .status()
        .expect("Failed to create archive index with `ranlib`");

    Ok(())
}

pub fn merge(mut output: impl ArBuilder, archives: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    let objects_dir = extract_objects(archives)?;

    for obj_path in objects_dir.objects {
        output.append_obj(obj_path)?;
    }

    output.close()?;
    Ok(())
}
