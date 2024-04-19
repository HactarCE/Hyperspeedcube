use crate::gui::App;

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    #[cfg(debug_assertions)]
    {
        let mut debug_info = std::mem::take(&mut *crate::debug::FRAME_DEBUG_INFO.lock().unwrap());
        ui.add(egui::TextEdit::multiline(&mut debug_info).code_editor());
    }
    #[cfg(not(debug_assertions))]
    ui.label("Debug info are disabled in release builds");
}
