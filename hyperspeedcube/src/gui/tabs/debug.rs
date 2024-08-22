use crate::{gui::App, L};

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    #[cfg(debug_assertions)]
    {
        let mut debug_info = std::mem::take(&mut *crate::debug::FRAME_DEBUG_INFO.lock().unwrap());
        ui.add(egui::TextEdit::multiline(&mut debug_info).code_editor());
    }
    #[cfg(not(debug_assertions))]
    ui.label(L.debug.disabled);
}
