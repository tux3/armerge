use anyhow::Result;
use std::path::Path;

pub mod common;
pub mod mac;

pub trait ArBuilder {
    fn append_obj(&mut self, path: &Path) -> Result<()>;
    fn close(self: Box<Self>) -> Result<()>;
}

pub fn host_platform_builder(path: &Path, verbose: bool) -> Box<dyn ArBuilder> {
    if std::env::consts::OS == "macos" {
        Box::new(mac::MacArBuilder::new(path, verbose))
    } else {
        Box::new(common::CommonArBuilder::new(path, verbose))
    }
}
