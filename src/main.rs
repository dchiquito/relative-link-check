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
    pub fn parse_path_fragment(path: &Path) -> (PathBuf, Option<&str>) {
        // println!(
        //     "{:?}",
        //     path.components().collect::<Vec<std::path::Component>>()
        // );
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
    pub fn contains(&self, path: &Path) -> bool {
        let (path, fragment_option) = Self::parse_path_fragment(path);
        // println!(
        //     "{path:?} {fragment_option:?} {:?}",
        //     self.0.keys().map(PathBuf::clone).collect::<Vec<PathBuf>>()
        // );
        let path_with_index = path.join("index.html");
        if let Some(info) = self.0.get(&path).or_else(|| self.0.get(&path_with_index)) {
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
    // println!("Does it exxists? {:?}", base_dir.join(path));
    base_dir.join(path).is_file()
}

fn main() -> Result<(), std::io::Error> {
    let mut args = Args::parse();
    let base_dir = args.base_dir()?;
    let files = FileCache::build(args.resolve_directories()?)?;
    // println!("{:?}", files);
    for (path, info) in files.iter() {
        // println!("{}", path.display());
        for href in info.hrefs.iter() {
            // Test if URL is actually relative
            if Url::parse(href) == Err(url::ParseError::RelativeUrlWithoutBase) {
                let xxx = &path.parent().expect("No parent").join(href);
                let xxx = normalize_path(xxx);
                // println!("Resolving {href} to {xxx:?} from file {path:?}");
                if files.contains(&xxx) || file_exists(&base_dir, &xxx) {
                    // println!("Passed {xxx:?}");
                } else {
                    println!("Failed {xxx:?} in {path:?} in {base_dir:?}");
                }
                // println!()
            }
        }
    }
    Ok(())
}
