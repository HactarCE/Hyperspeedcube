use crate::app::App;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.set_enabled(app.has_active_puzzle());

    let prefs_set = app.prefs.latest_view_prefs_set;
    let presets = app.prefs.view_presets_mut();

    let mut changed = false;

    let mut presets_ui = crate::gui::components::PresetsUi {
        id: unique_id!(),
        presets,
        changed: &mut changed,
        text: crate::gui::components::PresetsUiText {
            presets_set: Some(prefs_set.as_ref()),
            what: "view settings",
            ..Default::default()
        },
    };
    presets_ui.show_presets_selector(ui);
    presets_ui.show_current_prefs_ui(
        ui,
        |p| p[prefs_set].last_loaded_preset(),
        |prefs_ui| crate::gui::components::prefs::build_view_section(prefs_set, prefs_ui),
    );

    // Copy settings back to active puzzle.
    if changed {
        let current_preset = presets.current_preset();
        app.with_active_puzzle_view(|p| {
            p.view.camera.view_preset = current_preset;
            // TODO: tell it to redraw?
        });
    }

    app.prefs.needs_save |= changed;
}
