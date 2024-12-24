use hyperpuzzle::ScrambleType;
use hyperpuzzle_view::ReplayEvent;

use super::{AppUi, Tab};
use crate::L;

pub fn build(ui: &mut egui::Ui, app_ui: &mut AppUi) {
    egui::menu::bar(ui, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            const PROGRAM: &str = concat!("HSC v", env!("CARGO_PKG_VERSION"));
            let version_text = egui::RichText::new(PROGRAM).small();
            let version_button = egui::Button::new(version_text).frame(false);
            if ui.add(version_button).clicked() {
                app_ui.set_tab_state(&Tab::About, !app_ui.has_tab(&Tab::About));
            }

            #[cfg(target_arch = "wasm32")]
            ui.hyperlink_to(L.top_bar.desktop_link, env!("CARGO_PKG_HOMEPAGE"))
                .on_hover_text(L.top_bar.desktop_link_hover);
            #[cfg(not(target_arch = "wasm32"))]
            // Make rustc think that we've used these values.
            let _ = (L.top_bar.desktop_link, L.top_bar.desktop_link_hover);

            egui::warn_if_debug_build(ui);

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if ui.available_width() < width_of_all_menu_buttons(ui) {
                    ui.menu_button(L.menu.title, |ui| draw_menu_buttons(ui, app_ui));
                } else {
                    draw_menu_buttons(ui, app_ui);
                }
            })
        });
    });
}

