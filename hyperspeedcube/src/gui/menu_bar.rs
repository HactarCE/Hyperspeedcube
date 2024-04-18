use crate::app::App;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    egui::menu::bar(ui, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            #[cfg(target_arch = "wasm32")]
            ui.hyperlink_to("Download desktop app", env!("CARGO_PKG_HOMEPAGE"))
                .on_hover_text(
                    "The desktop version of Hyperspeedcube \
                     has the same features, but runs faster.",
                );

            egui::warn_if_debug_build(ui);

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if ui.available_width() < width_of_all_menu_buttons(ui) {
                    ui.menu_button("Menu", |ui| draw_menu_buttons(ui, app));
                } else {
                    draw_menu_buttons(ui, app);
                }
            })
        });
    });
}

const MENU_BUTTON_NAMES: &[&str] = &[
    "File", "Edit", "Scramble", "Puzzle", "Settings", "Tools", "Help",
];
fn draw_menu_buttons(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("File", |ui| {
        ui.label("yo");
    });
    ui.menu_button("Edit", |ui| {
        ui.label("yo");
    });
    ui.menu_button("Scramble", |ui| {
        ui.label("yo");
    });
    ui.menu_button("Puzzles", |ui| {
        if let Some(paths) = &*crate::PATHS {
            if ui.button("Show Lua directory").clicked() {
                crate::open_dir(&paths.lua_dir);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if ui.button("Extract built-in Lua files").clicked() {
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
    });
    ui.menu_button("Settings", |ui| {
        ui.label("yo");
    });
    ui.menu_button("Tools", |ui| {
        ui.label("yo");
    });
    ui.menu_button("Help", |ui| {
        ui.label("yo");
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
