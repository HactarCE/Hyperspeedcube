// TODO: this isn't just Lua

use hyperpuzzle_core::LogLine;
use log::Level;

use crate::app::App;
use crate::L;

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    let logger = hyperpuzzle::catalog().logger().clone();
    if ui.button(L.dev.logs.clear).clicked() {
        logger.clear();
    }

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
                for line in logger
                    .lines()
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

fn colored_log_line(ui: &mut egui::Ui, line: &LogLine) {
    let text = egui::RichText::new(&line.msg).monospace();
    let text = match ui.visuals().dark_mode {
        true => match line.level {
            Level::Error => text.color(egui::Color32::LIGHT_RED),
            Level::Warn => text.color(egui::Color32::GOLD),
            Level::Info | Level::Debug | Level::Trace => text.color(egui::Color32::LIGHT_BLUE),
        },
        false => match line.level {
            Level::Error => text
                .color(egui::Color32::DARK_RED)
                .background_color(egui::Color32::from_rgb(255, 223, 223)),
            Level::Warn => text.color(egui::Color32::DARK_RED),
            Level::Info | Level::Debug | Level::Trace => text.color(egui::Color32::BLUE),
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
