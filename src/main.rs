use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Output yaml with recursive "!included" data
#[derive(clap::Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// main yaml file path to process
    file_path: PathBuf,
}

mod helpers;
mod transformer;

fn main() -> Result<()> {
    let args = Args::parse();

    let f = transformer::Transformer::new(args.file_path, None)?;

    print!("{}", f);

    Ok(())
}
