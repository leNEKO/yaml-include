use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(clap::Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    file_path: PathBuf,
}

mod flattener;
mod helpers;

fn main() -> Result<()> {
    let args = Args::parse();

    let f = flattener::Flattener::new(args.file_path, vec![]);

    print!("{}", f);

    Ok(())
}
