use ar::{Builder, Header};
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use tempdir::TempDir;

pub struct ObjectTempDir {
    pub dir: TempDir,
    pub objects: Vec<PathBuf>,
}

fn create_merged_object(merged_path: &Path, objects: Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    let ld_path = if let Some(ld_var) = std::env::var_os("LD") {
        ld_var
    } else {
        OsString::from_str("ld").unwrap()
    };
    let mut args = [
        OsString::from("-r"),
        OsString::from("-o"),
        merged_path.as_os_str().to_owned(),
    ]
    .to_vec();
    args.extend(objects.into_iter().map(|p| p.into_os_string()));
    Command::new(&ld_path).args(args).status().expect(&format!(
        "Failed to merged object files with `{}`",
        ld_path.to_string_lossy()
    ));

    Ok(())
}

fn filter_symbols(object_path: &Path, keep_regexes: Vec<String>) -> Result<(), Box<dyn Error>> {
    let localize_args = [
        OsString::from("--regex"),
        OsString::from("--localize-symbol"),
        OsString::from(".*"),
        object_path.as_os_str().to_owned(),
    ]
    .to_vec();
    Command::new("llvm-objcopy")
        .args(localize_args)
        .status()
        .expect("Failed to localize symbols with `llvm-objcopy`");

    let mut globalize_args = [OsString::from("--regex")].to_vec();
    for regex in keep_regexes {
        globalize_args.push("--globalize-symbol".into());
        globalize_args.push(regex.into());
    }
    globalize_args.push(object_path.as_os_str().to_owned());
    Command::new("llvm-objcopy")
        .args(globalize_args)
        .status()
        .expect("Failed to globalize symbols with `llvm-objcopy`");

    Ok(())
}

pub fn merge<T: Write>(
    mut output: Builder<T>,
    objects: ObjectTempDir,
    keep_symbols: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let merged_name = "merged.o";
    let mut merged_path = objects.dir.path().to_owned();
    merged_path.push(merged_name);

    create_merged_object(&merged_path, objects.objects)?;
    filter_symbols(&merged_path, keep_symbols)?;

    let obj = File::open(&merged_path)?;
    let header = Header::new(
        merged_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .as_bytes()
            .to_vec(),
        obj.metadata()?.len(),
    );
    output.append(&header, &obj)?;

    Ok(())
}
