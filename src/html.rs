use std::path::Path;

use scraper::{Html, Selector};
use url::Url;

/**
The relevant contents of an HTML document.

Currently we only care about:
* The `href` attributes of any link tags, split into absolute and relative URLs
* Any `id` attributes on any tags
 */
#[derive(Debug)]
pub struct HtmlInfo {
    pub relative_hrefs: Vec<String>,
    pub external_hrefs: Vec<String>,
    pub ids: Vec<String>,
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_parse() {
        let html_info = HtmlInfo::parse(
            "
<div id=\"main\">
    <a href=\"adjacent_file.txt\">a</a>
    <a href=\"/relative/file.txt\">b</a>
    <a id=\"url\" href=\"https://www.google.com\">c</a>
    <div id=\"sub\" />
</div>",
        );
        assert_eq!(html_info.relative_hrefs, vec!["adjacent_file.txt", "/relative/file.txt"]);
        assert_eq!(html_info.external_hrefs, vec!["https://www.google.com"]);
        assert_eq!(html_info.ids, vec!["main", "url", "sub"]);
    }
}
