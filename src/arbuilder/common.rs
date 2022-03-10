use crate::arbuilder::ArBuilder;
use crate::archives;
use anyhow::Result;
use ar::Builder;
use std::fs::File;
use std::path::{Path, PathBuf};

pub struct CommonArBuilder {
    builder: Builder<File>,
    output_path: PathBuf,
    closed: bool,
    verbose: bool,
}

impl ArBuilder for CommonArBuilder {
    fn append_obj<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.builder.append_path(path)?;
        Ok(())
    }

    fn close(mut self) -> Result<()> {
        self.finalize_index()
    }
}

impl CommonArBuilder {
    pub fn new(path: &Path, verbose: bool) -> Self {
        Self {
            builder: Builder::new(File::create(path).expect("Failed to create output library")),
            output_path: path.to_owned(),
            closed: false,
            verbose,
        }
    }

    fn finalize_index(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;

        archives::create_index(&self.output_path, self.verbose)
    }
}
