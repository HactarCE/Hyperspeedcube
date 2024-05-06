use crate::app::App;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.add_enabled_ui(app.has_active_puzzle(), |ui| {
        if ui.button("Reset camera").clicked() {
            app.with_active_puzzle_view(|puzzle_view| puzzle_view.view.reset_camera());
        }
        ui.label("Hold shift to rotate through W axis");
        ui.label("Hold alt to rotate through W axis");
    });
}
