use crate::{
    app::App,
    gui::components::{reset_button, HintWidget, PrefsUi},
    preferences::DEFAULT_PREFS,
};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.strong("Global color palette");
            HintWidget::show(ui, |ui| {
                ui.heading("Global color palette");
                ui.label("TODO: explain the global color palette!");
            })
        });
        ui.separator();
        let mut active_only = false;
        ui.group(|ui| {
            let (_r, value) = basic_checkbox(ui, unique_id!(), "Show colors used in active puzzle");
            active_only = value;
        });

        // let default_sets = color_set

        // for set in DEFAULT_PREFS.color_sets.iter().map(|color_set|color_set.name)

        // let num_columns = (ui.available_width() / 300.0).floor().at_least(1.0) as usize;
        let num_columns = 1;
        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
            ui.columns(num_columns, |uis| {
                let ui = &mut uis[0];
                ui.group(|ui| {
                    ui.heading("Single colors");
                    ui.add_space(ui.spacing().item_spacing.y);
                    for i in 0..app.prefs.colors.len() {
                        // if let Some(saved_color) = app.prefs.colors.get_mut(i) {
                        //     ui.horizontal(|ui| {
                        //         ui.set_width(300.0);
                        //         // let r = ui.color_edit_button_srgb(saved_color);
                        //         // changed |= r.changed();
                        //         if i < 7 {
                        //             color_label(
                        //                 ui,
                        //                 &saved_color.name,
                        //                 active_only.then_some(i % 7 == 0),
                        //             );
                        //         } else {
                        //             let r =
                        //                 egui::TextEdit::singleline(&mut saved_color.name).show(ui);
                        //             changed |= r.response.changed();
                        //         }
                        //     });
                        // }
                    }
                });

                // let ui = &mut uis[1];

                let split = app.prefs.color_sets.len() / 2;
                ui.group(|ui| {
                    ui.heading("Triads");
                    ui.add_space(ui.spacing().item_spacing.y);
                    for i in 0..split {
                        let mut changed = false;
                        let mut prefs_ui = PrefsUi {
                            ui,
                            current: &mut app.prefs.color_sets,
                            defaults: &DEFAULT_PREFS.color_sets,
                            changed: &mut changed,
                        };
                        // let name = prefs_ui.current[i]
                        //     .name
                        //     .strip_suffix(" triad")
                        //     .unwrap()
                        //     .to_string();
                        // prefs_ui.fixed_multi_color(&name, access!([i].colors));
                        // color_label(
                        //     ui,
                        //     ,
                        //     active_only.then_some(i % 8 == 0),
                        // );
                    }
                });

                // let ui = &mut uis[2];

                ui.group(|ui| {
                    ui.heading("Tetrads");
                    ui.add_space(ui.spacing().item_spacing.y);
                    for (i, set) in &mut app.prefs.color_sets[split..].iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            // for color in &mut set.colors {
                            //     // ui.color_edit_button_srgb(color);
                            // }
                            // color_label(
                            //     ui,
                            //     set.name.strip_suffix(" tetrad").unwrap(),
                            //     active_only.then_some(i % 4 == 2),
                            // );
                        });
                    }
                });
            });
        });
    });

    app.prefs.needs_save |= changed;
}

// TODO: pair/dyad, triad, tetrad, pentad, hexad, heptad, octad

fn color_label(ui: &mut egui::Ui, s: &str, highlight: Option<bool>) -> egui::Response {
    match highlight {
        None => ui.label(s),
        Some(true) => ui.strong(s),
        Some(false) => ui.add_enabled(false, egui::Label::new(s)),
    }
}

fn basic_checkbox(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: impl Into<egui::WidgetText>,
) -> (egui::Response, bool) {
    let mut value = ui.data(|data| data.get_temp(id).unwrap_or(false));
    let r = ui.checkbox(&mut value, text);
    ui.data_mut(|data| data.insert_temp(id, value));
    (r, value)
}
