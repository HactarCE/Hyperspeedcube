use std::sync::Arc;

use hyperpuzzle::ColorSystem;

use crate::{
    app::App,
    preferences::{ColorSystemPreferences, Preset},
};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let active_puzzle_ty = app.active_puzzle_type();
    let has_active_puzzle = active_puzzle_ty.is_some();
    ui.set_enabled(has_active_puzzle);

    let color_system = match &active_puzzle_ty {
        Some(puz) => Arc::clone(&puz.colors),
        None => Arc::new(ColorSystem::new_empty()),
    };
    let mut empty_color_system_prefs = ColorSystemPreferences::default();
    let color_system_prefs = match active_puzzle_ty {
        Some(_) => app.prefs.color_schemes.color_system_mut(&color_system),
        None => &mut empty_color_system_prefs,
    };

    let get_color_name = |id| color_system.list[id].name.clone();

    let mut changed = false;

    // Ensure that the active color scheme is valid.
    let current = &mut color_system_prefs.schemes.current;
    changed |= app
        .prefs
        .color_palette
        .ensure_color_scheme_is_valid_for_color_system(current, &color_system);

    let presets_ui = crate::gui::components::PresetsUi {
        id: unique_id!(),
        presets: &mut color_system_prefs.schemes,
        changed: &mut changed,
        text: crate::gui::components::PresetsUiText {
            presets_set: Some(&color_system.name),
            preset: "color scheme",
            saved_presets: "Saved color schemes",
            what: "color scheme",
        },
        autosave: false,
        vscroll: true,
        help_contents: Some(Box::new(
            crate::gui::components::show_color_schemes_help_ui(true),
        )),
        extra_validation: None,
    };

    let mut temp_colors_override = None; // temporary color override
    let get_backup_defaults = |_| {
        Some(Preset {
            name: color_system.default_scheme.clone(),
            value: color_system
                .default_scheme()
                .iter()
                .map(|(id, default_color)| (get_color_name(id), default_color.clone()))
                .collect(),
        })
    };
    presets_ui.show(ui, get_backup_defaults, |mut prefs_ui| {
        let (prefs, ui) = prefs_ui.split();

        ui.set_enabled(has_active_puzzle);

        let mut colors_ui = crate::gui::components::ColorsUi::new(&app.prefs.color_palette)
            .clickable(false)
            .drag_puzzle_colors(ui, true);

        let (changed, temp_scheme) =
            colors_ui.show_compact_palette(ui, Some((prefs.current, &color_system)), None);
        *prefs.changed |= changed;
        temp_colors_override = temp_scheme;
    });

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
    if let Some(temp_colors) = temp_colors_override {
        app.with_active_puzzle_view(|p| p.view.temp_colors = Some(temp_colors));
    }

    app.prefs.needs_save |= changed;
}
