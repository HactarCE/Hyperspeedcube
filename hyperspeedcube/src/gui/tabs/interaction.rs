use crate::app::App;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.set_enabled(app.has_active_puzzle());

    let mut changed = false;

    let presets = &mut app.prefs.interaction;

    let mut presets_ui = crate::gui::components::PresetsUi {
        id: unique_id!(),
        presets,
        changed: &mut changed,
    };
    presets_ui.show_presets_selector(ui, |_| ());
    presets_ui.show_current_prefs_ui(
        ui,
        |p| p.interaction.last_loaded_preset(),
        |prefs_ui| crate::gui::components::prefs::build_interaction_section(prefs_ui),
    );

    // Copy settings back to active puzzle.
    if changed {
        if let Some(current) = presets.current_preset() {
            app.with_active_puzzle_view(|p| {
                p.sim().lock().interaction_prefs = current;
            });
        }
    }

    app.prefs.needs_save |= changed;
}
