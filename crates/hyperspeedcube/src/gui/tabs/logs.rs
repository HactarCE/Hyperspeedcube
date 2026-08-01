use std::collections::HashSet;
use std::hash::Hash;

use hyperpuzzle::{LogLine, Logger};
use log::Level;

use crate::L;
use crate::app::App;
use crate::gui::EguiValue;

#[derive(Debug, Clone)]
struct LogViewState {
    last_logger: Logger,
    open_indices: HashSet<usize>,
}

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    let logger = &hyperpuzzle::catalog().logger;

    let mut state = EguiValue::load_or(ui, unique_id!(), || LogViewState {
        last_logger: logger.clone(),
        open_indices: HashSet::new(),
    });

    if *logger != state.last_logger {
        state.last_logger = logger.clone();
        state.open_indices.clear();
    }

    if ui.button(L.dev.logs.clear).clicked() {
        logger.clear();
        state.open_indices.clear();
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
            ui.with_layout(ui.layout().with_main_wrap(false), |ui| {
                for (i, line) in logger
                    .lines()
                    .iter()
                    .filter(|line| line.matches_filter_string(&filter_string))
                    .enumerate()
                {
                    let mut is_open = state.open_indices.remove(&i);
                    colored_log_line(ui, line, i, &mut is_open);
                    ui.add_space(2.0);
                    if is_open {
                        state.open_indices.insert(i);
                    }
                }
            });
        });
}

fn colored_log_line(ui: &mut egui::Ui, line: &LogLine, i: usize, is_open: &mut bool) {
    let mut layout_job = egui::text::LayoutJob::default();
    layout_job.wrap.max_width = ui.available_width();
    let mono_font_id = egui::TextStyle::Monospace.resolve(ui.style());

    if let Some(filename) = &line.filename {
        let format = egui::TextFormat::simple(mono_font_id.clone(), ui.visuals().text_color());
        layout_job.append(filename, 0.0, format.clone());
        layout_job.append(": ", 0.0, format);
    }

    let (fg, bg) = match (ui.visuals().dark_mode, line.level) {
        (true, Level::Error) => (egui::Color32::LIGHT_RED, None),
        (true, Level::Warn) => (egui::Color32::GOLD, None),
        (true, Level::Info | Level::Debug | Level::Trace) => (egui::Color32::LIGHT_BLUE, None),
        (false, Level::Error) => (
            egui::Color32::DARK_RED,
            Some(egui::Color32::from_rgb(255, 223, 223)),
        ),
        (false, Level::Warn) => (egui::Color32::DARK_RED, None),
        (false, Level::Info | Level::Debug | Level::Trace) => (egui::Color32::BLUE, None),
    };
    let mut format = egui::TextFormat::simple(mono_font_id, fg);
    if let Some(bg) = bg {
        format.background = bg;
    }
    layout_job.append(&line.msg, 0.0, format);

    if let Some(full) = &line.full {
        let r = ui.add(egui::Button::new(layout_job).fill(egui::Color32::TRANSPARENT));
        *is_open ^= r.clicked();
        if (r.hovered() || r.is_pointer_button_down_on()) && !*is_open {
            egui::Area::new(unique_id!())
                .anchor(
                    egui::Align2::LEFT_TOP,
                    r.rect.left_bottom().to_vec2() + egui::vec2(0.0, 3.0),
                )
                .constrain(false)
                .interactable(false)
                .fade_in(false)
                .show(ui, |ui| {
                    egui::Frame::new()
                        .fill(ui.visuals().extreme_bg_color)
                        .stroke(ui.visuals().window_stroke)
                        .inner_margin(8.0)
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            crate::gui::components::show_ariadne_error_in_egui(ui, full);
                        });
                });
        }
        if *is_open {
            egui::ScrollArea::horizontal().id_salt(i).show(ui, |ui| {
                egui::Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .stroke(ui.visuals().window_stroke)
                    .inner_margin(8.0)
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        crate::gui::components::show_ariadne_error_in_egui(ui, full);
                    });
            });
        };
    } else {
        ui.label(layout_job);
    }
}
