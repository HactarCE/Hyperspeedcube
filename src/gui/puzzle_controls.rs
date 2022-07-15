use super::util;
use crate::app::App;
use crate::puzzle::{traits::*, LayerMask, Twist, TwistDirection, TwistSelection};

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    egui::ScrollArea::new([false, true]).show(ui, |ui| {
        build_twist_section(ui, app);
    });
}

fn build_twist_section(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();

    let sel = app.puzzle_selection();
    let toggle_sel = &mut app.toggle_selections;

    ui.horizontal(|ui| {
        ui.style_mut().wrap = Some(false);
        ui.heading("Puzzle Controls");
        util::reset_button(ui, toggle_sel, TwistSelection::default(), "");
    });

    ui.separator();

    let h_layout = egui::Layout::left_to_right()
        .with_cross_align(egui::Align::TOP)
        .with_main_wrap(true);

    ui.strong("Twist axis");
    ui.with_layout(h_layout, |ui| {
        for (i, twist_axis) in puzzle_type.twist_axes().iter().enumerate() {
            let bit = 1 << i;
            let mut is_sel = sel.axis_mask & bit != 0;
            let r = ui.selectable_value(&mut is_sel, true, twist_axis.name);
            if r.changed() {
                toggle_sel.axis_mask ^= bit;
                if !ui.input().modifiers.command {
                    toggle_sel.axis_mask &= bit;
                }
            }
        }
    });

    ui.separator();

    ui.strong("Layers");
    ui.with_layout(h_layout, |ui| {
        for i in 0..puzzle_type.layer_count() {
            let bit = 1 << i;
            let mut is_sel = sel.layer_mask & bit != 0;
            let r = ui.selectable_value(&mut is_sel, true, format!("{}", i + 1));
            if r.changed() {
                toggle_sel.layer_mask ^= bit;
            }
        }
    });

    ui.separator();

    let twist_axis = app.selected_twist_axis(None);
    let can_twist = twist_axis.is_ok() && sel.layer_mask != 0_u32;

    ui.strong("Twist");
    ui.add_enabled_ui(can_twist, |ui| {
        ui.with_layout(h_layout, |ui| {
            for (i, twist_direction) in puzzle_type.twist_directions().iter().enumerate() {
                if ui.button(twist_direction.name).clicked() {
                    if let Ok(axis) = twist_axis {
                        // should always be `Some`
                        app.event(Twist {
                            axis,
                            direction: TwistDirection(i as _),
                            layers: app.selected_layers(None),
                        })
                    }
                }
            }
        });
    });
}
