use crate::app::App;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    egui::menu::bar(ui, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            #[cfg(target_arch = "wasm32")]
            ui.hyperlink_to("Download the full version", env!("CARGO_PKG_HOMEPAGE"));

            egui::warn_if_debug_build(ui);
        });
    });
}
