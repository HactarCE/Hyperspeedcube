use crate::app::App;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    crate::gui::prefs::build_interaction_section(ui, app);
}
