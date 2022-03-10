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
                app.event(Command::Reset);
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
            if ui.button("Preferences").clicked() {
                ui.close_menu();
                super::Window::SidePanel.toggle(ui.ctx());
            }
            ui.separator();
            if ui.button("General keybinds").clicked() {
                ui.close_menu();
                super::Window::GeneralKeybinds.toggle(ui.ctx());
            }
            if ui.button("Puzzle keybinds").clicked() {
                ui.close_menu();
                super::Window::PuzzleKeybinds.toggle(ui.ctx());
            }
        });

        ui.menu_button("Help", |ui| {
            if ui.button("About").clicked() {
                ui.close_menu();
                super::Window::About.toggle(ui.ctx());
            }
        });
    });
}
