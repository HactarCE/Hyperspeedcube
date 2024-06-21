use crate::app::App;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.set_enabled(app.has_active_puzzle());

    let prefs_set = app.prefs.latest_view_prefs_set;
    let mut changed = false;
    let presets = app.prefs.view_presets_mut();

    let mut presets_ui = crate::gui::components::PresetsUi {
        id: unique_id!(),
        presets,
        changed: &mut changed,
    };
    presets_ui.show_presets_selector(ui, |ui| {
        ui.label(format!("({prefs_set})"));
    });
    presets_ui.show_current_prefs_ui(
        ui,
        |p| p[prefs_set].last_loaded_preset(),
        |prefs_ui| crate::gui::components::prefs::build_view_section(prefs_set, prefs_ui),
    );

    // Copy settings back to active puzzle.
    if changed {
        if let Some(current) = presets.current_preset() {
            app.with_active_puzzle_view(|p| {
                p.view.camera.view_preset = current;
                // TODO: tell it to redraw?
            });
        }
    }

    app.prefs.needs_save |= changed;
}
