use std::collections::HashSet;

use hyperpuzzle::DefaultColor;
use indexmap::map::MutableKeys;

use crate::app::App;
use crate::gui::components::{reset_button, HintWidget, PrefsUi, SMALL_ICON_BUTTON_SIZE};
use crate::preferences::DEFAULT_PREFS;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    // let active_colors = match app.active_puzzle_type() {
    //     Some(p) => p
    //         .colors
    //         .iter_values()
    //         .filter_map(|c| c.default_color.clone())
    //         .collect(),
    //     None => HashSet::new(),
    // };

    let active_colors = HashSet::<DefaultColor>::new();

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
            ui.set_width(ui.available_width());
            let (_r, value) = basic_checkbox(ui, unique_id!(), "Show colors used in active puzzle");
            active_only = value;
        });

        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.set_height(22.0);
                ui.strong("Single colors");
            });
            ui.separator();
            // ui.add_space(ui.spacing().item_spacing.y);

            let mut prefs_ui = PrefsUi {
                ui,
                current: &mut app.prefs.colors,
                defaults: &DEFAULT_PREFS.colors,
                changed: &mut changed,
            };

            for (i, color_name) in DEFAULT_PREFS.colors.singles.keys().enumerate() {
                let is_active = active_only.then(|| {
                    active_colors.contains(&DefaultColor::Single {
                        name: color_name.clone(),
                    })
                });
                prefs_ui.color(&color_name, access!(.singles[i]), is_active);
            }
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
