mod archives;
mod objects;

use ar::Builder;
use std::error::Error;
use std::fs::File;
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

    /// Static libraries to merge
    #[structopt(name = "INPUTS", parse(from_os_str))]
    inputs: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    let builder = Builder::new(File::create(opt.output)?);
    if opt.keep_symbols.is_empty() {
        archives::merge(builder, &opt.inputs)?;
    } else {
        let objects_dir = archives::extract_objects(&opt.inputs)?;
        objects::merge(builder, objects_dir, opt.keep_symbols)?;
    }

    Ok(())
}
