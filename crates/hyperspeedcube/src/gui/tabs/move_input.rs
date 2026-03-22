use hyperpuzzle::notation::Invert;
use hyperpuzzle::symmetric::SymmetricTwistSystemEngineData;
use hyperpuzzle::{LayerMask, Move};
use hyperpuzzle_view::ReplayEvent;
use itertools::Itertools;
use smallvec::{SmallVec, smallvec};

use crate::L;
use crate::app::App;
use crate::gui::util::EguiTempValue;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    app.active_puzzle.with_opt_view(|view| {
        let Some(view) = view else {
            ui.label(L.no_active_puzzle);
            return;
        };

        let mut move_input_value = EguiTempValue::new(ui);
        let mut s: String = move_input_value.get().unwrap_or_default();
        ui.text_edit_singleline(&mut s);
        let parsed =
            hyperpuzzle::notation::parse_notation(&s, hyperpuzzle::notation::Features::MAXIMAL)
                .map_err(|e| e.into_iter().join("\n"));
        move_input_value.set(Some(s));
        ui.scope(|ui| {
            if parsed.is_err() {
                ui.disable();
            }
            let mut sim = view.sim.lock();
            if ui.button("Execute moves").clicked()
                && let Ok(node_list) = &parsed
                && let Ok(moves) = node_list.flatten()
            {
                sim.do_event(ReplayEvent::Twists(
                    moves.into_iter().filter_map(|mv| mv.into_move()).collect(),
                ));
            }
            if ui.button("Execute inverse").clicked()
                && let Ok(node_list) = &parsed
                && let Ok(moves) = node_list.flatten()
                && let Ok(moves) = moves.inv()
            {
                sim.do_event(ReplayEvent::Twists(
                    moves.into_iter().filter_map(|mv| mv.into_move()).collect(),
                ));
            }
        });
        match parsed {
            Err(e) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            Ok(node_list) => {
                if let Some(symmetric) = view
                    .puzzle()
                    .twists
                    .engine_data
                    .downcast_ref::<SymmetricTwistSystemEngineData>()
                    && let Some(e) = node_list.iter().find_map(|node| {
                        symmetric
                            .resolve_twist_transform(&node.as_move()?.transform)
                            .err()
                    })
                {
                    ui.colored_label(ui.visuals().error_fg_color, e.to_string());
                }
            }
        }

        ui.group(|ui| {
            ui.strong("Axis names");
            for name in view.puzzle().axes().names.iter_values() {
                ui.label(&name.preferred);
            }
        });

        // let puz = view.puzzle();
        // egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        //     for transform in puz.twists.twists.iter_keys() {
        //         if ui.button(&puz.twists.names[transform]).clicked() {
        //             let twist = Move::new((), &puz.twists.names[transform], None, 1);
        //             view.sim
        //                 .lock()
        //                 .do_event(ReplayEvent::Twists(smallvec![twist]));
        //         }
        //     }
        // });
    });
}
