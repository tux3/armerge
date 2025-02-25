use crate::MergeError;
use std::{fmt::Debug, path::Path};

pub mod common;
pub mod mac;

pub trait ArBuilder: Debug {
    fn append_obj(&mut self, path: &Path) -> Result<(), MergeError>;
    fn close(self: Box<Self>) -> Result<(), MergeError>;
}

pub fn host_platform_builder(path: &Path) -> Box<dyn ArBuilder> {
    if std::env::consts::OS == "macos" {
        Box::new(mac::MacArBuilder::new(path))
    } else {
        Box::new(common::CommonArBuilder::new(path))
    }
}
