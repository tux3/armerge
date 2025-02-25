use armerge::{ArMerger, ArmergeKeepOrRemove};
use clap::Parser;
use regex::Regex;
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};
use tracing::{error, Level};
use tracing_subscriber::{filter::Directive, fmt::time::UtcTime};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Opt {
    /// Accepts regexes of the symbol names to keep global, and localizes the rest
    #[arg(short, long, num_args = 1)]
    keep_symbols: Vec<String>,

    /// Accepts regexes of the symbol names to hide, and keep the rest global
    #[arg(short, long, num_args = 1)]
    remove_symbols: Vec<String>,

    /// Order file to control the sorting of merged objects
    #[arg(long)]
    order_file: Option<PathBuf>,

    /// Output static library
    #[arg(short, long)]
    output: PathBuf,

    /// Print verbose information
    #[arg(short, long)]
    verbose: bool,

    /// Static libraries to merge
    inputs: Vec<PathBuf>,
}

fn main() {
    let opt = Opt::parse();
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(Directive::from(if opt.verbose { Level::INFO } else { Level::WARN }))
        .from_env_lossy();

    let time_format = time::format_description::parse("[hour]:[minute]:[second]").unwrap();
    tracing_subscriber::fmt()
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
    let object_order = if let Some(path) = &opt.order_file {
        parse_order_file(path)
    } else {
        Vec::new()
    };

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
            merger.merge_and_localize_ordered(ArmergeKeepOrRemove::KeepSymbols, keep_symbols, object_order)?;
        },
        (true, false) => {
            let remove_symbols: Vec<Regex> = opt
                .remove_symbols
                .into_iter()
                .map(|s| Regex::new(&s))
                .collect::<Result<Vec<_>, _>>()?;
            merger.merge_and_localize_ordered(ArmergeKeepOrRemove::RemoveSymbols, remove_symbols, object_order)?;
        },
        (false, false) => {
            return Err(
                "Can't have both keep-symbols and remove-symbols options at the same time"
                    .to_string()
                    .into(),
            );
        },
    }

    Ok(())
}

fn parse_order_file(path: &Path) -> Vec<String> {
    BufReader::new(File::open(path).unwrap())
        .lines()
        .map(|line| line.unwrap().trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}
