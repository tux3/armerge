use std::error::Error;
use std::path::Path;

#[cfg(not(target_os = "macos"))]
mod common;

#[cfg(target_os = "macos")]
mod mac;

pub trait ArBuilder {
    fn append_obj<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>>;
    fn close(self) -> Result<(), Box<dyn Error>>;
}

#[cfg(not(target_os = "macos"))]
pub fn platform_builder(path: &Path, verbose: bool) -> impl ArBuilder {
    common::CommonArBuilder::new(path, verbose)
}

#[cfg(target_os = "macos")]
pub fn platform_builder(path: &Path, verbose: bool) -> impl ArBuilder {
    mac::MacArBuilder::new(path, verbose)
}
