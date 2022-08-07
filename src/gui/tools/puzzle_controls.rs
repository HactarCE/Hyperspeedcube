use super::super::util;
use crate::app::App;
use crate::puzzle::*;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();

    let grip = app.grip();

    let h_layout = egui::Layout::left_to_right()
        .with_cross_align(egui::Align::TOP)
        .with_main_wrap(true);

    // Allow selecting multiple by holding cmd/ctrl.
    let multi_select = ui.input().modifiers.command;

    ui.strong("Twist axis");
    ui.with_layout(h_layout, |ui| {
        util::reset_button(ui, &mut app.toggle_grip.axes, Grip::default().axes, "");
        for (i, twist_axis) in puzzle_type.twist_axes().iter().enumerate() {
            let mut is_sel = grip.axes.contains(&TwistAxis(i as _));
            let r = ui.selectable_value(&mut is_sel, true, twist_axis.name);
            if r.changed() {
                app.toggle_grip
                    .toggle_axis(TwistAxis(i as _), !multi_select);
            }
        }
    });

    ui.separator();

    ui.strong("Layers");
    ui.with_layout(h_layout, |ui| {
        util::reset_button(ui, &mut app.toggle_grip.layers, Grip::default().layers, "");
        for i in 0..puzzle_type.layer_count() {
            let mut is_sel = grip.layers.unwrap_or_default()[i as u8];
            let r = ui.selectable_value(&mut is_sel, true, format!("{}", i + 1));
            if r.changed() {
                app.toggle_grip.toggle_layer(i as u8, !multi_select);
            }
        }
    });

    ui.separator();

    let twist_axis = app.gripped_twist_axis(None);
    let can_twist = twist_axis.is_ok() && grip.layers != Some(LayerMask(0));

    ui.strong("Twist");
    ui.add_enabled_ui(can_twist, |ui| {
        ui.with_layout(h_layout, |ui| {
            for (i, twist_direction) in puzzle_type.twist_directions().iter().enumerate() {
                if ui.button(twist_direction.name).clicked() {
                    if let Ok(axis) = twist_axis {
                        // should always be `Ok`
                        app.event(Twist {
                            axis,
                            direction: TwistDirection(i as _),
                            layers: app.grip().layers.unwrap_or_default(),
                        })
                    }
                }
            }
        });
    });
}
