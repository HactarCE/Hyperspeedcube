use super::{Location, Window};
use crate::gui::components::{
    GlobalKeybindsAccessor, KeybindIncludesList, KeybindSetsList, KeybindsTable,
    PuzzleKeybindsAccessor,
};

pub(crate) const GLOBAL_KEYBINDS: Window = Window {
    name: "Global keybinds",
    location: Location::LeftSide,
    build: |ui, app| {
        let r = ui.add(KeybindsTable {
            app,
            keybind_set: GlobalKeybindsAccessor,
        });
        app.prefs.needs_save |= r.changed();
    },
    ..Window::DEFAULT
};

pub(crate) const PUZZLE_KEYBINDS: Window = Window {
    name: "Puzzle keybinds",
    location: Location::LeftSide,
    build: |ui, app| {
        let puzzle_type = app.puzzle.ty();

        egui::CollapsingHeader::new("Keybind sets")
            .default_open(true)
            .show(ui, |ui| ui.add(KeybindSetsList { app }));
        ui.separator();
        egui::CollapsingHeader::new("Include")
            .default_open(true)
            .show(ui, |ui| ui.add(KeybindIncludesList { app }));
        ui.separator();
        egui::CollapsingHeader::new("Keybinds")
            .default_open(true)
            .show(ui, |ui| {
                let set_name = app.prefs.puzzle_keybinds[puzzle_type].active.clone();

                // Show keybinds table.
                let r = ui.add(KeybindsTable {
                    app,
                    keybind_set: PuzzleKeybindsAccessor {
                        puzzle_type,
                        set_name,
                    },
                });
                app.prefs.needs_save |= r.changed();
            });
    },
    ..Window::DEFAULT
};
