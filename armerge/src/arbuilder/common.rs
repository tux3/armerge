use crate::{arbuilder::ArBuilder, archives, MergeError};
use ar::Builder;
use std::{
    fmt::{Debug, Formatter},
    fs::File,
    path::{Path, PathBuf},
};

pub struct CommonArBuilder {
    builder: Builder<File>,
    output_path: PathBuf,
    closed: bool,
}

impl Debug for CommonArBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommonArBuilder")
            .field("output_path", &self.output_path)
            .field("closed", &self.closed)
            .finish()
    }
}

impl ArBuilder for CommonArBuilder {
    fn append_obj(&mut self, path: &Path) -> Result<(), MergeError> {
        self.builder.append_path(path).map_err(MergeError::WritingArchive)?;
        Ok(())
    }

    fn close(mut self: Box<Self>) -> Result<(), MergeError> {
        self.finalize_index()
    }
}

impl CommonArBuilder {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self {
            builder: Builder::new(File::create(&path).expect("Failed to create output library")),
            output_path: path,
            closed: false,
        }
    }

    fn finalize_index(&mut self) -> Result<(), MergeError> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;

        archives::create_index(&self.output_path)
    }
}
