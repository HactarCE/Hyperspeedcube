use hyperprefs::DEFAULT_PREFS;

use crate::L;
use crate::app::App;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    ui.group(|ui| {
        ui.set_width(ui.available_width());

        let prefs_ui = crate::gui::components::PrefsUi {
            ui,
            current: &mut app.prefs.interaction,
            defaults: Some(&DEFAULT_PREFS.interaction),
            changed: &mut changed,
        };

        crate::gui::components::prefs::build_interaction_section(prefs_ui);
    });

    app.prefs.needs_save |= changed;
}
