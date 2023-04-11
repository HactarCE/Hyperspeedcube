use super::{Window, ABOUT_WINDOW_WIDTH};
use crate::app::App;

pub(crate) const ABOUT: Window = Window {
    name: "About",
    fixed_width: Some(ABOUT_WINDOW_WIDTH),
    build,
    ..Window::DEFAULT
};

fn build(ui: &mut egui::Ui, _app: &mut App) {
    ui.vertical_centered(|ui| {
        ui.strong(format!("{} v{}", crate::TITLE, env!("CARGO_PKG_VERSION")));
        ui.label(env!("CARGO_PKG_DESCRIPTION"));
        ui.hyperlink(env!("CARGO_PKG_REPOSITORY"));
        ui.label("");
        ui.label(
            format!("Created by {}", env!("CARGO_PKG_AUTHORS"))
        );
        ui.hyperlink("https://ajfarkas.dev/");
        ui.label("");
        ui.label(format!("Licensed under {}", env!("CARGO_PKG_LICENSE")));
    });
}
