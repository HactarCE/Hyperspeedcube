use hyperpuzzle::lua::LuaLogLevel;
use hyperpuzzle::LuaLogLine;

use crate::app::App;
use crate::L;

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    let mut log_lines = hyperpuzzle_library::LIBRARY_LOG_LINES.lock();
    if ui.button(L.dev.logs.clear).clicked() {
        log_lines.clear();
    }

    hyperpuzzle_library::LIBRARY.with(|lib| log_lines.extend(lib.pending_log_lines()));

    let filter_string_id = unique_id!();
    let mut filter_string: String =
        ui.data_mut(|data| data.get_temp(filter_string_id).clone().unwrap_or_default());
    ui.horizontal(|ui| {
        ui.label(L.dev.logs.filter);
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
                    .filter(|line| line.matches_filter_string(&filter_string))
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
    let text = egui::RichText::new(&line.msg).monospace();
    let text = match ui.visuals().dark_mode {
        true => match line.level {
            LuaLogLevel::Info => text.color(egui::Color32::LIGHT_BLUE),
            LuaLogLevel::Warn => text.color(egui::Color32::GOLD),
            LuaLogLevel::Error => text.color(egui::Color32::LIGHT_RED),
        },
        false => match line.level {
            LuaLogLevel::Info => text.color(egui::Color32::BLUE),
            LuaLogLevel::Warn => text.color(egui::Color32::DARK_RED),
            LuaLogLevel::Error => text
                .color(egui::Color32::DARK_RED)
                .background_color(egui::Color32::from_rgb(255, 223, 223)),
        },
    };
    let label = egui::Label::new(text);
    if let Some(traceback) = &line.traceback {
        ui.add(label.sense(egui::Sense::hover()))
            .on_hover_text(egui::RichText::new(traceback).monospace());
    } else {
        ui.add(label);
    }
}
