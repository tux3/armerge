mod arbuilder;
mod archives;
mod input_library;
mod merge_error;
mod objects;
mod process_input_error;

pub use crate::input_library::InputLibrary;
use crate::{
    arbuilder::{common::CommonArBuilder, mac::MacArBuilder, ArBuilder},
    archives::{ArchiveContents, ExtractedArchive},
    merge_error::MergeError,
    process_input_error::ProcessInputError,
};
use rayon::prelude::*;
use regex::Regex;
use std::{fs::File, io::Read, path::Path};
use tracing::error;

#[derive(Debug)]
pub struct ArMerger {
    extracted: ExtractedArchive,
    builder: Box<dyn ArBuilder>,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum ArmergeKeepOrRemove {
    KeepSymbols,
    RemoveSymbols,
}

impl ArMerger {
    /// Open and extract the contents of the input static libraries
    pub fn new<I: IntoParallelIterator<Item = InputLibrary<R>>, R: Read, O: AsRef<Path>>(
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
                    Err(e) => Err(ProcessInputError::FileOpen { path: path.to_owned(), inner: e }),
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
                error!(
                    "Input archives contain neither ELF nor Mach-O files, trying to continue with your host toolchain"
                );
                arbuilder::host_platform_builder(output.as_ref())
            },
            ArchiveContents::Mixed => {
                error!(
                    "Input archives contain different object file formats, trying to continue with your host toolchain"
                );
                arbuilder::host_platform_builder(output.as_ref())
            },
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
        keep_or_remove: ArmergeKeepOrRemove,
        symbols_regexes: Iter,
    ) -> Result<(), MergeError> {
        self.merge_and_localize_ordered(keep_or_remove, symbols_regexes, std::iter::empty())
    }

    /// Merge input libraries in a specified order and localize non-public symbols
    /// `keep_symbols_regexes` contains the regex name pattern for public symbols to keep exported
    /// `object_order` contains the order in which certain object files will be merged
    pub fn merge_and_localize_ordered<Iter: IntoIterator<Item = Regex>>(
        self,
        keep_or_remove: ArmergeKeepOrRemove,
        symbols_regexes: Iter,
        object_order: impl IntoIterator<Item = String>,
    ) -> Result<(), MergeError> {
        objects::merge(
            self.builder,
            self.extracted.contents_type,
            self.extracted.object_dir,
            keep_or_remove,
            symbols_regexes.into_iter().collect(),
            object_order.into_iter().enumerate().map(|(i, s)| (s, i)).collect(),
        )
    }
}
