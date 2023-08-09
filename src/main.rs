use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::exit,
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
        let nondirs: Vec<&PathBuf> = self
            .directories
            .iter()
            .filter(|d| !d.is_dir())
            // .map(PathBuf::clone)
            .collect();
        if !nondirs.is_empty() {
            for nondir in nondirs {
                eprintln!("Directory {:?} does not exist", nondir);
            }
            exit(1)
        }
        return Ok(&self.directories);
    }
}

#[derive(Debug)]
struct HtmlInfo {
    hrefs: Vec<String>,
    ids: Vec<String>,
}

impl HtmlInfo {
    pub fn parse_file(path: &Path) -> std::io::Result<HtmlInfo> {
        let contents = std::fs::read_to_string(path)?;
        Ok(Self::parse(&contents))
    }
    pub fn parse(document: &str) -> HtmlInfo {
        let document = Html::parse_document(document);
        let link_selector = Selector::parse("a[href]").unwrap();
        let hrefs = document
            .select(&link_selector)
            .filter_map(|element| element.value().attr("href"))
            .map(String::from)
            .collect();
        let id_selector = Selector::parse("*[id]").unwrap();
        let ids = document
            .select(&id_selector)
            .filter_map(|element| element.value().attr("id"))
            .map(String::from)
            .collect();
        HtmlInfo { hrefs, ids }
    }
}

#[derive(Debug)]
struct FileCache(HashMap<PathBuf, HtmlInfo>);
impl FileCache {
    pub fn build(directories: &[PathBuf]) -> std::io::Result<FileCache> {
        let mut map = HashMap::new();
        for directory in directories {
            for result in WalkDir::new(directory) {
                let entry = result?;
                let path = entry
                    .path()
                    .strip_prefix(directory)
                    .expect("can't strip the prefix");
                if path.extension() == Some(OsStr::new("html")) {
                    let info = HtmlInfo::parse_file(path)?;
                    map.insert(PathBuf::from(path), info);
                }
            }
        }
        Ok(FileCache(map))
    }
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, PathBuf, HtmlInfo> {
        let FileCache(map) = self;
        map.iter()
    }
}

fn main() -> Result<(), std::io::Error> {
    let mut args = Args::parse();
    let base_dir = args.base_dir()?;
    let files = FileCache::build(args.resolve_directories()?)?;
    println!("{:?}", files);
    for (path, info) in files.iter() {
        println!("{}", path.display());
        for href in info.hrefs.iter() {
            // let href = href.strip_prefix('/').unwrap_or(href);
            // Test if URL is actually relative
            if Url::parse(href) == Err(url::ParseError::RelativeUrlWithoutBase) {
                // TODO "/foo" should refer to base_dir/foo, but "foo" should refer to
                // base_dir/file_dir/foo.
                let xxx = path.join(href);
                println!("xxx {:?}", xxx);
                let base_url = Url::from_directory_path(&base_dir.join("")).unwrap();
                let data = base_url.join(href).expect("bad url");
                let path = PathBuf::from(data.path());
                if !(path.is_file() || path.is_dir() && path.join("index.html").is_file()) {
                    println!("FAILURE {} {:?}", path.display(), path.join("index.html"));
                }
            }
        }
    }
    Ok(())
}
