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

pub fn exomem_dir_path(dir: String) -> PathBuf {
    let mut exomem_dir_path = PathBuf::from("/");
    if dir.starts_with("~/") {
        match env::var("HOME") {
            Ok(env_var_home) => {
                exomem_dir_path.push(env_var_home);
                exomem_dir_path.push("exomem.d");
            }
            Err(_err) => {
                let err_msg: Box<dyn Error> = String::from(
                    "Reading env var HOME failed; Provide DIR argument with full path.",
                )
                .into();
                // return Err(err_msg);
            }
        }
    } else {
    }

    exomem_dir_path
}
