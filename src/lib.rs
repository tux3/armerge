mod arbuilder;
mod archives;
mod input_library;
mod merge_error;
mod objects;
mod process_input_error;

use crate::arbuilder::common::CommonArBuilder;
use crate::arbuilder::mac::MacArBuilder;
use crate::arbuilder::ArBuilder;
use crate::archives::{ArchiveContents, ExtractedArchive};
pub use crate::input_library::InputLibrary;
use crate::merge_error::MergeError;
use crate::process_input_error::ProcessInputError;
use regex::Regex;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tracing::error;

#[derive(Debug)]
pub struct ArMerger {
    extracted: ExtractedArchive,
    builder: Box<dyn ArBuilder>,
}

impl ArMerger {
    /// Open and extract the contents of the input static libraries
    pub fn new<I: IntoIterator<Item = InputLibrary<R>>, R: Read, O: AsRef<Path>>(
        input_libs: I,
        output: O,
    ) -> Result<Self, ProcessInputError> {
        let extracted = archives::extract_objects(input_libs)?;
        let builder = Self::create_ar_builder(extracted.contents_type, output)?;
        Ok(Self { extracted, builder })
    }

    /// Open and extract the contents of the input static libraries at the given paths
    pub fn new_from_paths<I: IntoIterator<Item = P>, P: AsRef<Path>, O: AsRef<Path>>(
        input_paths: I,
        output_path: O,
    ) -> Result<Self, ProcessInputError> {
        let libs: Result<Vec<InputLibrary<File>>, _> = input_paths
            .into_iter()
            .map(|p| {
                let path = p.as_ref();
                let filename = path
                    .file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy()
                    .replace('/', "_");
                match File::open(path) {
                    Ok(f) => Ok(InputLibrary::new(filename, f)),
                    Err(e) => Err(ProcessInputError::FileOpen {
                        path: path.to_owned(),
                        inner: e,
                    }),
                }
            })
            .collect();
        Self::new(libs?, output_path)
    }

    fn create_ar_builder<P: AsRef<Path>>(
        contents_type: ArchiveContents,
        output: P,
    ) -> Result<Box<dyn ArBuilder>, ProcessInputError> {
        Ok(match contents_type {
            ArchiveContents::Empty => return Err(ProcessInputError::Empty),
            ArchiveContents::Elf => Box::new(CommonArBuilder::new(output.as_ref())),
            ArchiveContents::MachO => Box::new(MacArBuilder::new(output.as_ref())),
            ArchiveContents::Other => {
                error!("Input archives contain neither ELF nor Mach-O files, trying to continue with your host toolchain");
                arbuilder::host_platform_builder(output.as_ref())
            }
            ArchiveContents::Mixed => {
                error!("Input archives contain different object file formats, trying to continue with your host toolchain");
                arbuilder::host_platform_builder(output.as_ref())
            }
        })
    }

    /// The type of object files detected in all the input archives
    pub fn archive_contents(&self) -> ArchiveContents {
        self.extracted.contents_type
    }

    /// Merge without localizing any symbols, this just re-packs extracted object files into an archive
    pub fn merge_simple(self) -> Result<(), MergeError> {
        archives::merge(self.builder, self.extracted.object_dir)
    }

    /// Merge input libraries and localize non-public symbols
    /// `keep_symbols_regexes` contains the regex name pattern for public symbols to keep exported
    pub fn merge_and_localize<Iter: IntoIterator<Item = Regex>>(
        self,
        keep_symbols_regexes: Iter,
    ) -> Result<(), MergeError> {
        objects::merge(
            self.builder,
            self.extracted.contents_type,
            self.extracted.object_dir,
            keep_symbols_regexes.into_iter().collect(),
        )
    }
}
