use crate::app::App;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    ui.collapsing("Colors", |ui| {
        crate::gui::prefs::build_colors_section(ui, app);
    });
    ui.collapsing("Outlines", |ui| {
        crate::gui::prefs::build_outlines_section(ui, app);
    });
    ui.collapsing("Opacity", |ui| {
        crate::gui::prefs::build_opacity_section(ui, app);
    });
    ui.collapsing("Performance", |ui| {
        crate::gui::prefs::build_graphics_section(ui, app);
    });
}
