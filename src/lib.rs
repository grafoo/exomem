use glob::{glob, Paths};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
pub fn find_md_file_paths(exomem_dir: &Path) -> Paths {
    let dir = exomem_dir.to_str().unwrap();
    glob(format!("{dir}/**/*.md").as_str()).expect("[FAIL] Reading glob pattern")
}
// TODO: do this non line based
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

pub fn home_path() -> Result<PathBuf, Box<dyn Error>> {
    let home = env::var("HOME")?;
    Ok(PathBuf::from(home))
}

pub fn exomem_dir_path() -> Result<PathBuf, Box<dyn Error>> {
    let mut exomem_dir_path = PathBuf::from("/");
    let home_path = home_path()?;
    exomem_dir_path.push(home_path);
    exomem_dir_path.push("exomem.d");
    Ok(exomem_dir_path)
}

// TODO: implement
pub fn add_link(text: String) -> Result<&'static str, &'static str> {
    lazy_static! {
        static ref RE_MD_LINK: Regex = Regex::new(r"\[\w+]\(\w+\)").unwrap();
    }
    if !RE_MD_LINK.is_match(&text) {
        return Err("Unsupported link format");
    }
    let exomem_dir_path = exomem_dir_path();
    let file_path = Path::new("");
    Ok("")
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
}
