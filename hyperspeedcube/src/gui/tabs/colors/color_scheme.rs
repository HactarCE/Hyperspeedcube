use indexmap::IndexMap;
use std::sync::Arc;
use strum::IntoEnumIterator;

use hyperpuzzle::{ColorSystem, DefaultColor};

use crate::{
    app::App,
    gui::{components::ReverseColorMap, util::set_widget_spacing_to_space_width},
    preferences::{BeforeOrAfter, DefaultColorGradient, GlobalColorPalette, Preset},
};

fn show_color_schemes_help_ui(ui: &mut egui::Ui) {
    // TODO: markdown renderer
    ui.spacing_mut().item_spacing.y = 9.0;
    ui.heading("Color assignments");
    ui.horizontal_wrapped(|ui| {
        set_widget_spacing_to_space_width(ui);
        ui.label(
            "Each facet on the puzzle is assigned to a color. \
            Color values can be customized in the",
        );
        ui.strong("global color palette");
        ui.add_space(-ui.spacing().item_spacing.x);
        ui.label(".");
    });
    crate::gui::util::bullet_list(
        ui,
        &[
            "Single colors are best for small puzzles",
            "Color sets are best for medium puzzles",
            "Gradients are best for large puzzles",
            "Colors within a color set are designed to contrast with \
             each other and with other color sets of the same size",
            "Drag a facet name to assign it to a different color",
        ],
    );
}

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
            let mut dnd = crate::gui::components::DragAndDrop::new(ui).dragging_opacity(1.0);
            show_color_palette(ui, &app.prefs.color_palette, &rev_map, &mut dnd);
            if let Some(r) = dnd.take_response() {
                match r.before_or_after {
                    Some(before_or_after) => reorder_color_to(
                        prefs_ui.current,
                        &rev_map,
                        r.payload,
                        r.end,
                        before_or_after,
                    ),
                    None => swap_color_to(prefs_ui.current, &rev_map, r.payload, r.end),
                }
                let _ = app
                    .prefs
                    .color_palette
                    .ensure_color_scheme_is_valid_for_color_system(prefs_ui.current, &color_system);
                *prefs_ui.changed = true;
            }
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
    dnd: &mut crate::gui::components::DragAndDrop<String, DefaultColor>,
) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(egui::RichText::from("Single colors").strong()).wrap(false));
        crate::gui::components::HintWidget::show(ui, show_color_schemes_help_ui);
    });
    ui.add_space(ui.spacing().item_spacing.x - ui.spacing().item_spacing.y);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.y = ui.spacing().item_spacing.x;
        for color_name in palette.singles.keys() {
            crate::gui::components::display_single_color(
                ui,
                palette,
                color_name.clone(),
                rev_map,
                dnd,
            );
        }
    });

    ui.separator();

    ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();

    egui::ScrollArea::horizontal()
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
                        ui.spacing_mut().item_spacing = ui.spacing().item_spacing.yx();
                        ui.add(
                            egui::Label::new(egui::RichText::from(group_name).strong()).wrap(false),
                        );
                        for (set_name, _set) in sets {
                            crate::gui::components::display_color_set(
                                ui, palette, set_name, rev_map, dnd,
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
        crate::gui::components::display_color_gradient(ui, palette, gradient, rev_map, dnd);
    }
}

fn reorder_color_to(
    map: &mut IndexMap<String, DefaultColor>,
    rev_map: &ReverseColorMap,
    name: String,
    new_default_color: DefaultColor,
    before_or_after: BeforeOrAfter,
) {
    let DefaultColor::Gradient {
        gradient_name,
        mut index,
        total: _,
    } = &new_default_color
    else {
        log::error!("attempt to reorder color to something other than a gradient");
        return;
    };

    if before_or_after == BeforeOrAfter::After {
        index += 1;
    }

    let Ok(gradient) = gradient_name.parse::<DefaultColorGradient>() else {
        log::error!("unknown gradient name {gradient_name:?}");
        return;
    };

    // Shift other colors up by one.
    let total = *rev_map.gradient_totals.get(&gradient).unwrap_or(&0);
    for i in index..total {
        if let Some(name) = rev_map.colors.get(&DefaultColor::Gradient {
            gradient_name: gradient_name.clone(),
            index: i,
            total,
        }) {
            map.insert(
                name.clone(),
                DefaultColor::Gradient {
                    gradient_name: gradient_name.clone(),
                    index: i + 1,
                    total: total + 1,
                },
            );
        }
    }

    // Insert the new color.
    map.insert(name, new_default_color);
}

fn swap_color_to(
    map: &mut IndexMap<String, DefaultColor>,
    rev_map: &ReverseColorMap,
    name: String,
    new_default_color: DefaultColor,
) {
    let old_name = rev_map.colors.get(&new_default_color);
    let old_default_color = map.insert(name, new_default_color);

    if let Some(old_default_color) = old_default_color {
        if let Some(old_name) = old_name {
            map.insert(old_name.clone(), old_default_color);
        }
    }
}
