use hyperpuzzle::{LayerMask, Move};
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
        egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
            for transform in puz.twists.twists.iter_keys() {
                if ui.button(&puz.twists.names[transform]).clicked() {
                    let twist = Move::new((), &puz.twists.names[transform], None, 1);
                    view.sim
                        .lock()
                        .do_event(ReplayEvent::Twists(smallvec![twist]));
                }
            }
        });
    });
}
