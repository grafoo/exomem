use clap::{ArgGroup, Parser, Subcommand};
use clipboard::{ClipboardContext, ClipboardProvider};
use exomem::{add_link, exomem_dir_path, find_md_file_paths, process_files, url_to_markdown_link};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::Validator;
use rustyline::Editor;
use rustyline::{CompletionType, Config, Context, EditMode};
use rustyline_derive::{Helper, Hinter};
use std::collections::HashSet;
use std::error::Error;

const PROMPT_OK: &str = "\u{2713}";
const PROMPT_ERR: &str = "\u{2717}";
const PROMPT_BRAIN: &str = "\u{1f9e0}";

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
        // match commands
        if line.starts_with(":") {
            let commands = vec![":add", ":format"];
            let candidates = commands
                .iter()
                .filter(|c| c.starts_with(line))
                .map(|c| Pair {
                    display: c.to_string(),
                    replacement: c.to_string(),
                })
                .collect();
            return Ok((pos - line.len(), candidates));
        }
        // match tags
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

fn format_link(link_line: &str) -> Option<String> {
    match url_to_markdown_link(&link_line) {
        Some(link) => {
            println!("{PROMPT_OK} {}", link);
            let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
            ctx.set_contents(link.to_owned()).unwrap();
            Some(link)
        }
        None => {
            println!("{PROMPT_ERR} Formating failed");
            None
        }
    }
}

fn gui() {
    std::process::Command::new("emg")
        .spawn()
        .expect("Launching GUI failed");
}

fn cli(args: Args) -> Result<(), clap::Error> {
    match &args.command {
        Some(Commands::Format { link, store }) => {
            if *store {
                if let Some(formated_link) = format_link(link) {
                    match add_link(formated_link.to_string()) {
                        Ok(file_name) => println!("{PROMPT_OK} {file_name}"),
                        Err(err) => println!("{PROMPT_ERR} {:?}", err),
                    }
                }
            } else {
                format_link(link);
            }
        }
        None => {
            return Err(clap::Error::new(
                clap::error::ErrorKind::MissingRequiredArgument,
            ));
        }
    }
    Ok(())
}

#[derive(Parser, Debug)]
#[command(version)]
#[command(group(ArgGroup::new("mode").required(false).args(["repl", "gui", "cli"])))]
struct Args {
    #[arg(short, long, default_value_t = true)]
    repl: bool,
    #[arg(short, long, default_value_t = false)]
    gui: bool,
    #[arg(default_value_t = String::from("~/exomem.d"))]
    dir: String,
    #[arg(short, long, default_value_t = false)]
    cli: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Format {
        link: String,
        #[arg(short, long, default_value_t = false)]
        store: bool,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    if args.cli {
        // Subcommand required for CLI mode
        if let Err(err) = cli(args) {
            err.print().expect("Printing error failed");
        }
        return Ok(());
    }
    if args.gui {
        gui();
        return Ok(());
    }
    let exomem_dir_path = exomem_dir_path()?;
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
        let readline = rl.readline(PROMPT_BRAIN);
        match readline {
            Ok(line) => {
                if line.starts_with(":add ") {
                    if let Some(link_line) = line.splitn(2, " ").nth(1) {
                        match add_link(link_line.to_string()) {
                            Ok(file_name) => println!("{PROMPT_OK} {file_name}"),
                            Err(err) => println!("{PROMPT_ERR} {:?}", err),
                        }
                    }
                    continue;
                }
                if line.starts_with(":format ") {
                    if let Some(link_line) = line.splitn(2, " ").nth(1) {
                        format_link(link_line);
                        continue;
                    }
                }

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
                println!("{PROMPT_ERR} {err:?}");
            }
        }
    }
    Ok(())
}
