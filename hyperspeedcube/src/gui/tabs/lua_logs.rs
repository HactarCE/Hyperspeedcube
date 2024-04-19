use hyperpuzzle::lua::LuaLogLevel;
use hyperpuzzle::LuaLogLine;

use crate::app::App;

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    let mut log_lines = crate::LIBRARY_LOG_LINES.lock();
    if ui.button("Clear logs").clicked() {
        log_lines.clear();
    }

    crate::LIBRARY.with(|lib| log_lines.extend(lib.pending_log_lines()));

    let filter_string_id = unique_id!();
    let mut filter_string: String =
        ui.data_mut(|data| data.get_temp(filter_string_id).clone().unwrap_or_default());
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(&mut filter_string);
    });
    ui.data_mut(|data| data.insert_temp(filter_string_id, filter_string.clone()));

    egui::ScrollArea::new([true; 2])
        .auto_shrink(false)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            // no wrap
            ui.with_layout(ui.layout().with_main_wrap(false), |ui| {
                let mut is_first = true;
                let mut last_file = &None;
                for line in log_lines
                    .iter()
                    .filter(|line| line.matches_filter_string(&*filter_string))
                {
                    if is_first {
                        is_first = false;
                    } else if last_file != &line.file {
                        ui.separator();
                    }

                    if last_file != &line.file {
                        if let Some(f) = &line.file {
                            ui.label(
                                egui::RichText::new(f)
                                    .strong()
                                    .text_style(egui::TextStyle::Monospace),
                            );
                        }
                    }

                    colored_log_line(ui, line);

                    last_file = &line.file;
                }
            });
        });
}

fn colored_log_line(ui: &mut egui::Ui, line: &LuaLogLine) {
    let color = match line.level {
        LuaLogLevel::Info => egui::Color32::LIGHT_BLUE,
        LuaLogLevel::Warn => egui::Color32::GOLD,
        LuaLogLevel::Error => egui::Color32::LIGHT_RED,
    };
    // let s = match &line.file {
    //     Some(file) => format!("[{}] {}", file, line.msg),
    //     None => format!("{}", line.msg),
    // };
    ui.label(
        egui::RichText::new(&line.msg)
            .color(color)
            .text_style(egui::TextStyle::Monospace),
    );
}
