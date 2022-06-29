use super::util;
use crate::app::App;
use crate::puzzle::{traits::*, LayerMask, Selection, TwistDirection};

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    egui::ScrollArea::new([false, true]).show(ui, |ui| {
        build_select_section(ui, app);
        ui.separator();
        build_twist_section(ui, app);
    });
}

fn build_select_section(ui: &mut egui::Ui, app: &mut App) {
    let sel = &mut app.toggle_selections;
    ui.horizontal(|ui| {
        ui.style_mut().wrap = Some(false);
        ui.heading("Puzzle Controls");
        util::reset_button(ui, sel, Selection::default(), "");
    });
    ui.separator();
    let puzzle_type = app.puzzle.ty();

    let h_layout = egui::Layout::left_to_right()
        .with_cross_align(egui::Align::TOP)
        .with_main_wrap(true);

    ui.strong("Faces");
    ui.with_layout(h_layout, |ui| {
        for (i, face) in puzzle_type.faces().iter().enumerate() {
            let bit = 1 << i;
            let mut is_sel = sel.face_mask & bit != 0;
            let r = ui.selectable_value(&mut is_sel, true, face.name);
            if r.changed() {
                sel.face_mask ^= bit;
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
                sel.layer_mask ^= bit;
            }
        }
    });

    ui.separator();

    // TODO: piece types

    // ui.strong("Piece types");
    // ui.with_layout(h_layout, |ui| {
    //     for (i, &piece_type) in puzzle_type.piece_type_names().iter().enumerate() {
    //         let bit = 1 << i;
    //         let mut is_sel = sel.piece_type_mask & bit != 0;
    //         let r = ui.selectable_value(&mut is_sel, true, piece_type);
    //         if r.changed() {
    //             sel.piece_type_mask ^= bit;
    //         }
    //     }
    // });
}

fn build_twist_section(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();

    let h_layout = egui::Layout::left_to_right()
        .with_cross_align(egui::Align::TOP)
        .with_main_wrap(true);

    let can_twist = app
        .puzzle_selection()
        .exactly_one_face(puzzle_type)
        .is_some();

    ui.strong("Twist");
    ui.add_enabled_ui(can_twist, |ui| {
        ui.with_layout(h_layout, |ui| {
            for (i, twist_direction) in puzzle_type.twist_directions().iter().enumerate() {
                if ui.button(twist_direction.name).clicked() {
                    app.do_twist(None, TwistDirection(i as _), LayerMask::default());
                }
            }
        });
    });
}
