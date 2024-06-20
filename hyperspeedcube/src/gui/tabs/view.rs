use egui::Widget;
use strum::IntoEnumIterator;

use crate::app::App;
use crate::gui::components::prefs::build_view_section;
use crate::gui::components::{big_icon_button, with_reset_button, PrefsUi};
use crate::gui::ext::ResponseExt;
use crate::gui::util::{set_widget_spacing_to_space_width, text_spacing, text_width};
use crate::preferences::{PuzzleViewPreferencesSet, DEFAULT_PREFS};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.set_enabled(app.has_active_puzzle_view());

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

    // ui.strong("Current preset");
    // ui.horizontal(|ui| {
    //     big_icon_button(ui, "🗑", &format!("Delete preset {}", NAME.lock()));
    //     big_icon_button(ui, "💾", &format!("Overwrite preset {}", NAME.lock()));
    //     with_reset_button(ui, &mut *NAME.lock(), LOADED.lock().clone(), "", |ui, s| {
    //         ui.add(egui::TextEdit::singleline(s).desired_width(150.0))
    //     });

    //     static A: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
    //     // ui.add_enabled_ui(A.load(std::sync::atomic::Ordering::Relaxed), |ui| {
    //     //     if ui.button("Save").clicked() {
    //     //         A.store(false, std::sync::atomic::Ordering::Relaxed);
    //     //     }
    //     // });
    // });
    // ui.collapsing("Defaults", |ui| {
    //     egui::ComboBox::new(unique_id!(), "Everything")
    //         .selected_text("(none)")
    //         .show_ui(ui, |ui| {
    //             ui.button("(none)");
    //             ui.button("Fallback");
    //             ui.button("Speedsolving");
    //             ui.button("Unfolded (back)");
    //             ui.button("Unfolded (fallback)");
    //             Some(())
    //         });
    //     egui::ComboBox::new(unique_id!(), "Cube")
    //         .selected_text("(none)")
    //         .show_ui(ui, |ui| {
    //             ui.button("(none)");
    //             ui.button("Fallback");
    //             ui.button("Speedsolving");
    //             ui.button("Unfolded (back)");
    //             ui.button("Unfolded (fallback)");
    //             Some(())
    //         });
    //     egui::ComboBox::new(unique_id!(), "3x3x3x3")
    //         .selected_text("(none)")
    //         .show_ui(ui, |ui| {
    //             ui.button("(none)");
    //             ui.button("Fallback");
    //             ui.button("Speedsolving");
    //             ui.button("Unfolded (back)");
    //             ui.button("Unfolded (fallback)");
    //             Some(())
    //         });
    // });

    // ui.separator();
}
