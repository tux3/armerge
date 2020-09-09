use crate::arbuilder::ArBuilder;
use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct MacArBuilder {
    output_path: PathBuf,
    obj_paths: Vec<PathBuf>,
    verbose: bool,
    closed: bool,
}

impl ArBuilder for MacArBuilder {
    fn append_obj<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>> {
        self.obj_paths.push(path.as_ref().to_owned());
        Ok(())
    }

    fn close(mut self) -> Result<(), Box<dyn Error>> {
        self.write_obj()
    }
}

impl MacArBuilder {
    pub fn new(path: &Path, verbose: bool) -> Self {
        Self {
            output_path: path.to_owned(),
            obj_paths: vec![],
            verbose,
            closed: false,
        }
    }

    fn write_obj(&mut self) -> Result<(), Box<dyn Error>> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;

        let mut args = [
            OsString::from("-static"),
            OsString::from("-o"),
            self.output_path.as_os_str().to_owned(),
        ]
        .to_vec();
        let mut count = 0;
        args.extend(
            self.obj_paths
                .iter()
                .inspect(|_| count += 1)
                .map(|p| p.as_os_str().into()),
        );
        if self.verbose {
            println!(
                "Merging {} objects: libtool {}",
                count,
                args.iter()
                    .map(|s| s.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }

        let status = Command::new("libtool").args(args).status();
        if let Ok(status) = status {
            if status.success() {
                return Ok(());
            }
        }

        panic!("Failed to merged object files with `libtool`")
    }
}

impl Drop for MacArBuilder {
    fn drop(&mut self) {
        self.write_obj().unwrap();
    }
}
