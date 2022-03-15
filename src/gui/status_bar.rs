use strum::EnumMessage;

use super::util::ResponseExt;
use crate::app::App;
use crate::puzzle::PuzzleControllerTrait;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    ui.with_layout(egui::Layout::right_to_left(), |ui| {
        // Display twist count.
        let metric = &mut app.prefs.info.metric;
        let twist_count = app.puzzle.twist_count(*metric);
        let r = ui
            .add(
                egui::Label::new(format!("{}: {}", metric, twist_count))
                    .sense(egui::Sense::click()),
            )
            .on_hover_explanation(
                metric.get_message().unwrap_or(""),
                metric.get_detailed_message().unwrap_or(""),
            );
        {
            let mut data = ui.data();
            let last_frame_metric = data.get_temp_mut_or_default(unique_id!());
            if *last_frame_metric != Some(*metric) {
                // The selected value changed between this frame and the last, so
                // request another repaint to handle the tooltip size change.
                *last_frame_metric = Some(*metric);
                drop(data);
                ui.ctx().request_repaint();
            }
        }
        if r.clicked() {
            *metric = metric.next();
            app.prefs.needs_save = true;
        }
        ui.separator();

        // Display status message (left-aligned).
        ui.with_layout(egui::Layout::left_to_right(), |ui| {
            ui.label(app.status_msg());
        });
    });
}
