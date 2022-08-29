use crate::app::App;
use crate::gui::widgets;
use crate::preferences::KeybindSet;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_keybinds = &mut app.prefs.puzzle_keybinds[app.puzzle.ty()];

    let mut changed = false;

    let mut presets_ui = widgets::PresetsUi {
        id: unique_id!(),
        presets: &mut puzzle_keybinds.sets,
        changed: &mut changed,
        strings: widgets::PresetsUiStrings {
            edit: "Edit keybind sets",
            save: "Add new keybind set",
            name: "Keybind set name",
        },
        enable_yaml: false,
    };

    presets_ui.show_header_with_active_preset(ui, KeybindSet::default, |new_preset| {
        puzzle_keybinds.active = new_preset.preset_name.clone();
    });
    ui.separator();
    presets_ui.show_list(ui, |ui, _idx, set| {
        let mut changed = false;

        let mut r = ui.with_layout(
            egui::Layout::centered_and_justified(egui::Direction::TopDown)
                .with_cross_align(egui::Align::LEFT),
            |ui| {
                // Highlight name of active keybind set.
                if puzzle_keybinds.active == set.preset_name {
                    let visuals = ui.visuals_mut();
                    visuals.widgets.hovered = visuals.widgets.active;
                    visuals.widgets.inactive = visuals.widgets.active;
                }

                if ui
                    .add(egui::Button::new(&set.preset_name).frame(false))
                    .clicked()
                {
                    changed = true;
                    puzzle_keybinds.active = set.preset_name.clone();
                }
            },
        );

        if changed {
            r.response.mark_changed();
        }
        r.response
    });

    app.prefs.needs_save |= changed;
}
