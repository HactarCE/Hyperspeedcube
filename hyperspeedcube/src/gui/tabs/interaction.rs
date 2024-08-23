use crate::{app::App, L};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    let presets = &mut app.prefs.interaction;

    let presets_ui = crate::gui::components::PresetsUi {
        id: unique_id!(),
        presets,
        changed: &mut changed,
        text: &L.presets.interaction_settings,
        autosave: false,
        vscroll: true,
        help_contents: None,
        extra_validation: None,
    };
    presets_ui.show(
        ui,
        None,
        |p| p.interaction.last_loaded_preset().cloned(),
        crate::gui::components::prefs::build_interaction_section,
    );

    // Copy settings back to active puzzle.
    if changed {
        let current_preset = presets.current_preset();
        app.with_active_puzzle_view(|p| {
            p.sim().lock().interaction_prefs = current_preset;
        });
    }

    app.prefs.needs_save |= changed;
}
