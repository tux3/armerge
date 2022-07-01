use crate::arbuilder::ArBuilder;
use anyhow::Result;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct MacArBuilder {
    output_path: PathBuf,
    obj_paths: Vec<PathBuf>,
    verbose: bool,
    closed: bool,
}

impl ArBuilder for MacArBuilder {
    fn append_obj(&mut self, path: &Path) -> Result<()> {
        self.obj_paths.push(path.to_owned());
        Ok(())
    }

    fn close(mut self: Box<Self>) -> Result<()> {
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

    fn write_obj(&mut self) -> Result<()> {
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

        let output = Command::new("libtool").args(args).output()?;
        if output.status.success() {
            Ok(())
        } else {
            std::io::stdout().write_all(&output.stdout).unwrap();
            std::io::stderr().write_all(&output.stderr).unwrap();
            panic!("Failed to merged object files with `libtool`")
        }
    }
}
