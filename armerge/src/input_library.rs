use std::io::Read;

/// A static library (ar archive) to be merged
#[derive(Debug)]
pub struct InputLibrary<R: Read> {
    pub(crate) name: String,
    pub(crate) reader: R,
}

impl<R: Read> InputLibrary<R> {
    /// The library's name is used to get more meaningful messages in case of errors.
    /// The reader reads the binary data of the static library file.
    pub fn new<IntoString: Into<String>>(name: IntoString, reader: R) -> Self {
        Self { name: name.into(), reader }
    }
}

impl<R: Read> InputLibrary<R> {
    pub fn name(&self) -> &str {
        &self.name
    }
}