fn width_of_all_menu_buttons(ui: &mut egui::Ui) -> f32 {
    [
        L.menu.file.title,
        L.menu.edit.title,
        L.menu.scramble.title,
        L.menu.settings.title,
        L.menu.tools.title,
        L.menu.puzzles.title,
        L.menu.help.title,
        #[cfg(debug_assertions)]
        L.menu.debug.title,
    ]
    .iter()
    .map(|text| menu_button_width(ui, text))
    .sum()
}
fn draw_menu_buttons(ui: &mut egui::Ui, app_ui: &mut AppUi) {
    fn show_tab_toggle(ui: &mut egui::Ui, app_ui: &mut AppUi, tab: Tab) {
        let mut open = app_ui.has_tab(&tab);
        if ui.checkbox(&mut open, tab.menu_name()).clicked() {
            app_ui.set_tab_state(&tab, open);
        }
    }

    ui.menu_button(L.menu.file.title, |ui| {
        if ui.button(L.menu.file.open).clicked()
            && app_ui.confirm_discard(L.confirm_discard.open_another_file)
        {
            ui.close_menu();
            app_ui.app.open_file();
        }
        ui.separator();
        if ui.button(L.menu.file.save).clicked() {
            ui.close_menu();
            app_ui.app.save_file();
        }
        if ui.button(L.menu.file.save_as).clicked() {
            ui.close_menu();
            app_ui.app.save_file_as();
        }
        ui.separator();
        if ui.button(L.menu.file.copy_hsc).clicked() {
            ui.close_menu();
            if let Some(copy_text) = app_ui.app.serialize_puzzle_log() {
                ui.ctx().copy_text(copy_text);
            }
        }
        // let _ = ui.button(L.menu.file.copy_log);
        ui.separator();
        if ui.button(L.menu.file.exit).clicked() && app_ui.confirm_discard(L.confirm_discard.exit) {
            ui.close_menu();
            todo!("exit, but not on web");
        }
    });
    ui.menu_button(L.menu.edit.title, |ui| {
        let undo_button = egui::Button::new(L.menu.edit.undo_twist);
        if ui.add_enabled(app_ui.app.has_undo(), undo_button).clicked() {
            app_ui.app.undo();
        }
        let redo_button = egui::Button::new(L.menu.edit.redo_twist);
        if ui.add_enabled(app_ui.app.has_redo(), redo_button).clicked() {
            app_ui.app.redo();
        }
        ui.separator();
        if ui.button(L.menu.edit.reset_puzzle).clicked()
            && app_ui.confirm_discard(L.confirm_discard.reset_puzzle)
        {
            app_ui.app.reset_puzzle();
            ui.close_menu();
        }
    });
    ui.menu_button(L.menu.scramble.title, |ui| {
        let can_scramble = app_ui
            .app
            .active_puzzle_view
            .with(|p| p.puzzle().can_scramble())
            .unwrap_or(false);
        let full_scramble_button = egui::Button::new(L.menu.scramble.full);
        if ui.add_enabled(can_scramble, full_scramble_button).clicked()
            && app_ui.confirm_discard(&L.confirm_discard.scramble)
        {
            app_ui.app.scramble(ScrambleType::Full);
        }
        ui.separator();
        let scramble_1_button = egui::Button::new(L.menu.scramble.one);
        if ui.add_enabled(can_scramble, scramble_1_button).clicked()
            && app_ui.confirm_discard(&L.confirm_discard.scramble)
        {
            app_ui.app.scramble(ScrambleType::Partial(1));
        }
        let scramble_2_button = egui::Button::new(L.menu.scramble.two);
        if ui.add_enabled(can_scramble, scramble_2_button).clicked()
            && app_ui.confirm_discard(&L.confirm_discard.scramble)
        {
            app_ui.app.scramble(ScrambleType::Partial(2));
        }
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::Scrambler);
    });
    ui.menu_button(L.menu.settings.title, |ui| {
        show_tab_toggle(ui, app_ui, Tab::Colors);
        show_tab_toggle(ui, app_ui, Tab::Styles);
        show_tab_toggle(ui, app_ui, Tab::View);
        show_tab_toggle(ui, app_ui, Tab::Animations);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::Interaction);
        show_tab_toggle(ui, app_ui, Tab::Keybinds);
        show_tab_toggle(ui, app_ui, Tab::Mousebinds);
        ui.separator();
        // TODO: add "auto" mode that follows OS
        egui::global_theme_preference_buttons(ui);
    });
    ui.menu_button(L.menu.tools.title, |ui| {
        show_tab_toggle(ui, app_ui, Tab::Camera);
        show_tab_toggle(ui, app_ui, Tab::PieceFilters);
        show_tab_toggle(ui, app_ui, Tab::Timer);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::Macros);
        show_tab_toggle(ui, app_ui, Tab::MoveInput);
        show_tab_toggle(ui, app_ui, Tab::Timeline);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::PuzzleControls);
        show_tab_toggle(ui, app_ui, Tab::ModifierKeys);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::Scrambler);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::ImageGenerator);
    });
    ui.menu_button(L.menu.puzzles.title, |ui| {
        show_tab_toggle(ui, app_ui, Tab::PuzzleLibrary);
        show_tab_toggle(ui, app_ui, Tab::PuzzleInfo);

        ui.separator();

        let r = ui.checkbox(
            &mut app_ui.app.prefs.show_experimental_puzzles,
            L.menu.puzzles.show_experimental,
        );
        app_ui.app.prefs.needs_save |= r.changed();

        ui.menu_button(L.menu.puzzles.custom, |ui| {
            if let Ok(lua_dir) = hyperpaths::lua_dir() {
                if ui.button(L.menu.puzzles.show_lua_dir).clicked() {
                    ui.close_menu();
                    crate::open_dir(lua_dir);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            if ui.button(L.menu.puzzles.show_lua_dir).clicked() {
                ui.close_menu();
                if let Some(mut dir_path) = rfd::FileDialog::new()
                    .set_title(L.menu.puzzles.extract_lua)
                    .pick_folder()
                {
                    dir_path.push("lua");
                    match hyperpuzzle_library::LUA_BUILTIN_DIR.extract(&dir_path) {
                        Ok(()) => crate::open_dir(&dir_path),
                        Err(e) => log::error!("Error extracting built-in Lua files: {e}"),
                    }
                }
            }

            show_tab_toggle(ui, app_ui, Tab::LuaLogs);
            show_tab_toggle(ui, app_ui, Tab::DevTools);
        });
    });
    ui.menu_button(L.menu.help.title, |ui| {
        ui.heading(L.menu.help.guides);
        let _ = ui.button("Welcome");
        show_tab_toggle(ui, app_ui, Tab::About);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::KeybindsReference);
    });
    #[cfg(debug_assertions)]
    ui.menu_button(L.menu.debug.title, |ui| {
        show_tab_toggle(ui, app_ui, Tab::Debug);
    });
}

fn menu_button_width(ui: &egui::Ui, text: &str) -> f32 {
    super::util::text_width(ui, text)
        + ui.spacing().button_padding.x * 2.0
        + ui.spacing().item_spacing.x
}
