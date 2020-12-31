mod arbuilder;
mod archives;
mod objects;

use std::error::Error;
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

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    if opt.inputs.is_empty() {
        return Err(From::from("No input file specified"));
    }

    let builder = arbuilder::platform_builder(&opt.output, opt.verbose);
    if opt.keep_symbols.is_empty() {
        archives::merge(builder, &opt.inputs)?;
    } else {
        let objects_dir = archives::extract_objects(&opt.inputs)?;
        objects::merge(builder, objects_dir, opt.keep_symbols, opt.verbose)?;
    }

    Ok(())
}
