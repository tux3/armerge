use anyhow::Result;
use std::path::Path;

#[cfg(not(target_os = "macos"))]
mod common;

#[cfg(target_os = "macos")]
mod mac;

pub trait ArBuilder {
    fn append_obj<P: AsRef<Path>>(&mut self, path: P) -> Result<()>;
    fn close(self) -> Result<()>;
}

#[cfg(not(target_os = "macos"))]
pub fn platform_builder(path: &Path, verbose: bool) -> impl ArBuilder {
    common::CommonArBuilder::new(path, verbose)
}

#[cfg(target_os = "macos")]
pub fn platform_builder(path: &Path, verbose: bool) -> impl ArBuilder {
    mac::MacArBuilder::new(path, verbose)
}
