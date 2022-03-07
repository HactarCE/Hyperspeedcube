use strum::EnumMessage;

use crate::app::App;
use crate::puzzle::PuzzleControllerTrait;

const TWIST_METRIC_TOOLTIP_WIDTH: f32 = 200.0;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    ui.with_layout(egui::Layout::right_to_left(), |ui| {
        // Display twist count.
        let metric = &mut app.prefs.info.metric;
        let twist_count = app.puzzle.twist_count(*metric);
        let r = ui.add(
            egui::Label::new(format!("{}: {}", metric, twist_count)).sense(egui::Sense::click()),
        );
        if r.clicked() {
            *metric = metric.next();
            app.prefs.needs_save = true;
        }
        r.on_hover_ui(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(TWIST_METRIC_TOOLTIP_WIDTH, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label(egui::RichText::new(metric.get_message().unwrap_or("")).strong());
                    ui.label(metric.get_detailed_message().unwrap_or(""));
                },
            );
        });
        ui.separator();

        // Display status message (left-aligned).
        ui.with_layout(egui::Layout::left_to_right(), |ui| {
            ui.label(app.status_msg());
        });
    });
}
