use super::components::puzzle_type_menu;
use super::ext::ResponseExt;
use super::windows;
use crate::app::App;
use crate::commands::Command;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            #[cfg(not(target_arch = "wasm32"))]
            command_button(ui, app, "Open...", Command::Open);
            command_button(ui, app, "Open from clipboard", Command::PasteLog);
            ui.separator();
            #[cfg(not(target_arch = "wasm32"))]
            {
                command_button(ui, app, "Save", Command::Save);
                command_button(ui, app, "Save as...", Command::SaveAs);
                ui.separator();
            }
            command_button_with_explanation(
                ui,
                app,
                "Copy (.hsc)",
                Command::CopyHscLog,
                "Hyperspeedcube log file (recommended)",
                "Includes extra metadata such as move count",
            );
            command_button_with_explanation(
                ui,
                app,
                "Copy (.log)",
                Command::CopyMc4dLog,
                "MC4D-compatible log file",
                "Backwards-compatible with Magic Cube 4D",
            );

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.separator();
                command_button(ui, app, "Exit", Command::Exit);
            }
        });

        ui.menu_button("Edit", |ui| {
            ui.add_enabled_ui(app.puzzle.has_undo(), |ui| {
                command_button(ui, app, "Undo twist", Command::Undo);
            });
            ui.add_enabled_ui(app.puzzle.has_redo(), |ui| {
                command_button(ui, app, "Redo twist", Command::Redo);
            });
            ui.separator();
            command_button(ui, app, "Reset puzzle", Command::Reset);
        });

        ui.menu_button("Scramble", |ui| {
            for n in 1..=8 {
                command_button(ui, app, &n.to_string(), Command::ScrambleN(n));
            }
            ui.separator();
            command_button(ui, app, "Full", Command::ScrambleFull);
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

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            #[cfg(target_arch = "wasm32")]
            ui.hyperlink_to("Download the full version", env!("CARGO_PKG_HOMEPAGE"));

            egui::warn_if_debug_build(ui);
        });
    });
}

fn command_button(ui: &mut egui::Ui, app: &mut App, text: &str, command: Command) {
    let mut button = egui::Button::new(text);
    let matching_keybind = app
        .prefs
        .global_keybinds
        .iter()
        .find(|keybind| keybind.command == command);
    if let Some(keybind) = matching_keybind {
        button = button.shortcut_text(keybind.key.to_string());
    }
    if ui.add(button).clicked() {
        ui.close_menu();
        app.event(command);
    }
}

fn command_button_with_explanation(
    ui: &mut egui::Ui,
    app: &mut App,
    text: &str,
    command: Command,
    strong_text: &str,
    detailed_message: &str,
) {
    let mut button = egui::Button::new(text);
    let matching_keybind = app
        .prefs
        .global_keybinds
        .iter()
        .find(|keybind| keybind.command == command);
    if let Some(keybind) = matching_keybind {
        button = button.shortcut_text(keybind.key.to_string());
    }
    let r = ui.add(button);
    if r.clicked() {
        ui.close_menu();
        app.event(command);
    }
    if !strong_text.is_empty() || !detailed_message.is_empty() {
        r.on_hover_explanation(strong_text, detailed_message);
    }
}
