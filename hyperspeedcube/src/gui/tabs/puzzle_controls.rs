use hyperpuzzle::LayerMask;

use crate::app::App;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    super::ui_with_active_puzzle_view(ui, app, |ui, _app, view| {
        for (twist, info) in &view.puzzle().twists {
            if ui.button(&info.name).clicked() {
                view.controller().lock().do_twist(twist, LayerMask(1));
            }
        }
    });
}
