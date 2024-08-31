use crate::{app::App, L};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    let presets = &mut app.prefs.interaction;

    let presets_ui = crate::gui::components::PresetsUi {
        id: unique_id!(),
        presets,
        current: &mut app.interaction_prefs,
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
        crate::gui::components::prefs::build_interaction_section,
    );

    app.prefs.needs_save |= changed;
}
