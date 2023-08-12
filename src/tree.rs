use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use regex::Regex;
use walkdir::WalkDir;

use crate::html::HtmlInfo;

/**
A link to an HTML file, with optional fragment.
*/
#[derive(Debug)]
pub struct HtmlFileLink {
    pub path: PathBuf,
    pub fragment: Option<String>,
}

impl HtmlFileLink {
    pub fn new(path: &Path) -> HtmlFileLink {
        let path = path.to_str().expect("Invalid path");
        let pattern = Regex::new("^(.*?)(?:#([^#]*))?$").unwrap();
        if let Some(captures) = pattern.captures(path) {
            let path = PathBuf::from(captures.get(1).unwrap().as_str());
            let fragment = captures.get(2).map(|m| m.as_str());
            let fragment = fragment.filter(|s| !s.is_empty()).map(|s| s.to_string());
            return HtmlFileLink { path, fragment };
        }
        panic!("Failed to parse path {path:?}")
    }
}

#[derive(Debug)]
pub struct HtmlFiles(HashMap<PathBuf, HtmlInfo>);
impl HtmlFiles {
    pub fn new(directories: &[PathBuf]) -> std::io::Result<HtmlFiles> {
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
        Ok(HtmlFiles(map))
    }
    pub fn contains(&self, HtmlFileLink { path, fragment }: &HtmlFileLink) -> bool {
        let path_with_index = path.join("index.html");
        if let Some(info) = self.0.get(path).or_else(|| self.0.get(&path_with_index)) {
            // If a "#fragment" id is present, also check that the document contains the fragment
            if let Some(fragment) = fragment {
                info.ids.contains(&fragment.to_string())
            } else {
                true
            }
        } else {
            false
        }
    }
    pub fn missing_file_links(&self) -> Vec<HtmlFileLink> {
        self.0
            .iter()
            .flat_map(|(file_path, info)| {
                info.relative_hrefs
                    .iter()
                    .map(|href| file_path.parent().expect("No parent").join(href))
                    .map(|href| normalize_path(&href))
                    .map(|href| HtmlFileLink::new(&href))
                    .filter(|link| !self.contains(link))
            })
            .collect()
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
