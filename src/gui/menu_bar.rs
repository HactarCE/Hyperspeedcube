use crate::app::App;
use crate::commands::Command;
use crate::puzzle::{PuzzleControllerTrait, PuzzleType, PuzzleTypeTrait};

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            let can_save = app.puzzle.ty() == PuzzleType::Rubiks4D;

            if ui.button("Open").clicked() {
                ui.close_menu();
                app.event(Command::Open);
            }
            ui.separator();
            ui.add_enabled_ui(can_save, |ui| {
                if ui.button("Save").clicked() {
                    ui.close_menu();
                    app.event(Command::Save);
                }
                if ui.button("Save As...").clicked() {
                    ui.close_menu();
                    app.event(Command::SaveAs);
                }
            });
            ui.separator();
            if ui.button("Exit").clicked() {
                ui.close_menu();
                app.event(Command::Exit);
            }
        });

        ui.menu_button("Edit", |ui| {
            ui.add_enabled_ui(app.puzzle.has_undo(), |ui| {
                if ui.button("Undo").clicked() {
                    ui.close_menu();
                    app.event(Command::Undo);
                }
            });
            ui.add_enabled_ui(app.puzzle.has_redo(), |ui| {
                if ui.button("Redo").clicked() {
                    ui.close_menu();
                    app.event(Command::Redo);
                }
            });
            ui.separator();
            if ui.button("Reset").clicked() {
                ui.close_menu();
                app.event(Command::Redo);
            }
        });

        ui.menu_button("Puzzle", |ui| {
            for &puzzle_type in PuzzleType::ALL {
                if ui.button(puzzle_type.name()).clicked() {
                    ui.close_menu();
                    app.event(Command::NewPuzzle(puzzle_type));
                }
            }
        });

        ui.menu_button("Settings", |ui| {
            let win_states = &mut app.prefs.window_states;
            app.prefs.needs_save |= ui
                .checkbox(&mut win_states.graphics, "Preferences")
                .changed();
            ui.separator();
            app.prefs.needs_save |= ui
                .checkbox(&mut win_states.general_keybinds, "General keybinds")
                .changed();
            app.prefs.needs_save |= ui
                .checkbox(&mut win_states.puzzle_keybinds, "Puzzle keybinds")
                .changed();
        });

        ui.menu_button("Help", |ui| {
            let win_states = &mut app.prefs.window_states;
            app.prefs.needs_save |= ui.checkbox(&mut win_states.about, "About").changed();
        });
    });
}
