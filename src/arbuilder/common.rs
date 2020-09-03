use crate::arbuilder::ArBuilder;
use crate::archives;
use ar::Builder;
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};

pub struct CommonArBuilder {
    builder: Builder<File>,
    output_path: PathBuf,
    closed: bool,
}

impl ArBuilder for CommonArBuilder {
    fn append_obj<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>> {
        self.builder.append_path(path)?;
        Ok(())
    }

    fn close(mut self) -> Result<(), Box<dyn Error>> {
        self.finalize_index()
    }
}

impl CommonArBuilder {
    pub fn new(path: &Path, _verbose: bool) -> Self {
        Self {
            builder: Builder::new(File::create(path).expect("Failed to create output library")),
            output_path: path.to_owned(),
            closed: false,
        }
    }

    fn finalize_index(&mut self) -> Result<(), Box<dyn Error>> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;

        archives::create_index(&self.output_path)
    }
}

impl Drop for CommonArBuilder {
    fn drop(&mut self) {
        self.finalize_index().unwrap();
    }
}
