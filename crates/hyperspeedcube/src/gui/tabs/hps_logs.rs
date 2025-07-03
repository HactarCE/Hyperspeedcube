// TODO: this isn't just HPS

use hyperpuzzle::LogLine;
use log::Level;

use crate::L;
use crate::app::App;

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    let logger = hyperpuzzle::catalog().default_logger().clone();
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
                for line in logger
                    .lines()
                    .iter()
                    .filter(|line| line.matches_filter_string(&filter_string))
                {
                    colored_log_line(ui, line);
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

    let r = ui.label(text);

    if let Some(full) = &line.full
        && r.hovered()
    {
        r.show_tooltip_ui(|ui| {
            ui.set_max_width(ui.ctx().screen_rect().width());
            crate::gui::components::show_ariadne_error_in_egui(ui, full);
        });
    }
}
