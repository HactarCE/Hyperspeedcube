use hyperpuzzle::{LayerMask, LayeredTwist};
use hyperpuzzle_view::ReplayEvent;
use smallvec::smallvec;

use crate::app::App;
use crate::L;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    app.active_puzzle.with_opt_view(|view| {
        let Some(view) = view else {
            ui.label(L.no_active_puzzle);
            return;
        };

        for (transform, info) in &view.puzzle().twists {
            if ui.button(&info.name).clicked() {
                let layers = LayerMask::default();
                let twist = LayeredTwist { layers, transform };
                view.sim
                    .lock()
                    .do_event(ReplayEvent::Twists(smallvec![twist]));
            }
        }
    });
}
