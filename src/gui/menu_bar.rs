use crate::app::App;
use crate::commands::Command;
use crate::puzzle::{rubiks_3d, PuzzleType, PuzzleTypeEnum, Rubiks3D};

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            // let can_save = app.puzzle.ty() == PuzzleTypeEnum::Rubiks4D{};
            let can_save = false; // TODO

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
                if ui.button("Undo twist").clicked() {
                    ui.close_menu();
                    app.event(Command::Undo);
                }
            });
            ui.add_enabled_ui(app.puzzle.has_redo(), |ui| {
                if ui.button("Redo twist").clicked() {
                    ui.close_menu();
                    app.event(Command::Redo);
                }
            });
            ui.separator();
            if ui.button("Reset puzzle").clicked() {
                ui.close_menu();
                app.event(Command::Reset);
            }
        });

        ui.menu_button("Scramble", |ui| {
            for n in 1..=8 {
                if ui.button(n.to_string()).clicked() {
                    ui.close_menu();
                    app.event(Command::ScrambleN(n));
                }
            }
            ui.separator();
            if ui.button(format!("Full")).clicked() {
                ui.close_menu();
                app.event(Command::ScrambleFull);
            }
        });

        ui.menu_button("Puzzle", |ui| {
            if let Some(ty) = super::util::puzzle_select_menu(ui) {
                app.event(Command::NewPuzzle(ty));
            }
        });

        ui.menu_button("Settings", |ui| {
            if ui.button("Preferences...").clicked() {
                ui.close_menu();
                super::Window::PrefsPanel.toggle(ui.ctx());
            }
            ui.separator();
            if ui.button("General keybinds...").clicked() {
                ui.close_menu();
                super::Window::GeneralKeybinds.toggle(ui.ctx());
            }
            if ui.button("Puzzle keybinds...").clicked() {
                ui.close_menu();
                super::Window::PuzzleKeybinds.toggle(ui.ctx());
            }
        });

        ui.menu_button("Tools", |ui| {
            if ui.button("Puzzle controls...").clicked() {
                ui.close_menu();
                super::Window::PuzzleControlsPanel.toggle(ui.ctx());
            }
        });

        ui.menu_button("Help", |ui| {
            if ui.button("Keybinds reference...").clicked() {
                ui.close_menu();
                super::Window::KeybindsReference.toggle(ui.ctx());
            }

            ui.separator();

            if ui.button("About").clicked() {
                ui.close_menu();
                super::Window::About.toggle(ui.ctx());
            }

            #[cfg(debug_assertions)]
            if ui.button("Debug").clicked() {
                ui.close_menu();
                super::Window::Debug.toggle(ui.ctx());
            }
        });
    });
}
