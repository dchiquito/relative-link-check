use std::{
    path::{Path, PathBuf},
    process::exit,
};

use clap::Parser;

mod html;
mod tree;
use crate::tree::HtmlFiles;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    base: Option<PathBuf>,

    #[arg()]
    directories: Vec<PathBuf>,
}

impl Args {
    pub fn base_dir(&self) -> std::io::Result<PathBuf> {
        let current_dir = std::env::current_dir()?;
        let base_dir = self
            .base
            .clone()
            .unwrap_or(current_dir.clone())
            .canonicalize()?;
        Ok(base_dir)
    }
    pub fn resolve_directories(&mut self) -> std::io::Result<&[PathBuf]> {
        let current_dir = std::env::current_dir()?;
        if self.directories.is_empty() {
            self.directories.push(current_dir.clone());
        }
        let nondirs: Vec<&PathBuf> = self.directories.iter().filter(|d| !d.is_dir()).collect();
        if !nondirs.is_empty() {
            for nondir in nondirs {
                eprintln!("Directory {:?} does not exist", nondir);
            }
            exit(1)
        }
        Ok(&self.directories)
    }
}

pub fn file_exists(base_dir: &Path, path: &Path) -> bool {
    base_dir.join(path).is_file()
}

pub fn main() -> std::io::Result<()> {
    let mut args = Args::parse();
    let base_dir = args.base_dir()?;
    let files = HtmlFiles::new(args.resolve_directories()?)?;
    for link in files.missing_file_links() {
        if !file_exists(&base_dir, &link.path) {
            println!("Failed {link:?} in {base_dir:?}");
        }
    }
    Ok(())
}
