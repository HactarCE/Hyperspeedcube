use crate::app::App;
use crate::gui::{util, widgets};
use crate::preferences::DEFAULT_PREFS;
use crate::puzzle::{traits::*, ProjectionType};

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();
    let proj_ty = puzzle_type.projection_type();
    let prefs = &mut app.prefs;
    let presets = prefs.view_presets(&app.puzzle);

    let mut changed = false;

    ui.collapsing("Presets", |ui| {
        let mut presets_ui = widgets::PresetsUi {
            id: unique_id!(),
            presets: &mut presets.presets,
            changed: &mut changed,
            strings: Default::default(),
            enable_yaml: true,
        };

        presets_ui.show_header_with_active_preset(
            ui,
            || presets.current.clone(),
            |new_preset| presets.active_preset = Some(new_preset.clone()),
        );
        ui.separator();
        presets_ui.show_list(ui, |ui, _idx, preset| {
            let mut changed = false;

            let mut r = ui.scope(|ui| {
                if ui.button("Load").clicked() {
                    let old = std::mem::replace(&mut presets.current, preset.value.clone());
                    app.puzzle.animate_from_view_settings(old);
                    presets.active_preset = Some(preset.clone());
                    changed = true;
                }
                if presets.active_preset.as_ref() == Some(preset) {
                    ui.strong(&preset.preset_name);
                } else {
                    ui.label(&preset.preset_name);
                }
            });
            if changed {
                r.response.mark_changed();
            }
            r.response
        });
    });

    let mut prefs_ui = util::PrefsUi {
        ui,
        current: &mut presets.current,
        defaults: match &presets.active_preset {
            Some(p) => &p.value,
            None => DEFAULT_PREFS.view(puzzle_type),
        },
        changed: &mut changed,
    };

    prefs_ui.collapsing("Position", |mut prefs_ui| {
        prefs_ui.float("Horizontal align", access!(.align_h), |dv| {
            dv.clamp_range(-1.0..=1.0).fixed_decimals(2).speed(0.01)
        });
        prefs_ui.float("Vertical align", access!(.align_v), |dv| {
            dv.clamp_range(-1.0..=1.0).fixed_decimals(2).speed(0.01)
        });
    });

    prefs_ui.collapsing("View angle", |mut prefs_ui| {
        prefs_ui.angle("Pitch", access!(.pitch), |dv| dv.clamp_range(-90.0..=90.0));
        prefs_ui.angle("Yaw", access!(.yaw), |dv| dv.clamp_range(-180.0..=180.0));
        prefs_ui.angle("Roll", access!(.roll), |dv| dv.clamp_range(-180.0..=180.0));
    });

    prefs_ui.collapsing("Projection", |mut prefs_ui| {
        let speed = prefs_ui.current.scale / 100.0; // logarithmic speed
        prefs_ui.float("Scale", access!(.scale), |dv| {
            dv.fixed_decimals(2).clamp_range(0.1..=5.0_f32).speed(speed)
        });

        if proj_ty == ProjectionType::_4D {
            prefs_ui.angle("4D FOV", access!(.fov_4d), |dv| {
                dv.clamp_range(1.0..=120.0).speed(0.5)
            });
        }

        let label = if prefs_ui.current.fov_3d == 120.0 {
            "QUAKE PRO"
        } else if prefs_ui.current.fov_3d == -120.0 {
            "ORP EKAUQ"
        } else {
            "3D FOV"
        };
        prefs_ui.angle(label, access!(.fov_3d), |dv| {
            dv.clamp_range(-120.0..=120.0).speed(0.5)
        });
    });

    prefs_ui.collapsing("Geometry", |mut prefs_ui| {
        if proj_ty == ProjectionType::_3D {
            prefs_ui.checkbox("Show frontfaces", access!(.show_frontfaces));
            prefs_ui.checkbox("Show backfaces", access!(.show_backfaces));
        }
        if proj_ty == ProjectionType::_4D {
            prefs_ui.checkbox("Clip 4D", access!(.clip_4d));
        }

        prefs_ui.float("Face spacing", access!(.face_spacing), |dv| {
            dv.fixed_decimals(2).clamp_range(0.0..=0.9_f32).speed(0.005)
        });

        prefs_ui.float("Sticker spacing", access!(.sticker_spacing), |dv| {
            dv.fixed_decimals(2).clamp_range(0.0..=0.9_f32).speed(0.005)
        });
    });

    prefs_ui.collapsing("Lighting", |mut prefs_ui| {
        prefs_ui.angle("Pitch", access!(.light_pitch), |dv| {
            dv.clamp_range(-90.0..=90.0)
        });
        prefs_ui.angle("Yaw", access!(.light_yaw), |dv| {
            dv.clamp_range(-180.0..=180.0)
        });
        prefs_ui.percent("Directional", access!(.light_directional));
        prefs_ui.percent("Ambient", access!(.light_ambient));
    });

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}
