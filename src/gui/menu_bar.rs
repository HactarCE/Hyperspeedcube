use super::components::puzzle_type_menu;
use super::ext::ResponseExt;
use super::windows;
use crate::app::App;
use crate::commands::Command;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            #[cfg(not(target_arch = "wasm32"))]
            if ui.button("Open...").clicked() {
                ui.close_menu();
                app.event(Command::Open);
            }
            if ui.button("Open from clipboard").clicked() {
                ui.close_menu();
                app.event(Command::PasteLog);
            }
            ui.separator();

            #[cfg(not(target_arch = "wasm32"))]
            {
                if ui.button("Save").clicked() {
                    ui.close_menu();
                    app.event(Command::Save);
                }
                if ui.button("Save As...").clicked() {
                    ui.close_menu();
                    app.event(Command::SaveAs);
                }
                ui.separator();
            }

            let r = ui.button("Copy (.hsc)").on_hover_explanation(
                "Hyperspeedcube log file (recommended)",
                "Includes extra metadata such as move count",
            );
            if r.clicked() {
                ui.close_menu();
                app.event(Command::CopyHscLog);
            }

            let r = ui.button("Copy (.log)").on_hover_explanation(
                "MC4D-compatible log file",
                "Backwards-compatible with Magic Cube 4D",
            );
            if r.clicked() {
                ui.close_menu();
                app.event(Command::CopyMc4dLog);
            }

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
            if ui.button("Full").clicked() {
                ui.close_menu();
                app.event(Command::ScrambleFull);
            }
        });

        ui.menu_button("Puzzle", |ui| {
            if let Some(ty) = puzzle_type_menu(ui) {
                app.event(Command::NewPuzzle(ty));
            }
        });

        ui.menu_button("Settings", |ui| {
            windows::APPEARANCE_SETTINGS.menu_button_toggle(ui);
            windows::INTERACTION_SETTINGS.menu_button_toggle(ui);
            windows::VIEW_SETTINGS.menu_button_toggle(ui);
            ui.separator();
            windows::GLOBAL_KEYBINDS.menu_button_toggle(ui);
            windows::PUZZLE_KEYBINDS.menu_button_toggle(ui);
            windows::MOUSEBINDS.menu_button_toggle(ui);
        });

        ui.menu_button("Tools", |ui| {
            windows::PIECE_FILTERS.menu_button_toggle(ui);
            windows::PUZZLE_CONTROLS.menu_button_toggle(ui);
            windows::KEYBIND_SETS.menu_button_toggle(ui);
            windows::MODIFIER_KEYS.menu_button_toggle(ui);
        });

        ui.menu_button("Help", |ui| {
            windows::KEYBINDS_REFERENCE.menu_button_toggle(ui);
            ui.separator();
            windows::WELCOME.menu_button_toggle(ui);
            windows::ABOUT.menu_button_toggle(ui);
            #[cfg(debug_assertions)]
            windows::DEBUG.menu_button_toggle(ui);
        });
    });
}
