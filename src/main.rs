use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use glob::{glob, Paths};

use lazy_static::lazy_static;
use regex::Regex;

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::Validator;
use rustyline::Editor;
use rustyline::{CompletionType, Config, Context, EditMode};
use rustyline_derive::{Helper, Hinter};

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

fn process_files(
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

fn find_md_file_paths(root_directory: &str) -> Paths {
    glob(format!("{root_directory}/**/*.md").as_str()).expect("[FAIL] Reading glob pattern")
}

#[derive(Helper, Hinter)]
struct TagHelper {
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    tags: Vec<String>,
}

impl Completer for TagHelper {
    type Candidate = Pair;
    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        if line.is_empty() {
            let candidates = self
                .tags
                .iter()
                .map(|tag| Pair {
                    display: tag.to_string(),
                    replacement: tag.to_string(),
                })
                .collect();
            return Ok((0, candidates));
        }
        let candidates = self
            .tags
            .iter()
            .filter(|tag| tag.starts_with(line))
            .map(|tag| Pair {
                display: tag.to_string(),
                replacement: tag.to_string(),
            })
            .collect();
        Ok((0, candidates))
    }
}

impl Highlighter for TagHelper {}

impl Validator for TagHelper {}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let root_directory = &args[1];
    let file_paths = find_md_file_paths(&root_directory);
    // TODO: use _files_with_tags for graph export
    let (_files_with_tags, tags_with_files) = process_files(file_paths);
    let mut rl = Editor::with_config(
        Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build(),
    )?;
    rl.set_helper(Some(TagHelper {
        hinter: HistoryHinter {},
        tags: tags_with_files.keys().map(|key| key.to_string()).collect(),
    }));
    loop {
        let readline = rl.readline("# ");
        match readline {
            Ok(line) => match tags_with_files.get(&line) {
                Some(files) => {
                    for file in files {
                        println!("{file}");
                    }
                }
                None => {
                    println!("Tag not found")
                }
            },
            Err(ReadlineError::Interrupted) => {
                break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("{err:?}");
            }
        }
    }
    Ok(())
}
