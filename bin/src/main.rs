use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use yaml_include::Transformer;

/// Output yaml with processed "!include" tags
#[derive(clap::Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// main yaml file path to process
    file_path: PathBuf,
    /// optional output path (output to stdout if not set)
    #[arg(short, long)]
    output_path: Option<PathBuf>,
    /// panic on circular reference
    /// (default: gracefully handle circular references with !circular tag)
    #[arg(short, long)]
    error_on_circular: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let res = run(args);
    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:?}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<()> {
    let transformer = Transformer::new(args.file_path, args.error_on_circular);
    let data = transformer?.parse_to_string()?;
    match args.output_path {
        Some(path) => {
            fs::write(path, data)?;
        }
        None => print!("{}", data),
    };
    Ok(())
}
