use crate::app::App;
use crate::L;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    let presets_ui = crate::gui::components::PresetsUi {
        id: unique_id!(),
        presets: &mut app.prefs.animation,
        current: &mut app.animation_prefs,
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
        crate::gui::components::prefs::build_animation_section,
    );

    app.prefs.needs_save |= changed;
}
