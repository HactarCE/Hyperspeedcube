use crate::{app::App, L};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.add_enabled_ui(app.active_puzzle_view.has_puzzle(), |ui| {
        if ui.button(L.camera.reset).clicked() {
            app.active_puzzle_view.with(|p| p.view.reset_camera());
        }
        // TODO: customizable mousebinds
        ui.label(L.camera.w_axis_hint.with("shift"));
        ui.label(L.camera.v_axis_hint.with("alt"));
    });
}
