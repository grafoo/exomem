// TODO: https://github.com/emilk/egui/discussions/1778

use eframe::egui;
use egui_plot;

use egui_plot::{Legend, MarkerShape, Plot, Points};

use fdg_sim::{ForceGraph, ForceGraphHelper, Simulation, SimulationParameters};

fn main() {
    eframe::run_native(
        "exomem",
        eframe::NativeOptions::default(),
        Box::new(|c| Box::new(App::new(c))),
    );
}

struct App {
    v: Vec<[f64; 2]>,
}

impl Default for App {
    fn default() -> App {
        let mut graph: ForceGraph<(), ()> = ForceGraph::default();
        let one = graph.add_force_node("one", ());
        let two = graph.add_force_node("two", ());
        let _three = graph.add_force_node("three", ());
        graph.add_edge(one, two, ());
        let mut simulation = Simulation::from_graph(graph, SimulationParameters::default());
        for frame in 0..20 {
            simulation.update(0.035);
        }
        App {
            v: simulation
                .get_graph()
                .node_weights()
                .map(|node| [node.location.x as f64, node.location.y as f64])
                .collect::<Vec<[f64; 2]>>(),
        }
    }
}

impl App {
    fn new(c: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn nodes(&self) -> Vec<Points> {
        self.v
            .iter()
            .map(|v| {
                Points::new(*v)
                    .name("nodes")
                    .filled(false)
                    .radius(5.0)
                    .shape(MarkerShape::Circle)
            })
            .collect()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
