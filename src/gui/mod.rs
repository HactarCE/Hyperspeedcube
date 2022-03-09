mod keybinds_window;
mod menu_bar;
mod side_bar;
mod status_bar;
mod util;

use crate::app::App;
use keybinds_window::KeybindsWindow;

pub fn build(ctx: &egui::Context, app: &mut App) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, app));

    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| status_bar::build(ui, app));

    if app.prefs.window_states.graphics {
        egui::SidePanel::left("side_panel").show(ctx, |ui| side_bar::build(ui, app));
    }

    if app.prefs.window_states.general_keybinds {
        egui::Window::new("Keybinds").show(ctx, |ui| {
            if ui
                .add(KeybindsWindow {
                    keybinds: &mut app.prefs.general_keybinds,
                })
                .changed()
            {
                app.prefs.needs_save = true;
            }
        });
    }
}
