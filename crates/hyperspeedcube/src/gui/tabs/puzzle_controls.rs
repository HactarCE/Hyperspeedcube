use hyperpuzzle::prelude::*;
use hyperpuzzle_view::ReplayEvent;
use smallvec::smallvec;

use crate::L;
use crate::app::App;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    app.active_puzzle.with_opt_view(|view| {
        let Some(view) = view else {
            ui.label(L.no_active_puzzle);
            return;
        };

        let puz = view.puzzle();
        for (transform, info) in &puz.twists.twists {
            if ui.button(&puz.twists.names[transform]).clicked() {
                let layers = LayerMask::default();
                let twist = LayeredTwist { layers, transform };
                view.sim
                    .lock()
                    .do_event(ReplayEvent::Twists(smallvec![twist]));
            }
        }
    });
}
