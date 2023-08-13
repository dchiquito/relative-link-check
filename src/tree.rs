use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use regex::Regex;
use walkdir::WalkDir;

use crate::html::HtmlInfo;

/**
A link to an HTML file, with optional fragment.
*/
#[derive(Debug, PartialEq, Eq)]
pub struct HtmlFileLink {
    pub path: PathBuf,
    pub fragment: Option<String>,
}

impl HtmlFileLink {
    pub fn new<P: AsRef<Path>>(path: P) -> HtmlFileLink {
        let path = path.as_ref().to_str().expect("Invalid path");
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
                    .map(normalize_path)
                    .map(HtmlFileLink::new)
                    .filter(|link| !self.contains(link))
            })
            .collect()
    }
}

pub fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut components = path.as_ref().components().peekable();
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_html_file_link_new() {
        macro_rules! assert_link_eq {
            ($href:expr, $path:expr ) => {
                assert_eq!(
                    HtmlFileLink::new($href),
                    HtmlFileLink {
                        path: $path.into(),
                        fragment: None,
                    }
                );
            };
            ($href:expr, $path:expr, $fragment:expr) => {
                assert_eq!(
                    HtmlFileLink::new($href),
                    HtmlFileLink {
                        path: $path.into(),
                        fragment: Some($fragment.into())
                    }
                );
            };
        }
        assert_link_eq!("foo", "foo");
        assert_link_eq!("foo/bar", "foo/bar");
        assert_link_eq!("/foo/bar", "/foo/bar");
        assert_link_eq!("foo#bar", "foo", "bar");
        assert_link_eq!("foo#bar#baz", "foo#bar", "baz");
        assert_link_eq!("foo#", "foo");
        assert_link_eq!("#foo", "", "foo");
        assert_link_eq!("#", "");
        assert_link_eq!("", "");
    }

    #[test]
    fn test_html_files_contains() {
        macro_rules! link {
            ($path:expr) => {
                HtmlFileLink {
                    path: $path.into(),
                    fragment: None,
                }
            };
            ($path:expr, $fragment:expr) => {
                HtmlFileLink {
                    path: $path.into(),
                    fragment: Some($fragment.into()),
                }
            };
        }
        macro_rules! html_files {
            ($files:expr, $key:expr => $value:expr) => {{
                $files.0.insert($key.into(), HtmlInfo::parse($value));
            }};
            ($($key:expr => $value:expr),+) => {{
                let mut files = HtmlFiles(HashMap::new());
                $(
                    html_files!(files, $key => $value);
                )*
                files
            }};
        }
        let files = html_files!(
            "foo" => "<a href=\"foo\" id=\"foo\">",
            "/bar" => "<a href=\"/bar\" id=\"bar\">",
            "/baz/index.html" => "<a href=\"/baz\" id=\"baz\">"
        );
        assert!(files.contains(&HtmlFileLink::new("foo")));
        assert!(!files.contains(&HtmlFileLink::new("foooo")));
        assert!(files.contains(&HtmlFileLink::new("foo#foo")));
        assert!(files.contains(&HtmlFileLink::new("/bar")));
        assert!(files.contains(&HtmlFileLink::new("/bar#bar")));
        assert!(!files.contains(&HtmlFileLink::new("bar")));
        assert!(files.contains(&HtmlFileLink::new("/baz")));
        assert!(files.contains(&HtmlFileLink::new("/baz#baz")));
        assert!(files.contains(&HtmlFileLink::new("/baz/")));
        assert!(files.contains(&HtmlFileLink::new("/baz/#baz")));
        assert!(files.contains(&HtmlFileLink::new("/baz/index.html#baz")));
        assert!(files.contains(&HtmlFileLink::new("/baz/index.html#baz")));
    }
}
