use crate::app::App;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.add_enabled_ui(app.active_puzzle.has_puzzle(), |ui| {
        ui.label("Not yet implemented");
    });
}
