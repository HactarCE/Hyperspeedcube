mod menu_bar;
mod side_bar;
mod status_bar;

use crate::app::App;

pub fn build(ctx: &egui::Context, app: &mut App) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, app));

    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| status_bar::build(ui, app));

    if app.prefs.window_states.graphics {
        egui::SidePanel::left("side_panel").show(ctx, |ui| side_bar::build(ui, app));
    }
}
