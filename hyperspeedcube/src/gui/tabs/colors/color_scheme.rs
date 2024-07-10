use std::sync::Arc;
use strum::IntoEnumIterator;

use hyperpuzzle::ColorSystem;

use crate::{
    app::App,
    gui::components::ReverseColorMap,
    preferences::{DefaultColorGradient, GlobalColorPalette, Preset},
};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let active_puzzle_ty = app.active_puzzle_type();
    let has_active_puzzle = active_puzzle_ty.is_some();
    ui.set_enabled(has_active_puzzle);

    let color_system = match &active_puzzle_ty {
        Some(puz) => Arc::clone(&puz.colors),
        None => Arc::new(ColorSystem::new_empty()),
    };

    let get_color_name = |id| color_system.list[id].name.clone();

    let mut changed = false;

    let color_system_prefs = app.prefs.color_schemes.color_system_mut(&color_system);

    // Ensure that the active color scheme is valid.
    let current = &mut color_system_prefs.schemes.current;
    changed |= app
        .prefs
        .color_palette
        .ensure_color_scheme_is_valid_for_color_system(current, &color_system);

    // Now that we know it's valid, we can generate the reverse map.
    let rev_map = ReverseColorMap::from_color_scheme(current);

    let mut presets_ui = crate::gui::components::PresetsUi {
        id: unique_id!(),
        presets: &mut color_system_prefs.schemes,
        changed: &mut changed,
        text: crate::gui::components::PresetsUiText {
            presets_set: Some(&color_system.name),
            preset: "color scheme",
            presets: "color schemes",
            what: "color scheme",
        },
    };
    presets_ui.show_presets_selector(ui);
    let mut default_preset = None;
    presets_ui.show_current_prefs_ui(
        ui,
        |_| {
            default_preset = Some(Preset {
                name: color_system.default_scheme.clone(),
                value: color_system
                    .default_scheme()
                    .iter()
                    .map(|(id, default_color)| (get_color_name(id), default_color.clone()))
                    .collect(),
            });
            default_preset.as_ref()
        },
        |prefs_ui| {
            let ui = prefs_ui.ui;
            ui.set_enabled(has_active_puzzle);
            show_color_palette(ui, &app.prefs.color_palette, &rev_map);
        },
    );

    // Copy settings back to active puzzle.
    if changed {
        let current_color_scheme = app
            .prefs
            .color_schemes
            .color_system_mut(&color_system)
            .schemes
            .current_preset();
        app.with_active_puzzle_view(|p| {
            p.view.colors = current_color_scheme;
        });
    }

    app.prefs.needs_save |= changed;
}

fn show_color_palette(
    ui: &mut egui::Ui,
    palette: &GlobalColorPalette,
    rev_map: &ReverseColorMap,
) -> egui::Response {
    let mut changed = false;

    let drag_state_id = ui.auto_id_with("hyperspeedcube::drag_state");
    let mut drag_state: crate::gui::components::ColorDragState =
        ui.data(|data| data.get_temp(drag_state_id).unwrap_or_default());

    ui.add(egui::Label::new(egui::RichText::from("Single colors").strong()).wrap(false));
    ui.add_space(ui.spacing().item_spacing.x - ui.spacing().item_spacing.y);
    ui.horizontal_wrapped(|ui| {
        for color_name in palette.singles.keys() {
            crate::gui::components::display_single_color(
                ui,
                palette,
                color_name.clone(),
                rev_map,
                &mut drag_state,
            );
        }
    });

    ui.separator();

    ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
    // ui.style_mut().spacing.scroll.foreground_color = true;

    let mut r = egui::ScrollArea::horizontal()
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut is_first = true;
                for (group_name, sets) in palette.groups_of_sets() {
                    if is_first {
                        is_first = false;
                    } else {
                        ui.separator();
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = ui.spacing_mut().item_spacing.yx();
                        ui.add(
                            egui::Label::new(egui::RichText::from(group_name).strong()).wrap(false),
                        );
                        for (set_name, _set) in sets {
                            crate::gui::components::display_color_set(
                                ui,
                                palette,
                                set_name,
                                rev_map,
                                &mut drag_state,
                            );
                        }
                    });
                }
            })
            .response
        })
        .inner;

    ui.separator();

    ui.strong("Gradients");
    for gradient in DefaultColorGradient::iter() {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            crate::gui::components::display_color_gradient(
                ui,
                palette,
                gradient,
                rev_map,
                &mut drag_state,
            );
        });
    }

    if changed {
        r.mark_changed();
    }
    r
}

fn strip_set_suffix(s: &str) -> &str {
    None.or_else(|| s.strip_suffix(" Dyad"))
        .or_else(|| s.strip_suffix(" Triad"))
        .or_else(|| s.strip_suffix(" Tetrad"))
        .or_else(|| s.strip_suffix(" Pentad"))
        .or_else(|| s.strip_suffix(" Hexad"))
        .or_else(|| s.strip_suffix(" Heptad"))
        .or_else(|| s.strip_suffix(" Octad"))
        .or_else(|| s.strip_suffix(" Nonad"))
        .or_else(|| s.strip_suffix(" Decad"))
        .unwrap_or(s)
}
