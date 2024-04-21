use super::{AppUi, Tab};

pub fn build(ui: &mut egui::Ui, app_ui: &mut AppUi) {
    egui::menu::bar(ui, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            const PROGRAM: &str = concat!("HSC v", env!("CARGO_PKG_VERSION"));
            let version_text = egui::RichText::new(PROGRAM).small();
            let version_button = egui::Button::new(version_text).frame(false);
            if ui.add(version_button).clicked() {
                // TODO: open "about" window
            }

            #[cfg(target_arch = "wasm32")]
            ui.hyperlink_to("Download desktop app", env!("CARGO_PKG_HOMEPAGE"))
                .on_hover_text(
                    "The desktop version of Hyperspeedcube \
                     has the same features, but runs faster.",
                );

            egui::warn_if_debug_build(ui);

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if ui.available_width() < width_of_all_menu_buttons(ui) {
                    ui.menu_button("Menu", |ui| draw_menu_buttons(ui, app_ui));
                } else {
                    draw_menu_buttons(ui, app_ui);
                }
            })
        });
    });
}

const MENU_BUTTON_NAMES: &[&str] = &[
    "File",
    "Edit",
    "Scramble",
    "Settings",
    "Tools",
    "Puzzles",
    "Help",
    #[cfg(debug_assertions)]
    "Debug",
];
fn draw_menu_buttons(ui: &mut egui::Ui, app_ui: &mut AppUi) {
    fn show_tab_toggle(ui: &mut egui::Ui, app_ui: &mut AppUi, tab: Tab) {
        let mut open = app_ui.has_tab(&tab);
        if ui.checkbox(&mut open, tab.menu_name()).clicked() {
            match open {
                true => app_ui.open_tab(&tab),
                false => app_ui.close_tab(&tab),
            }
        }
    }

    ui.menu_button("File", |ui| {
        let _ = ui.button("Open...");
        let _ = ui.button("Open from clipboard");
        ui.separator();
        let _ = ui.button("Save");
        let _ = ui.button("Save as...");
        ui.separator();
        let _ = ui.button("Copy (.hsc)");
        let _ = ui.button("Copy (.log)");
        ui.separator();
        let _ = ui.button("Exit");
    });
    ui.menu_button("Edit", |ui| {
        let _ = ui.button("Undo twist");
        let _ = ui.button("Redo twist");
        ui.separator();
        let _ = ui.button("Reset puzzle");
    });
    ui.menu_button("Scramble", |ui| {
        let _ = ui.button("Full");
        ui.separator();
        let _ = ui.button("1");
        let _ = ui.button("2");
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::Scrambler);
    });
    ui.menu_button("Settings", |ui| {
        show_tab_toggle(ui, app_ui, Tab::Colors);
        show_tab_toggle(ui, app_ui, Tab::Styles);
        show_tab_toggle(ui, app_ui, Tab::View);
        show_tab_toggle(ui, app_ui, Tab::Animations);
        show_tab_toggle(ui, app_ui, Tab::Interaction);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::Keybinds);
        show_tab_toggle(ui, app_ui, Tab::Mousebinds);
        ui.separator();
        // TODO: add "auto" mode that follows OS
        egui::global_dark_light_mode_buttons(ui);
    });
    ui.menu_button("Tools", |ui| {
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
    });
    ui.menu_button("Puzzles", |ui| {
        show_tab_toggle(ui, app_ui, Tab::PuzzleLibrary);
        show_tab_toggle(ui, app_ui, Tab::PuzzleInfo);

        ui.separator();

        if let Some(paths) = &*crate::PATHS {
            if ui.button("Show Lua directory").clicked() {
                crate::open_dir(&paths.lua_dir);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if ui.button("Extract built-in Lua files...").clicked() {
            if let Some(mut dir_path) = rfd::FileDialog::new()
                .set_title("Extract built-in Lua files")
                .pick_folder()
            {
                dir_path.push("lua");
                match crate::LUA_BUILTIN_DIR.extract(&dir_path) {
                    Ok(()) => crate::open_dir(&dir_path),
                    Err(e) => log::error!("Error extracting built-in Lua files: {e}"),
                }
            }
        }

        show_tab_toggle(ui, app_ui, Tab::LuaLogs);
    });
    ui.menu_button("Help", |ui| {
        ui.heading("Guides");
        let _ = ui.button("Welcome");
        let _ = ui.button("About");
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::KeybindsReference);
    });
    #[cfg(debug_assertions)]
    ui.menu_button("Debug", |ui| {
        show_tab_toggle(ui, app_ui, Tab::Debug);
    });
}

fn width_of_all_menu_buttons(ui: &mut egui::Ui) -> f32 {
    MENU_BUTTON_NAMES
        .iter()
        .map(|text| menu_button_size(ui, text))
        .sum()
}

fn menu_button_size(ui: &mut egui::Ui, text: &str) -> f32 {
    let wrap = None;
    let max_width = f32::INFINITY;
    let text_size = egui::WidgetText::from(text)
        .into_galley(ui, wrap, max_width, egui::TextStyle::Button)
        .size();
    text_size.x + ui.spacing().button_padding.x * 2.0 + ui.spacing().item_spacing.x
}
