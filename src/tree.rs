use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use walkdir::WalkDir;

use crate::html::{HtmlInfo, RelativeLink};

#[derive(Debug)]
pub struct FileCache(HashMap<PathBuf, HtmlInfo>);
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
    pub fn contains(&self, RelativeLink { path, fragment }: &RelativeLink) -> bool {
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
    pub fn uncached_file_links(&self) -> Vec<RelativeLink> {
        self.0
            .iter()
            .flat_map(|(file_path, info)| {
                info.relative_hrefs
                    .iter()
                    .map(|href| file_path.parent().expect("No parent").join(href))
                    .map(|href| normalize_path(&href))
                    .map(|href| RelativeLink::new(&href))
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
