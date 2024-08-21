use crate::{app::App, L};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    let presets = &mut app.prefs.animation;

    let presets_ui = crate::gui::components::PresetsUi {
        id: unique_id!(),
        presets,
        changed: &mut changed,
        text: &L.presets.animation_settings,
        autosave: false,
        vscroll: true,
        help_contents: None,
        extra_validation: None,
    };
    presets_ui.show(
        ui,
        None,
        |p| p.animation.last_loaded_preset().cloned(),
        |prefs_ui| crate::gui::components::prefs::build_animation_section(prefs_ui),
    );

    // Copy settings back to active puzzle.
    if changed {
        let current_preset = presets.current_preset();
        app.with_active_puzzle_view(|p| {
            p.sim().lock().animation_prefs = current_preset;
        });
    }

    app.prefs.needs_save |= changed;
}
