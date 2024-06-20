use crate::app::App;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.add_enabled_ui(app.has_active_puzzle_view(), |ui| {
        ui.label("Not yet implemented");
    });
}
