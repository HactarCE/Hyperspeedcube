use crate::gui::App;

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    if let Ok(exe_path) = std::env::current_exe() {
        printlnd!("Launched from {}", exe_path.to_string_lossy());
    } else {
        printlnd!("error in current_exe()");
    }
    printlnd!();

    let mut debug_info = std::mem::take(&mut *crate::debug::FRAME_DEBUG_INFO.lock());
    ui.add(
        egui::TextEdit::multiline(&mut debug_info)
            .code_editor()
            .desired_width(ui.available_width()),
    );
}
