use super::util;
use crate::app::App;
use crate::puzzle::{LayerMask, PuzzleControllerTrait, PuzzleTypeTrait, Selection, TwistDirection};

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    egui::ScrollArea::new([false, true]).show(ui, |ui| {
        build_select_section(ui, app);
        ui.separator();
        build_twist_section(ui, app);
    });
}

fn build_select_section(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    let sel = &mut app.toggle_selections;
    ui.horizontal(|ui| {
        ui.style_mut().wrap = Some(false);
        ui.heading("Select");
        let r = util::reset_button(ui, sel, Selection::default(), "");
        changed |= r.clicked();
    });
    ui.separator();
    let puzzle_type = app.puzzle.ty();

    let h_layout = egui::Layout::left_to_right()
        .with_cross_align(egui::Align::TOP)
        .with_main_wrap(true);

    ui.strong("Faces");
    ui.with_layout(h_layout, |ui| {
        for face in puzzle_type.faces() {
            let bit = 1 << face.id();
            let mut is_sel = sel.face_mask & bit != 0;
            let r = ui.selectable_value(&mut is_sel, true, face.name());
            if r.changed() {
                sel.face_mask ^= bit;
                changed = true;
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
                changed = true;
            }
        }
    });

    ui.separator();

    ui.strong("Piece types");
    ui.with_layout(h_layout, |ui| {
        for (i, &piece_type) in puzzle_type.piece_type_names().iter().enumerate() {
            let bit = 1 << i;
            let mut is_sel = sel.piece_type_mask & bit != 0;
            let r = ui.selectable_value(&mut is_sel, true, piece_type);
            if r.changed() {
                sel.piece_type_mask ^= bit;
                changed = true;
            }
        }
    });

    app.wants_repaint |= changed;
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

    ui.heading("Twist");
    ui.add_enabled_ui(can_twist, |ui| {
        ui.with_layout(h_layout, |ui| {
            for &twist_direction in puzzle_type.twist_direction_names() {
                if ui.button(twist_direction).clicked() {
                    app.do_twist(
                        None,
                        TwistDirection::from_name(puzzle_type, twist_direction),
                        LayerMask::default(),
                    );
                }
            }
        });
    });
}
