use super::{Window, PREFS_WINDOW_WIDTH};
use crate::app::App;

pub(crate) const KEYBIND_SETS: Window = Window {
    name: "Keybind sets",
    fixed_width: Some(PREFS_WINDOW_WIDTH),
    build,
    ..Window::DEFAULT
};

const HIDDEN_PREFIX_CHAR: char = '^';

fn build(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_keybinds = &mut app.prefs.puzzle_keybinds[app.puzzle.ty()];

    let mut changed = false;

    if ui.button("Manage keybind sets").clicked() {
        super::PUZZLE_KEYBINDS.set_open(ui.ctx(), true);
    }

    ui.with_layout(
        egui::Layout::centered_and_justified(egui::Direction::TopDown)
            .with_main_justify(false)
            .with_cross_align(egui::Align::LEFT),
        |ui| {
            for set in &puzzle_keybinds.sets {
                if !set.preset_name.starts_with(HIDDEN_PREFIX_CHAR) {
                    let r = ui.selectable_value(
                        &mut puzzle_keybinds.active,
                        set.preset_name.clone(),
                        &set.preset_name,
                    );
                    changed |= r.changed();
                }
            }
        },
    );

    app.prefs.needs_save |= changed;
}
