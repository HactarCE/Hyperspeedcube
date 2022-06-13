use crate::app::App;
use crate::commands::Command;
use crate::puzzle::{PuzzleType, PuzzleTypeTrait};

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            let can_save = app.puzzle.ty() == PuzzleType::Rubiks34;

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

        ui.menu_button("View", |ui| {
            ui.add_enabled_ui(app.puzzle.ty() == PuzzleType::Rubiks24, |ui| {
                if ui.button("Switch 2^4 view").clicked() {
                    ui.close_menu();
                    app.event(Command::SwitchView);
                }
            })
        });

        ui.menu_button("Scramble", |ui| {
            for n in 1..=8 {
                if ui.button(n.to_string()).clicked() {
                    ui.close_menu();
                    app.event(Command::ScrambleN(n));
                }
            }
            ui.separator();
            if ui.button("Full").clicked() {
                ui.close_menu();
                app.event(Command::ScrambleFull);
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

        ui.add(
            egui::DragValue::new(&mut app.puzzle.target_view_mode)
                .clamp_range(0.0..=1.0)
                .fixed_decimals(2)
                .speed(0.01),
        );
    });
}
