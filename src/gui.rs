use exomem::{exomem_dir_path, find_md_file_paths, process_files};
use fdg_macroquad::fdg_sim::{ForceGraph, ForceGraphHelper};
use fdg_macroquad::macroquad;
use fdg_macroquad::run_window;
use itertools::Itertools;
use std::collections::HashMap;
use std::error::Error;
#[macroquad::main("exomem")]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut graph: ForceGraph<(), ()> = ForceGraph::default();
    let exomem_dir_path = exomem_dir_path()?;
    let file_paths = find_md_file_paths(exomem_dir_path.as_path());
    let (files_with_tags, tags_with_files) = process_files(file_paths);
    let mut file_nodes: HashMap<&String, _> = HashMap::new();
    // i am a petgraph::graph::NodeIndex-^
    let mut tag_nodes: HashMap<&String, _> = HashMap::new();
    for (file, tags) in files_with_tags.iter() {
        match file_nodes.get(file) {
            Some(file_node) => {
                for tag in tags.iter() {
                    let tag_node = graph.add_force_node(tag, ());
                    tag_nodes.insert(tag, tag_node);
                    graph.add_edge(*file_node, tag_node, ());
                }
            }
            None => {
                let file_node = graph.add_force_node(file, ());
                file_nodes.insert(file, file_node);
                for tag in tags.iter() {
                    match tag_nodes.get(tag) {
                        Some(tag_node) => {
                            graph.add_edge(file_node, *tag_node, ());
                        }
                        None => {
                            let tag_node = graph.add_force_node(tag, ());
                            tag_nodes.insert(tag, tag_node);
                            graph.add_edge(file_node, tag_node, ());
                        }
                    }
                }
            }
        }
    }
    // connect tag related files
    for (_, files) in tags_with_files.iter() {
        if files.len() <= 1 {
            continue;
        }
        let tag_related_files = files.iter().cartesian_product(files.iter());
        for (file_1, file_2) in tag_related_files {
            if file_1 != file_2 {
                if let Some(file_node_1) = file_nodes.get(file_1) {
                    if let Some(file_node_2) = file_nodes.get(file_2) {
                        graph.add_edge(*file_node_1, *file_node_2, ());
                    }
                }
            }
        }
    }
    run_window(&graph).await;
    Ok(())
}
