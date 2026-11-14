use hsc_view::ReplayEvent;
use hyperpuzzle::euclid::SymmetricTwistSystemComponent;
use hyperpuzzle::notation::Invert;
use itertools::Itertools;

use crate::L;
use crate::app::App;
use crate::gui::EguiValue;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    app.active_puzzle.with_opt_view(|view| {
        let Some(view) = view else {
            ui.label(L.no_active_puzzle);
            return;
        };

        ui.group(|ui| {
            let mut prefs_ui = crate::gui::components::PrefsUi {
                ui,
                current: &mut app.animation_prefs.value,
                defaults: app.prefs.animation.last_loaded().map(|p| &p.value),
                changed: &mut false, // don't care. it's a preset
            };

            let l = &L.prefs.animation.twists;
            prefs_ui.checkbox(&l.dynamic_twist_speed, access!(.dynamic_twist_speed));
            prefs_ui.animation_duration(&l.twist_duration, access!(.twist_duration));
        });

        let mut move_input_value = EguiValue::<String>::load_or_default(ui, unique_id!());
        ui.text_edit_multiline(&mut *move_input_value);
        let parsed = hyperpuzzle::notation::parse_notation(
            &move_input_value,
            hyperpuzzle::notation::Features::MAXIMAL,
        )
        .map_err(|e| e.into_iter().join("\n"));
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
                if let Ok(symmetric) = view
                    .puzzle()
                    .twists
                    .components
                    .get::<SymmetricTwistSystemComponent>()
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

        let mut seed_value = EguiValue::<String>::load_or_default(ui, unique_id!());
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Seed:");
            ui.text_edit_singleline(&mut *seed_value);
        });
        if ui.button("Set random twists").clicked() {
            let mut rng = hyperpuzzle::util::rng_from_seed(&seed_value);
            let twist_count = 10;
            *move_input_value = std::iter::from_fn(|| (view.puzzle().random_move)(&mut rng))
                .take(twist_count)
                .join("\n");
        }

        ui.group(|ui| {
            ui.strong("Axis names");
            for name in view.puzzle().axes().names.list().iter_values() {
                ui.label(&**name);
            }
        });

        /*
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
        */
    });
}
