mod arbuilder;
mod archives;
mod objects;

use crate::arbuilder::common::CommonArBuilder;
use crate::arbuilder::mac::MacArBuilder;
use crate::arbuilder::ArBuilder;
use crate::archives::ArchiveContents;
use anyhow::{bail, Result};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "armerge")]
struct Opt {
    /// Accepts regexes of the symbol names to keep global, and localizes the rest
    #[structopt(short, long, number_of_values = 1)]
    keep_symbols: Vec<String>,

    /// Output static library
    #[structopt(short, long, parse(from_os_str))]
    output: PathBuf,

    /// Print verbose information
    #[structopt(short, long)]
    verbose: bool,

    /// Static libraries to merge
    #[structopt(name = "INPUTS", parse(from_os_str))]
    inputs: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    if opt.inputs.is_empty() {
        bail!("No input file specified");
    }

    let extracted = archives::extract_objects(&opt.inputs)?;
    let builder: Box<dyn ArBuilder> = match extracted.contents_type {
        ArchiveContents::Empty => bail!("Input archives don't seem to contain any files"),
        ArchiveContents::Elf => Box::new(CommonArBuilder::new(&opt.output, opt.verbose)),
        ArchiveContents::MachO => Box::new(MacArBuilder::new(&opt.output, opt.verbose)),
        ArchiveContents::Other => {
            eprintln!("Input archives contain neither ELF nor Mach-O files, trying to continue with your host toolchain");
            arbuilder::host_platform_builder(&opt.output, opt.verbose)
        }
        ArchiveContents::Mixed => {
            eprintln!("Input archives contain different object file formats, trying to continue with your host toolchain");
            arbuilder::host_platform_builder(&opt.output, opt.verbose)
        }
    };

    if opt.keep_symbols.is_empty() {
        // If we don't need to localize any symbols, this is the easy case where we just extract
        // contents and re-pack them, no linker necessary.
        archives::merge(builder, extracted.object_dir)?;
    } else {
        objects::merge(
            builder,
            extracted.contents_type,
            extracted.object_dir,
            opt.keep_symbols,
            opt.verbose,
        )?;
    }

    Ok(())
}
