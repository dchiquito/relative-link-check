use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Component, Path, PathBuf},
    process::exit,
};

use clap::Parser;
use regex::Regex;
use scraper::{Html, Selector};
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

#[derive(Debug)]
struct HtmlInfo {
    relative_hrefs: Vec<String>,
    external_hrefs: Vec<String>,
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
        let (relative_hrefs, external_hrefs) = document
            .select(&link_selector)
            .filter_map(|element| element.value().attr("href"))
            .map(String::from)
            .partition(|href| Url::parse(href) == Err(url::ParseError::RelativeUrlWithoutBase));

        let id_selector = Selector::parse("*[id]").unwrap();
        let ids = document
            .select(&id_selector)
            .filter_map(|element| element.value().attr("id"))
            .map(String::from)
            .collect();
        HtmlInfo {
            relative_hrefs,
            external_hrefs,
            ids,
        }
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
                    let info = HtmlInfo::parse_file(entry.path())?;
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
    pub fn contains(&self, path: &Path, fragment_option: &Option<&str>) -> bool {
        let path_with_index = path.join("index.html");
        if let Some(info) = self.0.get(path).or_else(|| self.0.get(&path_with_index)) {
            if let Some(fragment) = fragment_option {
                info.ids.contains(&fragment.to_string())
            } else {
                true
            }
        } else {
            false
        }
    }
}

pub fn parse_path_fragment(path: &Path) -> (PathBuf, Option<&str>) {
    let path = path.to_str().expect("Invalid path");
    let pattern = Regex::new("^(.*?)(?:#([^#]*))?$").unwrap();
    if let Some(captures) = pattern.captures(path) {
        let path = PathBuf::from(captures.get(1).unwrap().as_str());
        let fragment = captures.get(2).map(|m| m.as_str());
        let fragment = fragment.filter(|s| !s.is_empty());
        return (path, fragment);
    }
    panic!("Failed to parse path {path:?}")
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret.strip_prefix("/").map(Path::to_path_buf).unwrap_or(ret)
}

pub fn file_exists(base_dir: &Path, path: &Path) -> bool {
    base_dir.join(path).is_file()
}

fn main() -> Result<(), std::io::Error> {
    let mut args = Args::parse();
    let base_dir = args.base_dir()?;
    let files = FileCache::build(args.resolve_directories()?)?;
    for (path, info) in files.iter() {
        for href in info.relative_hrefs.iter() {
            let href_path = &path.parent().expect("No parent").join(href);
            let href_path = normalize_path(href_path);
            let (href_path, fragment) = parse_path_fragment(&href_path);
            if files.contains(&href_path, &fragment) || file_exists(&base_dir, &href_path) {
                // println!("Passed {xxx:?}");
            } else {
                println!("Failed {href_path:?} in {path:?} in {base_dir:?}");
            }
        }
        // println!("Skipping {} external links", info.external_hrefs.len());
    }
    Ok(())
}
