use armerge::{ArmergeKeepOrRemove, ArMerger};
use regex::Regex;
use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;
use tracing::{error, Level};
use tracing_subscriber::filter::Directive;
use tracing_subscriber::fmt::time::UtcTime;

#[derive(StructOpt, Debug)]
#[structopt(name = "armerge")]
struct Opt {
    /// Accepts regexes of the symbol names to keep global, and localizes the rest
    #[structopt(short, long, number_of_values = 1)]
    keep_symbols: Vec<String>,

    /// Accepts regexes of the symbol names to hide, and keep the rest global
    #[structopt(short, long, number_of_values = 1)]
    remove_symbols: Vec<String>,

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

fn main() {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "warn")
    }

    let opt = Opt::from_args();
    let mut filter = tracing_subscriber::EnvFilter::from_default_env();
    if opt.verbose {
        filter = filter.add_directive(Directive::from(Level::INFO));
    }
    let time_format = time::format_description::parse("[hour]:[minute]:[second]").unwrap();
    tracing_subscriber::fmt::fmt()
        .with_timer(UtcTime::new(time_format))
        .with_env_filter(filter)
        .init();

    if let Err(e) = err_main(opt) {
        error!("{}", e);
        std::process::exit(1);
    }
}

fn err_main(opt: Opt) -> Result<(), Box<dyn Error>> {
    if opt.inputs.is_empty() {
        return Err("No input file specified".to_string().into());
    }

    let merger = ArMerger::new_from_paths(&opt.inputs, &opt.output)?;

    match (opt.keep_symbols.is_empty(), opt.remove_symbols.is_empty()) {
        (true, true) => {
            // If we don't need to localize any symbols, this is the easy case where we just extract
            // contents and re-pack them, no linker necessary.
            merger.merge_simple()?;
        },
        (false, true) => {
            let keep_symbols: Vec<Regex> = opt
                .keep_symbols
                .into_iter()
                .map(|s| Regex::new(&s))
                .collect::<Result<Vec<_>, _>>()?;
            merger.merge_and_localize(ArmergeKeepOrRemove::KeepSymbols, keep_symbols)?;
        },
        (true, false) => {
            let remove_symbols: Vec<Regex> = opt
                .remove_symbols
                .into_iter()
                .map(|s| Regex::new(&s))
                .collect::<Result<Vec<_>, _>>()?;
            merger.merge_and_localize(ArmergeKeepOrRemove::RemoveSymbols, remove_symbols)?;
        },
        (false, false) => {
            return Err("Can't have both keep-symbols and remove-symbols options at the same time".to_string().into());
        }
    }

    Ok(())
}
