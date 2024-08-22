use crate::{app::App, L};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.add_enabled_ui(app.has_active_puzzle_view(), |ui| {
        if ui.button(L.camera.reset).clicked() {
            app.with_active_puzzle_view(|puzzle_view| puzzle_view.view.reset_camera());
        }
        // TODO: customizable mousebinds
        ui.label(L.camera.w_axis_hint.with("shift"));
        ui.label(L.camera.v_axis_hint.with("alt"));
    });
}
