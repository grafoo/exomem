// TODO: https://github.com/emilk/egui/discussions/1778

use std::collections::HashMap;
use itertools::Itertools;

use eframe::egui;
use egui_plot::{Legend, MarkerShape, Plot, Points};

use fdg_sim::{ForceGraph, ForceGraphHelper, Simulation, SimulationParameters};

use exomem::{exomem_dir_path, find_md_file_paths, process_files, ExomemError};

fn main() {
    let _ = eframe::run_native(
        "exomem",
        eframe::NativeOptions::default(),
        Box::new(|cc| match App::new(cc) {
            Ok(app) => Box::new(app),
            Err(err) => panic!("{}", err),
        }),
    );
}

struct App {
    v: Vec<[f64; 2]>,
}

impl Default for App {
    fn default() -> App {
        App { v: Vec::new() }
    }
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Result<Self, ExomemError> {
        let exomem_dir_path = exomem_dir_path()?;
        let file_paths = find_md_file_paths(exomem_dir_path.as_path());
        let (files_with_tags, tags_with_files) = process_files(file_paths);
        let mut file_nodes: HashMap<&String, _> = HashMap::new();
        // i am a petgraph::graph::NodeIndex-^
        let mut tag_nodes: HashMap<&String, _> = HashMap::new();
        let mut graph: ForceGraph<(), ()> = ForceGraph::default();
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
        let mut simulation = Simulation::from_graph(graph, SimulationParameters::default());
        for _ in 0..50 {
            simulation.update(0.035);
        }
        let mut app = Self::default();
        app.v = simulation
            .get_graph()
            .node_weights()
            .map(|node| [node.location.x as f64, node.location.y as f64])
            .collect::<Vec<[f64; 2]>>();
        Ok(app)
    }

    fn nodes(&self) -> Vec<Points> {
        self.v
            .iter()
            .map(|v| {
                Points::new(*v)
                    .name("nodes")
                    .filled(false)
                    .radius(2.7)
                    .shape(MarkerShape::Circle)
            })
            .collect()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let plot = Plot::new("nodes")
                .data_aspect(1.0)
                .legend(Legend::default());
            plot.show(ui, |plot_ui| {
                for node in self.nodes() {
                    plot_ui.points(node);
                }
            });
        });
    }
}
