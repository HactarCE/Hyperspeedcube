use egui_dock::NodeIndex;

macro_rules! unique_id {
    ($($args:tt)*) => {
        egui::Id::new((file!(), line!(), column!(), $($args)*))
    };
}

mod menu_bar;
mod tabs;

pub use crate::app::App;

pub struct AppUi {
    dock_tree: egui_dock::Tree<Tab>,
}

impl AppUi {
    pub fn new() -> Self {
        let mut dock_tree = egui_dock::Tree::new(vec![
            Tab::Puzzle("3x3x3x3".to_string()),
            Tab::Puzzle("3x3x3".to_string()),
            Tab::Puzzle("Curvy Copter".to_string()),
        ]);
        dock_tree.split_right(NodeIndex::root(), 0.70, vec![Tab::PuzzleList, Tab::Colors]);
        AppUi { dock_tree }
    }

    pub fn build(&mut self, ctx: &egui::Context, app: &mut App) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, app));

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label("todo");
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(app.prefs.colors.background))
            .show(ctx, |ui| {
                egui_dock::DockArea::new(&mut self.dock_tree)
                    .style(
                        egui_dock::StyleBuilder::from_egui(&ui.style())
                            .expand_tabs(true)
                            .build(),
                    )
                    .show(ctx, app);
            });

        // key_combo_popup::build(ctx, app);
    }
}

impl egui_dock::TabViewer for App {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.ui(ui, self)
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title()
    }

    fn on_close(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub enum Tab {
    Puzzle(String),
    PuzzleList,
    View,
    Colors,
}
impl Tab {
    fn title(&self) -> egui::WidgetText {
        match self {
            Tab::Puzzle(puzzle_name) => puzzle_name.into(),
            Tab::PuzzleList => "Puzzles".into(),
            Tab::View => "View".into(),
            Tab::Colors => "Colors".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        match self {
            Tab::Puzzle(puzzle_name) => {
                ui.centered_and_justified(|ui| {
                    ui.label(format!("Puzzle view for {puzzle_name}"));
                });
            }
            Tab::PuzzleList => {
                let layout = egui::Layout::top_down_justified(egui::Align::LEFT);
                ui.with_layout(layout, |ui| {
                    ui.collapsing("3D cubic", |ui| {
                        ui.with_layout(layout, |ui| {
                            for i in 1..=9 {
                                ui.button(format!("{i}x{i}x{i}"));
                            }
                        });
                    });
                    ui.collapsing("4D hypercubic", |ui| {
                        ui.with_layout(layout, |ui| {
                            for i in 1..=9 {
                                ui.button(format!("{i}x{i}x{i}x{i}"));
                            }
                        });
                    });
                    ui.button(format!("Megaminx"));
                    ui.button(format!("120-cell"));
                });
            }
            Tab::View => {
                ui.centered_and_justified(|ui| {
                    ui.label(format!("View settings"));
                });
            }
            Tab::Colors => {
                ui.centered_and_justified(|ui| {
                    ui.label(format!("Color settings"));
                });
            }
        }
    }
}
