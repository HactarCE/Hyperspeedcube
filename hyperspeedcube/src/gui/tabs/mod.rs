use parking_lot::Mutex;
use std::sync::Arc;

mod debug;
mod puzzle_setup;
mod puzzle_view;

use super::App;
pub use debug::PolytopeTree;
pub use puzzle_setup::PuzzleSetup;
pub use puzzle_view::{PuzzleView, RenderEngine};

#[derive(Debug)]
pub enum Tab {
    PuzzleView(Arc<Mutex<PuzzleView>>),
    PuzzleSetup(PuzzleSetup),
    ViewSettings,
    PolytopeTree(PolytopeTree),
}
impl Tab {
    pub fn title(&self) -> egui::WidgetText {
        match self {
            Tab::PuzzleView(_) => "Unknown Puzzle".into(),
            Tab::PuzzleSetup(_) => "Puzzle Setup".into(),
            Tab::ViewSettings => "View Settings".into(),
            Tab::PolytopeTree(_) => "Polytope Tree".into(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        match self {
            Tab::PuzzleView(puzzle_view) => {
                if puzzle_view.lock().ui(ui) {
                    app.active_puzzle_view = Arc::downgrade(puzzle_view);
                }
            }
            Tab::PuzzleSetup(puzzle_setup) => puzzle_setup.ui(ui, app),
            Tab::ViewSettings => {
                if let Some(puzzle_view) = app.active_puzzle_view.upgrade() {
                    let mut puzzle_view_mutex_guard = puzzle_view.lock();

                    ui.horizontal(|ui| {
                        let options = [RenderEngine::SinglePass, RenderEngine::MultiPass];
                        let mut i = match puzzle_view_mutex_guard.render_engine {
                            RenderEngine::SinglePass => 0,
                            RenderEngine::MultiPass => 1,
                        };
                        let get_fn = |i: usize| options[i].to_string();
                        egui::ComboBox::new(unique_id!(), "Render engine")
                            .show_index(ui, &mut i, 2, get_fn);
                        puzzle_view_mutex_guard.render_engine = options[i];
                    });

                    ui.separator();

                    let view_params = &mut puzzle_view_mutex_guard.view_params;

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut view_params.facet_shrink)
                                .clamp_range(0.0..=0.9)
                                .speed(0.005)
                                .fixed_decimals(2),
                        );
                        ui.label("Facet shrink");
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut view_params.sticker_shrink)
                                .clamp_range(0.0..=0.9)
                                .speed(0.005)
                                .fixed_decimals(2),
                        );
                        ui.label("Sticker shrink");
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut view_params.piece_explode)
                                .clamp_range(0.0..=1.0)
                                .speed(0.005)
                                .fixed_decimals(2),
                        );
                        ui.label("Piece explode");
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut view_params.fov_3d)
                                .clamp_range(-120.0..=120.0)
                                .speed(0.5)
                                .fixed_decimals(0)
                                .suffix("°"),
                        );
                        ui.label("3D FOV");
                    });

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut view_params.fov_4d)
                                .clamp_range(-1.0..=120.0)
                                .speed(0.5)
                                .fixed_decimals(0)
                                .suffix("°"),
                        );
                        ui.label("4D FOV");
                    });
                }
            }
            Tab::PolytopeTree(polytope_tree) => polytope_tree.ui(ui, app),
        }
    }
}
