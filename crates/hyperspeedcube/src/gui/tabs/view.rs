use hypermath::pga::Motor;
use hyperprefs::{ModifiedPreset, PresetsList, ViewPreferences};
use hyperpuzzle::prelude::*;
use hyperpuzzle_view::PuzzleView;

use crate::L;
use crate::app::App;
use crate::gui::components::PresetsUi;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let id = unique_id!();

    let mut changed = false;

    app.active_puzzle.with_opt_view(|view| {
        if let Some(view) = view {
            match view.puzzle().view_prefs_set() {
                Some(PuzzleViewPreferencesSet::Perspective(dim)) => {
                    let presets = app.prefs.perspective_view_presets_mut(dim);
                    if let Some(cam) = view.nd_euclid_camera_mut() {
                        let presets_ui =
                            PresetsUi::new(id, presets, &mut cam.view_preset, &mut changed);
                        show_contents_for_perspective(ui, dim, presets_ui, &mut cam.rot);
                    } else {
                        show_disabled_contents(ui, id);
                    }
                }
                None => show_disabled_contents(ui, id),
            }
        } else {
            show_disabled_contents(ui, id);
        }
    });

    app.prefs.needs_save |= changed;
}

fn show_disabled_contents(ui: &mut egui::Ui, id: egui::Id) {
    let mut presets = PresetsList::<()>::default();
    let mut current = ModifiedPreset::default();

    ui.disable();
    PresetsUi::new(id, &mut presets, &mut current, &mut false)
        .with_text(&L.presets.view_settings)
        .show(ui, None, |_prefs_ui| ());
}

fn show_contents_for_perspective(
    ui: &mut egui::Ui,
    dim: PerspectiveDim,
    presets_ui: PresetsUi<'_, ViewPreferences>,
    rot: &mut Motor,
) {
    let presets_set = dim.as_ref();
    presets_ui
        .with_text(&L.presets.view_settings)
        .show(ui, Some(presets_set), |mut prefs_ui| {
            crate::gui::components::prefs::build_perspective_dim_view_section(dim, &mut prefs_ui);
            prefs_ui.ui.add_enabled_ui(!rot.is_ident(), |ui| {
                if ui.button(L.prefs.view.reset).clicked() {
                    *rot = Motor::ident(rot.ndim());
                }
            });
        });
}
