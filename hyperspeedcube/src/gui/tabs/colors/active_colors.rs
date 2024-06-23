use crate::app::App;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.set_enabled(app.has_active_puzzle());

    ui.group(|ui| {
        ui.collapsing("Puzzle colors", |ui| {
            if let Some(ty) = app.active_puzzle_type() {
                for (id, face_color) in &ty.colors {
                    ui.horizontal(|ui| {
                        ui.label(&face_color.name);
                        if let Some(default) = &face_color.default_color {
                            ui.label(default);
                        }
                    });
                }
            } else {
                ui.label("No puzzle loaded");
            }
        });
    });

    let mut changed = false;

    ui.group(|ui| {});
}
