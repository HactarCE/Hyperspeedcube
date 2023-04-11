macro_rules! unique_id {
    ($($args:tt)*) => {
        egui::Id::new((file!(), line!(), column!(), $($args)*))
    };
}

use crate::app::App;

pub fn build(ctx: &egui::Context, app: &mut App, puzzle_texture_id: egui::TextureId) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        ui.label("todo");
    });

    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        ui.label("todo");
    });

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(app.prefs.colors.background))
        .show(ctx, |ui| {
            // for window in windows::ALL {
            //     if window.location == windows::Location::Floating {
            //         window.show(ui.ctx(), app);
            //     }
            // }
            // puzzle_view::build(ui, app, puzzle_texture_id);
        });

    // key_combo_popup::build(ctx, app);
}
