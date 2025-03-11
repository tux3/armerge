use std::{io, path::PathBuf};
use thiserror::Error;

/// Errors that happen while processing inputs static libraries and extracting objects from them
#[derive(Debug, Error)]
pub enum ProcessInputError {
    #[error("failed to open input file {path}: {inner})")]
    FileOpen { path: PathBuf, inner: io::Error },
    #[error("failed to create temp dir to extract objects: {0})")]
    TempDir(io::Error),
    #[error("error reading input library {name}: {inner})")]
    ReadingArchive { name: String, inner: io::Error },
    #[error("input archives don't seem to contain any objects")]
    Empty,
    #[error("error writing extracted object file {path}: {inner})")]
    ExtractingObject { path: PathBuf, inner: io::Error },
}
