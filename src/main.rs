use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use clap::Parser;
use scraper::{Element, Html, Selector};
use url::Url;
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
    let current_dir = std::env::current_dir()?;
    let base_dir = args.base.unwrap_or(current_dir.clone()).canonicalize()?;
    if args.directories.is_empty() {
        args.directories.push(current_dir.clone());
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
                        let href = href.strip_prefix('/').unwrap_or(href);
                        // Test if URL is actually relative
                        if Url::parse(href) == Err(url::ParseError::RelativeUrlWithoutBase) {
                            // TODO "/foo" should refer to base_dir/foo, but "foo" should refer to
                            // base_dir/file_dir/foo.
                            let base_url = Url::from_directory_path(&base_dir.join("")).unwrap();
                            let data = base_url.join(href).expect("bad url");
                            let path = PathBuf::from(data.path());
                            if !(path.is_file()
                                || path.is_dir() && path.join("index.html").is_file())
                            {
                                println!(
                                    "FAILURE {} {:?}",
                                    path.display(),
                                    path.join("index.html")
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
