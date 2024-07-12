use std::collections::HashSet;

use hyperpuzzle::{DefaultColor, Rgb};
use indexmap::map::MutableKeys;
use rand::Rng;

use crate::app::App;
use crate::gui::components::{
    reset_button, HelpHoverWidget, PrefsUi, BIG_ICON_BUTTON_SIZE, SMALL_ICON_BUTTON_SIZE,
};
use crate::gui::util::set_widget_spacing_to_space_width;
use crate::preferences::DEFAULT_PREFS;

fn show_global_color_palette_help_ui(ui: &mut egui::Ui) {
    // TODO: markdown renderer
    ui.spacing_mut().item_spacing.y = 9.0;
    ui.heading("Global color palette");
    ui.horizontal_wrapped(|ui| {
        set_widget_spacing_to_space_width(ui);
        ui.label(
            "The global color palette provides a way to change colors \
             across all puzzles at once. For example, you can select a \
             particular shade of red to use on every puzzle with red \
             stickers.\n\
             \n\
             Some colors are organized into sets of colors that are \
             similar but still contrast with each other. For example, \
             a puzzle with two different shades of red needs those \
             shades to be distinguishable, so it uses the \"red dyad\" \
             from the global color palette.\n\
             \n\
             The color scheme for any particular puzzle can be customized in the",
        );
        ui.strong("color scheme");
        ui.label("settings.");
    });
}

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

    let rev_map = app
        .with_active_puzzle_view(|p| {
            let color_scheme = &mut p.view.colors.value;
            crate::gui::components::ReverseColorMap::from_color_scheme(color_scheme)
        })
        .unwrap_or_default();

    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.strong("Single colors");
            HelpHoverWidget::show(ui, show_global_color_palette_help_ui);
        });

        let mut prefs_ui = PrefsUi {
            ui,
            current: &mut app.prefs.color_palette,
            defaults: Some(&DEFAULT_PREFS.color_palette),
            changed: &mut changed,
        };

        for (i, color_name) in DEFAULT_PREFS.color_palette.singles.keys().enumerate() {
            prefs_ui.color(
                &color_name,
                access!(.singles[i]),
                rev_map.colors.get(&DefaultColor::Single {
                    name: color_name.clone(),
                }),
            );
        }
    });

    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong("Custom single colors");

        let mut dnd = crate::gui::components::DragAndDrop::new(ui);
        let mut to_delete = None;

        for (i, (name, color)) in app
            .prefs
            .color_palette
            .custom_singles
            .iter_mut()
            .enumerate()
        {
            let r = dnd.draggable(ui, i, |ui, _is_dragging| {
                let r = ui.horizontal(|ui| {
                    ui.set_width(ui.available_width());

                    let drag_response = crate::gui::components::drag_handle(ui);

                    // if crate::gui::components::small_icon_button(ui, "🗑", &format!("Delete {name}"))
                    //     .clicked()
                    // {
                    //     to_delete = Some(i);
                    // }

                    crate::gui::components::color_edit(
                        ui,
                        color,
                        rev_map
                            .colors
                            .get(&DefaultColor::Single { name: name.clone() }),
                        name,
                    );

                    drag_response
                });
                let drag_response = r.inner;
                r.response | drag_response
            });
            changed |= r.changed();
            dnd.reorder_drop_zone(ui, r, i);
        }

        dnd.draw_reorder_drop_lines(ui);
        if let Some(r) = dnd.end_drag() {
            if let Some(before_or_after) = r.before_or_after {
                crate::util::reorder_map(
                    &mut app.prefs.color_palette.custom_singles,
                    r.payload,
                    r.end,
                    before_or_after,
                );
                changed = true;
            }
        }

        if let Some(i) = to_delete {
            app.prefs.color_palette.custom_singles.shift_remove_index(i);
            changed = true;
        }

        if ui.button("Add color").clicked() {
            app.prefs.color_palette.custom_singles.insert(
                "new color".to_string(),
                Rgb {
                    rgb: rand::thread_rng().gen(),
                },
            );
        }
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
