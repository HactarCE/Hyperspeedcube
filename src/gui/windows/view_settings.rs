use crate::app::App;
use crate::gui::util;
use crate::preferences::DEFAULT_PREFS;
use crate::puzzle::{traits::*, ProjectionType};

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    let proj_ty = app.puzzle.ty().projection_type();
    let prefs = &mut app.prefs;

    let mut changed = false;

    ui.collapsing("View angle", |ui| {
        // Pitch
        let r = ui.add(resettable!(
            "Pitch",
            "{}°",
            (prefs[proj_ty].pitch),
            |value| util::make_degrees_drag_value(value).clamp_range(-90.0..=90.0),
        ));
        changed |= r.changed();

        // Yaw
        let r = ui.add(resettable!("Yaw", "{}°", (prefs[proj_ty].yaw), |value| {
            util::make_degrees_drag_value(value).clamp_range(-45.0..=45.0)
        }));
        changed |= r.changed();
    });

    ui.collapsing("Projection", |ui| {
        // Scale
        let r = ui.add(resettable!("Scale", (prefs[proj_ty].scale), |value| {
            let speed = *value / 100.0; // logarithmic speed
            egui::DragValue::new(value)
                .fixed_decimals(2)
                .clamp_range(0.1..=5.0_f32)
                .speed(speed)
        }));
        changed |= r.changed();

        if proj_ty == ProjectionType::_4D {
            // 4D FOV
            let r = ui.add(resettable!(
                "4D FOV",
                "{}°",
                (prefs.view_4d.fov_4d),
                |value| {
                    util::make_degrees_drag_value(value)
                        .clamp_range(1.0..=120.0)
                        .speed(0.5)
                },
            ));
            changed |= r.changed();
        }

        // 3D FOV
        let r = ui.add(resettable!(
            "3D FOV",
            "{}°",
            (prefs[proj_ty].fov_3d),
            |value| {
                util::make_degrees_drag_value(value)
                    .clamp_range(-120.0..=120.0)
                    .speed(0.5)
            },
        ));
        changed |= r.changed();
    });

    ui.collapsing("Geometry", |ui| {
        if proj_ty == ProjectionType::_3D {
            // Show front faces
            let r = ui.add(util::CheckboxWithReset {
                label: "Show frontfaces",
                value: &mut prefs.view_3d.show_frontfaces,
                reset_value: DEFAULT_PREFS.view_3d.show_frontfaces,
            });
            changed |= r.changed();

            // Show back faces
            let r = ui.add(util::CheckboxWithReset {
                label: "Show backfaces",
                value: &mut prefs.view_3d.show_backfaces,
                reset_value: DEFAULT_PREFS.view_3d.show_backfaces,
            });
            changed |= r.changed();
        }

        // Face spacing
        let r = ui.add(resettable!(
            "Face spacing",
            (prefs[proj_ty].face_spacing),
            |value| {
                egui::DragValue::new(value)
                    .fixed_decimals(2)
                    .clamp_range(0.0..=0.9_f32)
                    .speed(0.005)
            },
        ));
        changed |= r.changed();

        // Sticker spacing
        let r = ui.add(resettable!(
            "Sticker spacing",
            (prefs[proj_ty].sticker_spacing),
            |value| {
                egui::DragValue::new(value)
                    .fixed_decimals(2)
                    .clamp_range(0.0..=0.9_f32)
                    .speed(0.005)
            },
        ));
        changed |= r.changed();
    });

    ui.collapsing("Lighting", |ui| {
        // Pitch
        let r = ui.add(resettable!(
            "Pitch",
            "{}°",
            (prefs[proj_ty].light_pitch),
            |value| util::make_degrees_drag_value(value).clamp_range(-90.0..=90.0),
        ));
        changed |= r.changed();

        // Yaw
        let r = ui.add(resettable!(
            "Yaw",
            "{}°",
            (prefs[proj_ty].light_yaw),
            |value| util::make_degrees_drag_value(value).clamp_range(-180.0..=180.0),
        ));
        changed |= r.changed();

        // Directional
        let r = ui.add(resettable!(
            "Directional",
            |x| format!("{:.0}%", x * 100.0),
            (prefs[proj_ty].light_directional),
            util::make_percent_drag_value,
        ));
        changed |= r.changed();

        // Ambient
        let r = ui.add(resettable!(
            "Ambient",
            |x| format!("{:.0}%", x * 100.0),
            (prefs[proj_ty].light_ambient),
            util::make_percent_drag_value,
        ));
        changed |= r.changed();
    });

    prefs.needs_save |= changed;
}
