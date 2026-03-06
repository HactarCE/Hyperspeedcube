use hyperprefs::{ModifiedPreset, PresetData, PresetsList};

use crate::L;
use crate::gui::components::{PrefsUi, PresetsUi};
use crate::gui::util::menu_button_that_stays_open;
use crate::gui::{App, AppUi};

pub fn build_quick_settings_ui(ui: &mut egui::Ui, app: &mut App) {
    egui::containers::menu::MenuButton::new(mdi!(ui, COG))
        .config(
            egui::containers::menu::MenuConfig::default()
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .style(egui::style::StyleModifier::default()),
        )
        .ui(ui, |ui: &mut egui::Ui| {
            let mut changed = false;

            ui.heading(L.quick_settings);

            ui.separator();

            // Animation
            compact_presets_selector_ui(
                ui,
                L.tabs.animation.menu,
                &mut app.prefs.animation,
                &mut app.animation_prefs,
                &mut changed,
            );

            ui.separator();

            app.active_puzzle.with_view(|v| {
                // View
                let view_prefs_set = v.puzzle().view_prefs_set();
                if let Some(camera) = v.nd_euclid_camera_mut() {
                    match view_prefs_set {
                        Some(hyperpuzzle::PuzzleViewPreferencesSet::Perspective(dim)) => {
                            let mut presets = app.prefs.perspective_view_presets_mut(dim);
                            compact_presets_selector_ui(
                                ui,
                                L.tabs.view.menu,
                                presets,
                                &mut camera.view_preset,
                                &mut changed,
                            );

                            ui.separator();

                            let defaults = match presets.last_loaded() {
                                Some(p) => p.value.clone(),
                                None => Default::default(),
                            };
                            let mut prefs_ui = PrefsUi {
                                ui,
                                current: &mut camera.view_preset.value,
                                defaults: Some(&defaults),
                                changed: &mut changed,
                            };
                            crate::gui::components::prefs::build_view_geometry_section(
                                &mut prefs_ui,
                            );
                        }
                        None => (),
                    }
                }
            });

            app.prefs.needs_save |= changed;
        });
}

fn compact_presets_selector_ui<T: PresetData + Clone>(
    ui: &mut egui::Ui,
    name: &str,
    presets: &mut PresetsList<T>,
    current: &mut ModifiedPreset<T>,
    changed: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.strong(name);
        egui::ScrollArea::horizontal()
            .id_salt(name)
            .auto_shrink(true)
            .show(ui, |ui| {
                let mut preset_to_activate = None;
                for preset in presets.user_presets() {
                    let is_active = *preset.name() == current.base.name();
                    if ui.selectable_label(is_active, preset.name()).clicked() {
                        preset_to_activate = Some(preset.name().clone());
                    }
                }
                if let Some(p) = preset_to_activate.and_then(|s| presets.load(&s)) {
                    *current = p;
                    *changed = true;
                }
            });
    });
}
