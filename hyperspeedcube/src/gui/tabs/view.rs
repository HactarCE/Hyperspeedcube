use hyperprefs::{ModifiedPreset, PresetsList, PuzzleViewPreferencesSet, ViewPreferences};

use crate::app::App;
use crate::gui::components::PresetsUi;
use crate::L;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let id = unique_id!();

    app.active_puzzle_view.with_opt(|p| {
        if let Some(p) = p {
            let mut changed = false;

            let prefs_set = PuzzleViewPreferencesSet::from_ndim(p.puzzle().ndim());
            let presets = app.prefs.view_presets_mut(prefs_set);
            let current = &mut p.view.camera.view_preset;
            let presets_ui = PresetsUi::new(id, presets, current, &mut changed);
            show_contents(ui, Some(prefs_set), presets_ui);

            app.prefs.needs_save |= changed;
        } else {
            ui.disable();

            let mut presets = PresetsList::default();
            let mut current = ModifiedPreset::default();
            show_contents(
                ui,
                None,
                PresetsUi::new(id, &mut presets, &mut current, &mut false),
            );
        }
    });
}

fn show_contents(
    ui: &mut egui::Ui,
    prefs_set: Option<PuzzleViewPreferencesSet>,
    presets_ui: PresetsUi<'_, ViewPreferences>,
) {
    let presets_set = prefs_set.as_ref().map(|s| s.as_ref());
    presets_ui
        .with_text(&L.presets.view_settings)
        .show(ui, presets_set, |prefs_ui| {
            crate::gui::components::prefs::build_view_section(prefs_set, prefs_ui);
        });
}
