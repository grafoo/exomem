use glob::{glob, Paths};
use lazy_static::lazy_static;
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use ureq;

pub enum ExomemError {
    HomeEnvVarLookup,
    LinkTitleParse,
    FileCreate,
    FileWrite,
}

impl fmt::Display for ExomemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Debug for ExomemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} :: file: {}, line: {}", self, file!(), line!())
    }
}

pub fn find_md_file_paths(exomem_dir: &Path) -> Paths {
    let dir = exomem_dir.to_str().unwrap();
    glob(format!("{dir}/**/*.md").as_str()).expect("[FAIL] Reading glob pattern")
}

// TODO: match over multiple lines
fn find_tags(file_path: &Path) -> Option<HashSet<String>> {
    lazy_static! {
        static ref RE_TAGS: Regex = Regex::new(r"((?:#\w+\s*)+)").unwrap();
    }
    let file_reader = BufReader::new(File::open(file_path).ok()?);
    let mut tags = HashSet::new();
    for line in file_reader.lines() {
        let line = line.unwrap();
        if let Some(captures) = RE_TAGS.captures(&line) {
            (&captures[0])
                .split(" ")
                .filter(|tag| !tag.is_empty())
                .for_each(|tag| {
                    tags.insert(tag[1..].to_string());
                });
        }
    }
    if tags.is_empty() {
        None
    } else {
        Some(tags)
    }
}

pub fn process_files(
    file_path_entries: Paths,
) -> (
    HashMap<String, HashSet<String>>,
    HashMap<String, HashSet<String>>,
) {
    let mut files_with_tags: HashMap<String, HashSet<String>> = HashMap::new();
    let mut tags_with_files: HashMap<String, HashSet<String>> = HashMap::new();
    for entry in file_path_entries {
        if let Ok(path) = entry {
            if let Some(tags) = find_tags(&path) {
                let md_file_path = &path.to_str().unwrap().to_string();
                files_with_tags.insert(md_file_path.clone(), tags.clone());
                for tag in tags.iter() {
                    match tags_with_files.get_mut(tag) {
                        Some(files) => {
                            files.insert(md_file_path.clone());
                        }
                        None => {
                            let mut files = HashSet::new();
                            files.insert(md_file_path.clone());
                            tags_with_files.insert(tag.clone(), files);
                        }
                    }
                }
            }
        }
    }
    (files_with_tags, tags_with_files)
}

pub fn home_path() -> Result<PathBuf, ExomemError> {
    match env::var("HOME") {
        Ok(home) => Ok(PathBuf::from(home)),
        Err(_) => Err(ExomemError::HomeEnvVarLookup),
    }
}

pub fn exomem_dir_path() -> Result<PathBuf, ExomemError> {
    match home_path() {
        Ok(home_path) => {
            let mut exomem_dir_path = PathBuf::from("/");
            exomem_dir_path.push(home_path);
            exomem_dir_path.push("exomem.d");
            Ok(exomem_dir_path)
        }
        Err(err) => Err(err),
    }
}

#[derive(Debug, PartialEq)]
pub struct Link {
    name: String,
    href: String,
    tags: Vec<String>,
}

