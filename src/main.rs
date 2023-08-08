use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use clap::Parser;
use scraper::{Element, Html, Selector};
use walkdir::WalkDir;

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
    for directory in &args.directories {
        if !directory.is_dir() {}
    }
    let nondirs: Vec<PathBuf> = args
        .directories
        .iter()
        .filter(|d| !d.is_dir())
        .map(PathBuf::clone)
        .collect();
    if !nondirs.is_empty() {
        for nondir in nondirs {
            eprintln!("Directory {:?} does not exist", nondir);
        }
        return Ok(());
    }
    for directory in &args.directories {
        for result in WalkDir::new(directory) {
            let entry = result?;
            let path = entry.path();
            if path.extension() == Some(OsStr::new("html")) {
                println!("{:?}", path);
                let contents = std::fs::read_to_string(path)?;
                let document = Html::parse_document(&contents);
                let selector = Selector::parse("a").unwrap();
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        println!("{}", href);
                    }
                }
            }
        }
    }
    Ok(())
}
