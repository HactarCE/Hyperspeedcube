use hyperpuzzle::{LayerMask, LayeredTwist};
use hyperpuzzle_view::ReplayEvent;
use smallvec::smallvec;

use crate::app::App;
use crate::L;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    app.active_puzzle_view.with_opt(|p| {
        let Some(p) = p else {
            ui.label(L.no_active_puzzle);
            return;
        };

        for (transform, info) in &p.puzzle().twists {
            if ui.button(&info.name).clicked() {
                let layers = LayerMask::default();
                let twist = LayeredTwist { layers, transform };
                p.sim()
                    .lock()
                    .do_event(ReplayEvent::Twists(smallvec![twist]));
            }
        }
    });
}
