use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    base: Option<PathBuf>,

    #[arg()]
    directories: Vec<PathBuf>,
}

fn main() -> Result<(), std::io::Error> {
    let mut args = Args::parse();
    if args.base.is_none() {
        args.base = Some(std::env::current_dir()?);
    }
    if args.directories.is_empty() {
        args.directories.push(std::env::current_dir()?);
    }
    println!("Hello, world! {:?}", args.directories);
    Ok(())
}
