use itertools::Itertools;
use strum::{EnumMessage, IntoEnumIterator};

use super::util::ResponseExt;
use crate::app::App;
use crate::puzzle::TwistMetric;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    ui.with_layout(egui::Layout::right_to_left(), |ui| {
        // BLD toggle
        let bld = &mut app.prefs.colors.blindfold;
        let r = ui
            .selectable_label(*bld, "BLD")
            .on_hover_explanation("Blindfold mode", "Hides sticker colors");
        if r.clicked() {
            *bld ^= true;
            changed = true;
            app.request_redraw_puzzle();
        }
        ui.separator();

        // Twist count
        let metric = &mut app.prefs.info.metric;
        let twist_count = app.puzzle.twist_count(*metric);
        let mut r = ui.add(
            egui::Label::new(format!("{}: {}", metric, twist_count)).sense(egui::Sense::click()),
        );
        if ui.input().modifiers.shift {
            r = r.on_hover_explanation(
                "Twist count",
                &TwistMetric::iter()
                    .map(|m| format!("{m}: {}", app.puzzle.twist_count(m)))
                    .join("\n"),
            );
        } else {
            r = r.on_hover_explanation(
                metric.get_message().unwrap_or(""),
                metric.get_detailed_message().unwrap_or(""),
            );
        }
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
        if r.clicked_by(egui::PointerButton::Primary) {
            *metric = enum_iterator::next_cycle(metric).unwrap();
            changed = true;
        }
        if r.clicked_by(egui::PointerButton::Secondary) {
            *metric = enum_iterator::previous_cycle(metric).unwrap();
            changed = true;
        }
        ui.separator();

        // Status message (left-aligned)
        ui.with_layout(egui::Layout::left_to_right(), |ui| {
            ui.label(app.status_msg());
        });
    });

    app.prefs.needs_save |= changed;
}
