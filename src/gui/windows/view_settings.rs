use crate::app::App;
use crate::gui::util::{self, presets_list};
use crate::preferences::DEFAULT_PREFS;
use crate::puzzle::{traits::*, ProjectionType};

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();
    let proj_ty = puzzle_type.projection_type();
    let prefs = &mut app.prefs;

    let mut changed = false;

    ui.collapsing("Presets", |ui| {
        let presets = prefs.view_presets(&app.puzzle);
        presets_list(
            ui,
            unique_id!(),
            &mut presets.current,
            &mut presets.presets,
            |ui, preset, current_value| {
                if ui.button("Load").clicked() {
                    *current_value = preset.value.clone();
                }
                ui.label(preset.name);
            },
        );
    });

    let mut prefs_ui = util::PrefsUi {
        ui,
        current: prefs.view_mut(puzzle_type),
        defaults: DEFAULT_PREFS.view(puzzle_type),
        changed: &mut changed,
    };

    prefs_ui.collapsing("View angle", |mut prefs_ui| {
        prefs_ui.angle("Pitch", access!(.pitch), |dv| dv.clamp_range(-90.0..=90.0));
        prefs_ui.angle("Yaw", access!(.yaw), |dv| dv.clamp_range(-45.0..=45.0));
    });

    prefs_ui.collapsing("Projection", |mut prefs_ui| {
        let speed = prefs_ui.current.scale / 100.0; // logarithmic speed
        prefs_ui.angle("Scale", access!(.scale), |dv| {
            dv.fixed_decimals(2).clamp_range(0.1..=5.0_f32).speed(speed)
        });

        if proj_ty == ProjectionType::_4D {
            prefs_ui.angle("4D FOV", access!(.fov_4d), |dv| {
                dv.clamp_range(1.0..=120.0).speed(0.5)
            });
        }

        prefs_ui.angle("3D FOV", access!(.fov_3d), |dv| {
            dv.clamp_range(-120.0..=120.0).speed(0.5)
        });
    });

    prefs_ui.collapsing("Geometry", |mut prefs_ui| {
        if proj_ty == ProjectionType::_3D {
            prefs_ui.checkbox("Show frontfaces", access!(.show_frontfaces));
            prefs_ui.checkbox("Show backfaces", access!(.show_backfaces));
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
