macro_rules! unique_id {
    ($($args:tt)*) => {
        egui::Id::new((file!(), line!(), column!(), $($args)*))
    };
}

#[macro_use]
mod util;

mod key_combo_popup;
mod keybind_set_accessors;
mod menu_bar;
mod prefs;
mod puzzle_view;
mod side_bar;
mod status_bar;
mod widgets;
mod windows;

use crate::app::App;
pub(super) use key_combo_popup::{key_combo_popup_captures_event, key_combo_popup_handle_event};

pub fn build(ctx: &egui::Context, app: &mut App, puzzle_texture_id: egui::TextureId) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, app));

    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| status_bar::build(ui, app));

    for window in windows::ALL {
        if window.location != windows::Location::Floating {
            window.show(ctx, app);
        }
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(app.prefs.colors.background))
        .show(ctx, |ui| {
            for window in windows::ALL {
                if window.location == windows::Location::Floating {
                    window.show(ui.ctx(), app);
                }
            }
            puzzle_view::build(ui, app, puzzle_texture_id);
        });

    key_combo_popup::build(ctx, app);
}
