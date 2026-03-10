use crate::gui::App;

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    printlnd!(
        "Launched from {}",
        std::env::current_exe().unwrap().to_string_lossy()
    );
    printlnd!();

    let mut debug_info = std::mem::take(&mut *crate::debug::FRAME_DEBUG_INFO.lock());
    ui.add(
        egui::TextEdit::multiline(&mut debug_info)
            .code_editor()
            .desired_width(ui.available_width()),
    );
}
