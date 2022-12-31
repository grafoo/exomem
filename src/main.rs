use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use glob::{glob, Paths};

use lazy_static::lazy_static;
use regex::Regex;

use eframe::egui;
use egui::plot::{Plot, Points};
use egui::widgets::plot::Legend;
use egui::widgets::plot::MarkerShape::Circle;

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::Validator;
use rustyline::Editor;
use rustyline::{CompletionType, Config, Context, EditMode};
use rustyline_derive::{Helper, Hinter};

use clap::{ArgGroup, Parser};

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

fn find_md_file_paths(exomem_dir: &Path) -> Paths {
    let dir = exomem_dir.to_str().unwrap();
    glob(format!("{dir}/**/*.md").as_str()).expect("[FAIL] Reading glob pattern")
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
        pos: usize,
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
        let query: Vec<&str> = line.split("+").collect::<Vec<&str>>();
        if query.len() > 1 {
            let search = query.last().unwrap();
            let candidates = self
                .tags
                .iter()
                .filter(|tag| tag.starts_with(search))
                .map(|tag| Pair {
                    display: tag.to_string(),
                    replacement: tag.to_string(),
                })
                .collect();
            return Ok((pos - search.len(), candidates));
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

#[derive(Default)]
struct App {}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // TODO: implement files/tag graph
            let tag_points = vec![([1.0, 1.0], "foo"), ([2.0, 2.5], "bar")];
            let named_tag_points: Vec<Points> = tag_points
                .iter()
                .map(|p| Points::new(vec![p.0]).shape(Circle).radius(5.0).name(p.1))
                .collect();
            Plot::new("Files with tags")
                .view_aspect(1.0)
                .show_axes([false; 2])
                .legend(Legend::default())
                .show(ui, |plot_ui| {
                    for points in named_tag_points {
                        plot_ui.points(points);
                    }
                });
        });
    }
}

fn gui() {
    eframe::run_native(
        "exomem",
        eframe::NativeOptions::default(),
        Box::new(|cc| Box::new(App::new(cc))),
    );
}

#[derive(Parser, Debug)]
#[command(version)]
#[command(group(ArgGroup::new("mode").required(false).args(["repl", "gui"])))]
struct Args {
    #[arg(short, long, default_value_t = true)]
    repl: bool,
    #[arg(short, long, default_value_t = false)]
    gui: bool,
    #[arg(default_value_t = String::from("~/exomem.d"))]
    dir: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let mut exomem_dir_path = PathBuf::from("/");
    if args.dir.starts_with("~/") {
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
                return Err(err_msg);
            }
        }
    }
    if args.gui {
        gui();
        return Ok(());
    }
    let file_paths = find_md_file_paths(exomem_dir_path.as_path());
    // TODO: use _files_with_tags for graph export
    let (_files_with_tags, tags_with_files) = process_files(file_paths);
    let mut rl = Editor::with_config(
        Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build(),
    )?;
    let mut helper_tags = tags_with_files
        .keys()
        .map(|key| key.to_string())
        .collect::<Vec<String>>();
    helper_tags.sort_unstable();
    rl.set_helper(Some(TagHelper {
        hinter: HistoryHinter {},
        tags: helper_tags,
    }));
    loop {
        let readline = rl.readline("# ");
        match readline {
            Ok(line) => {
                let query: Vec<&str> = line.split("+").collect();
                if query.len() > 1 {
                    let tag0 = query.get(0).unwrap();
                    let mut result_files: HashSet<String> = tags_with_files
                        .get(*tag0)
                        .unwrap()
                        .iter()
                        .map(|file| file.to_string())
                        .collect();
                    for tag in &query[1..] {
                        let other_files: HashSet<String> = tags_with_files
                            .get(*tag)
                            .unwrap()
                            .iter()
                            .map(|file| file.to_string())
                            .collect();
                        result_files = result_files
                            .intersection(&other_files)
                            .map(|file| file.to_string())
                            .collect();
                    }
                    for file in result_files {
                        println!("{file}");
                    }
                    continue;
                }
                match tags_with_files.get(&line) {
                    Some(files) => {
                        for file in files {
                            println!("{file}");
                        }
                    }
                    None => {
                        println!("Tag not found")
                    }
                }
            }
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
