use super::Window;
use crate::app::App;

pub(crate) const MODIFIER_KEYS: Window = Window {
    name: "Modifier keys",
    fixed_width: Some(0.0),
    build,
    ..Window::DEFAULT
};

fn build(ui: &mut egui::Ui, app: &mut App) {
    // ui.style_mut().wrap = Some(false);
    // let r = ui.checkbox(&mut app.prefs.info.modifier_toggles, "Show in status bar");
    // app.prefs.needs_save |= r.changed();

    ui.horizontal(|ui| {
        ui.spacing_mut().interact_size.y *= 2.0;
        crate::gui::status_bar::modifier_toggles(ui, app, true);
    });
}
