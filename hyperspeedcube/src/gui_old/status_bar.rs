use key_names::KeyMappingCode;
use strum::EnumMessage;
use winit::event::VirtualKeyCode;

use super::ext::*;
use crate::app::App;
use crate::commands::Command;
use crate::preferences::Key;
use crate::puzzle::TwistMetric;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        // Right-aligned segments
        bld_toggle(ui, app);
        ui.separator();

        twist_count(ui, app);
        ui.separator();

        // Left-aligned segments
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            if app.prefs.info.modifier_toggles {
                modifier_toggles(ui, app, false);
                ui.separator();
            }

            ui.label(app.status_msg());
        });
    });
}

pub(super) fn modifier_toggles(ui: &mut egui::Ui, app: &mut App, big: bool) {
    for ch in key_names::MODIFIERS_ORDER.chars() {
        let (name, sc, vk) = match ch {
            'c' => (
                key_names::CTRL_STR,
                KeyMappingCode::ControlLeft,
                VirtualKeyCode::LControl,
            ),
            's' => (
                key_names::SHIFT_STR,
                KeyMappingCode::ShiftLeft,
                VirtualKeyCode::LShift,
            ),
            'a' => (
                key_names::ALT_STR,
                KeyMappingCode::AltLeft,
                VirtualKeyCode::LAlt,
            ),
            'm' => (
                key_names::LOGO_STR,
                KeyMappingCode::MetaLeft,
                VirtualKeyCode::LWin,
            ),
            _ => continue, // unreachable
        };
        let label = egui::SelectableLabel::new(
            app.pressed_modifiers().contains(Key::Vk(vk).modifier_bit()),
            name,
        );
        let r = if big {
            ui.add_sized(ui.spacing().interact_size, label)
        } else {
            ui.add(label)
        };
        if r.clicked() {
            app.toggle_key(Some(sc), Some(vk));
        }
    }
}

fn bld_toggle(ui: &mut egui::Ui, app: &mut App) {
    let bld = &mut app.prefs.colors.blindfold;
    let r = ui
        .selectable_label(*bld, "BLD")
        .on_hover_explanation("Blindfold mode", "Hides sticker colors");
    if r.clicked() {
        app.event(Command::ToggleBlindfold);
    }
}

fn twist_count(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    let metric = &mut app.prefs.info.metric;
    let twist_count = app.puzzle.twist_count(*metric);
    let r = ui
        .add(egui::Label::new(format!("{}: {}", metric, twist_count)).sense(egui::Sense::click()));
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

    let frame = egui::Frame::popup(ui.style());
    let offset = [
        -frame.inner_margin.right,
        -ui.available_height() - 4.0, // magic number 4.0 from `egui::popup::show_tooltip_for()` source code
    ];
    let popup_id = unique_id!();
    if ui.memory().is_popup_open(popup_id) {
        let popup_response = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .fixed_pos(r.rect.right_bottom())
            .anchor(egui::Align2::RIGHT_BOTTOM, offset)
            .show(ui.ctx(), |ui| {
                // Note: we use a separate clip-rect for this area, so the popup can be outside the parent.
                // See https://github.com/emilk/egui/issues/825
                frame.show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        ui.horizontal_top(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.set_width(EXPLANATION_TOOLTIP_WIDTH);
                                ui.vertical(|ui| {
                                    ui.strong(metric.get_message().unwrap_or(""));
                                    ui.label(metric.long_description());
                                })
                            });
                            ui.with_layout(
                                egui::Layout::top_down_justified(egui::Align::LEFT),
                                |ui| {
                                    ui.set_width(100.0);

                                    let mut selectable_metric = |ui: &mut egui::Ui, m| {
                                        changed |= ui
                                            .selectable_value(
                                                metric,
                                                m,
                                                format!("{m}: {}", app.puzzle.twist_count(m)),
                                            )
                                            .changed();
                                    };

                                    selectable_metric(ui, TwistMetric::Atm);
                                    selectable_metric(ui, TwistMetric::Etm);
                                    ui.separator();
                                    if app.prefs.info.qtm {
                                        selectable_metric(ui, TwistMetric::Qstm);
                                        selectable_metric(ui, TwistMetric::Qbtm);
                                        selectable_metric(ui, TwistMetric::Qobtm);
                                    } else {
                                        selectable_metric(ui, TwistMetric::Stm);
                                        selectable_metric(ui, TwistMetric::Btm);
                                        selectable_metric(ui, TwistMetric::Obtm);
                                    }
                                    changed |= ui
                                        .add(egui::Checkbox::new(&mut app.prefs.info.qtm, "QTM"))
                                        .changed();
                                    metric.set_qtm(app.prefs.info.qtm);
                                },
                            );
                        });
                    })
                })
            })
            .response;

        if ui.input().key_pressed(egui::Key::Escape) || popup_response.clicked_elsewhere() {
            ui.memory().close_popup();
        }
    }
    if r.clicked() {
        ui.memory().open_popup(popup_id);
    }

    app.prefs.needs_save |= changed;
}