pub fn url_to_markdown_link(url: &str) -> Option<String> {
    lazy_static! {
        static ref RE_URL: Regex = Regex::new(r#"^https?://"#).unwrap();
    }
    if RE_URL.is_match(&url) {
        let text = ureq::get(url).call().ok()?.into_string().ok()?;
        let html = Html::parse_document(&text);
        // <meta name="keywords" content="foo, bar, num">
        let keywords_selector = Selector::parse(r#"meta[name="keywords"]"#).unwrap();
        let tags: Vec<String> = match html.select(&keywords_selector).next() {
            Some(meta) => match meta.value().attr("content") {
                Some(keywords) => keywords
                    .split(",")
                    .map(|k| k.trim().replace(" ", "_").to_string())
                    .collect(),
                None => vec![],
            },
            None => vec![],
        };
        let title_selector = Selector::parse(r#"title"#).unwrap();
        let name: String = match html.select(&title_selector).next() {
            Some(title) => title.inner_html(),
            None => RE_URL.replace_all(url, "").to_string(),
        };
        let link = Link {
            name: name.trim().to_string(),
            href: url.to_string(),
            tags,
        };
        if !link.tags.is_empty() {
            let mut tags = link.tags.join("#");
            tags.insert(0, '#');
            return Some(format!("[{}]({}){}", link.name, link.href, tags));
        }
        return Some(format!("[{}]({})", link.name, link.href));
    }
    None
}

// preformated markdown link
pub fn link_line_to_struct(link: &str) -> Option<Link> {
    lazy_static! {
        static ref RE_MD_LINK: Regex =
            Regex::new(r"^\[(?P<title>.+)\]\((?P<href>.+)\)(?P<tags>(#[\w\-_]+)*)$").unwrap();
    }
    if let Some(captures) = RE_MD_LINK.captures(&link) {
        let title = captures.name("title").map_or("", |t| t.as_str());
        let href = captures.name("href").map_or("", |h| h.as_str());
        let tags = captures.name("tags").map_or("", |t| t.as_str());
        if title.is_empty() || href.is_empty() {
            return None;
        }
        let link_file = Link {
            name: title.to_string(),
            href: href.to_string(),
            tags: tags
                .split("#")
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string())
                .collect(),
        };
        return Some(link_file);
    }
    return None;
}

pub fn format_link_file_content(link: Link) -> String {
    let mut content = String::from("");
    for tag in &link.tags {
        content.push_str(format!("#{tag} ").as_str());
    }
    content.pop();
    if link.tags.len() > 0 {
        content.push_str(format!("\n\n").as_str());
    }
    content.push_str(format!("[{}]({})\n", link.name, link.href).as_str());
    content
}

pub fn add_link(line: String) -> Result<String, ExomemError> {
    match link_line_to_struct(&line) {
        Some(link) => {
            let mut path = exomem_dir_path()?;
            lazy_static! {
                static ref RE_FILE_NAME_CHARS: Regex = Regex::new(r"[^[[:word:]]\- ]").unwrap();
            }
            lazy_static! {
                static ref RE_DASHABLE_CHARS: Regex = Regex::new(r"[—\|/]").unwrap();
            }
            let dashed_link_name = RE_DASHABLE_CHARS.replace_all(link.name.as_str(), "-");
            let file_name = RE_FILE_NAME_CHARS.replace_all(&dashed_link_name, "_");
            path.push(format!("{}.md", file_name));
            match File::create(path.clone()) {
                Ok(mut file) => {
                    let content = format_link_file_content(link);
                    match file.write_all(content.as_bytes()) {
                        Ok(_) => Ok(format!("{:?}", path)),
                        Err(_) => Err(ExomemError::FileWrite),
                    }
                }
                Err(_) => Err(ExomemError::FileCreate),
            }
        }
        None => Err(ExomemError::LinkTitleParse),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env::{set_var, var};
    use std::path::PathBuf;
    #[test]
    fn home_path_returns_pathbuf_when_home_env_var_is_set() {
        let expected = PathBuf::from("/home/foo");
        set_var("HOME", "/home/foo");
        if let Ok(_) = var("HOME") {
            if let Ok(home_path) = home_path() {
                assert_eq!(expected, home_path);
            } else {
                panic!();
            }
        }
    }
    #[test]
    fn exomem_dir_path_returns_valid_pathbuf() {
        let expected = PathBuf::from("/home/foo/exomem.d");
        set_var("HOME", "/home/foo");
        if let Ok(_) = var("HOME") {
            if let Ok(exomem_dir_path) = exomem_dir_path() {
                assert_eq!(expected, exomem_dir_path);
            } else {
                panic!();
            }
        }
    }
    #[test]
    fn link_to_struct_accepts_valid_link() {
        let expected = Link {
            name: "foo-title".to_string(),
            href: "https://foo.tld".to_string(),
            tags: vec!["foo-tag".to_string(), "bar_tag".to_string()],
        };
        let s = "[foo-title](https://foo.tld)#foo-tag#bar_tag";
        if let Some(link_file) = link_line_to_struct(s) {
            assert_eq!(expected, link_file);
        } else {
            panic!();
        }
    }
    #[test]
    fn format_link_file_content_returns_markdown_str() {
        let link = Link {
            name: "foo-title".to_string(),
            href: "https://foo.tld".to_string(),
            tags: vec!["foo-tag".to_string(), "bar_tag".to_string()],
        };
        let expected = format!("#foo-tag #bar_tag\n\n[foo-title](https://foo.tld)\n");
        if let Ok(content) = format_link_file_content(link) {
            assert_eq!(expected, content);
        } else {
            panic!();
        }
    }
}
