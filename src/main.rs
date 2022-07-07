use armerge::ArMerger;
use regex::Regex;
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
        return Err("No input file specified".to_string().into());
    }

    // TODO: Handle verbose option again
    let _ = opt.verbose;

    let merger = ArMerger::new_from_paths(&opt.inputs, &opt.output)?;

    if opt.keep_symbols.is_empty() {
        // If we don't need to localize any symbols, this is the easy case where we just extract
        // contents and re-pack them, no linker necessary.
        merger.merge_simple()?;
    } else {
        let keep_symbols: Vec<Regex> = opt
            .keep_symbols
            .into_iter()
            .map(|s| Regex::new(&s))
            .collect::<Result<Vec<_>, _>>()?;
        merger.merge_and_localize(keep_symbols)?;
    }

    Ok(())
}
