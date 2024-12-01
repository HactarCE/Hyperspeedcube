use hyperpuzzle::LayerMask;

use crate::app::App;
use crate::L;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    app.active_puzzle_view.with_opt(|p| {
        let Some(p) = p else {
            ui.label(L.no_active_puzzle);
            return;
        };

        for (twist, info) in &p.puzzle().twists {
            if ui.button(&info.name).clicked() {
                p.sim().lock().do_twist(twist, LayerMask(1));
            }
        }
    });
}
