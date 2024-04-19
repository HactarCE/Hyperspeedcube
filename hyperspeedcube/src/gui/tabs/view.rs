use crate::app::App;
use crate::gui::components::{big_icon_button, with_reset_button, PrefsUi};
use crate::preferences::DEFAULT_PREFS;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.add_enabled_ui(app.has_active_puzzle(), |ui| {
        let Some(puzzle_type) = app.active_puzzle_type() else {
            ui.label("No puzzle loaded");
            return;
        };

        use parking_lot::Mutex;
        lazy_static! {
            static ref LOADED: Mutex<String> = Mutex::new("Fallback".to_string());
            static ref NAME: Mutex<String> = Mutex::new("Fallback".to_string());
        }

        ui.strong("Saved presets");
        ui.horizontal_wrapped(|ui| {
            ui.allocate_ui_with_layout(
                egui::Vec2::splat(22.0),
                egui::Layout {
                    main_dir: egui::Direction::LeftToRight,
                    main_wrap: false,
                    main_align: egui::Align::Center,
                    main_justify: true,
                    cross_align: egui::Align::Center,
                    cross_justify: true,
                },
                |ui| {
                    ui.menu_button("➕", |ui| {
                        ui.set_max_width(200.0);
                        ui.button("New empty preset");
                        ui.button("New preset from current settings");
                    });
                },
            );

            for s in [
                "Fallback",
                "Speedsolving",
                "Unfolded (back)",
                "Unfolded (front)",
            ] {
                if ui.selectable_label(*LOADED.lock() == s, s).clicked() {
                    *LOADED.lock() = s.to_string();
                }
            }
        });
        ui.separator();

        ui.strong("Current preset");
        ui.horizontal(|ui| {
            big_icon_button(ui, "🗑", &format!("Delete preset {}", NAME.lock()));
            big_icon_button(ui, "💾", &format!("Overwrite preset {}", NAME.lock()));
            with_reset_button(ui, &mut *NAME.lock(), LOADED.lock().clone(), "", |ui, s| {
                ui.add(egui::TextEdit::singleline(s).desired_width(150.0))
            });

            static A: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
            // ui.add_enabled_ui(A.load(std::sync::atomic::Ordering::Relaxed), |ui| {
            //     if ui.button("Save").clicked() {
            //         A.store(false, std::sync::atomic::Ordering::Relaxed);
            //     }
            // });
        });
        ui.collapsing("Defaults", |ui| {
            egui::ComboBox::new(unique_id!(), "Everything")
                .selected_text("(none)")
                .show_ui(ui, |ui| {
                    ui.button("(none)");
                    ui.button("Fallback");
                    ui.button("Speedsolving");
                    ui.button("Unfolded (back)");
                    ui.button("Unfolded (fallback)");
                    Some(())
                });
            egui::ComboBox::new(unique_id!(), "Cube")
                .selected_text("(none)")
                .show_ui(ui, |ui| {
                    ui.button("(none)");
                    ui.button("Fallback");
                    ui.button("Speedsolving");
                    ui.button("Unfolded (back)");
                    ui.button("Unfolded (fallback)");
                    Some(())
                });
            egui::ComboBox::new(unique_id!(), "3x3x3x3")
                .selected_text("(none)")
                .show_ui(ui, |ui| {
                    ui.button("(none)");
                    ui.button("Fallback");
                    ui.button("Speedsolving");
                    ui.button("Unfolded (back)");
                    ui.button("Unfolded (fallback)");
                    Some(())
                });
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let prefs = &mut app.prefs;
                let presets = prefs.view_presets(&puzzle_type);

                let mut changed = false;

                let mut prefs_ui = PrefsUi {
                    ui,
                    current: &mut presets.current,
                    defaults: match &presets.active_preset {
                        Some(p) => &p.value,
                        None => DEFAULT_PREFS.view(&puzzle_type),
                    },
                    changed: &mut changed,
                };

                prefs_ui.collapsing("View angle", |mut prefs_ui| {
                    prefs_ui.angle("Pitch", access!(.pitch), |dv| dv.clamp_range(-90.0..=90.0));
                    prefs_ui.angle("Yaw", access!(.yaw), |dv| dv.clamp_range(-180.0..=180.0));
                    prefs_ui.angle("Roll", access!(.roll), |dv| dv.clamp_range(-180.0..=180.0));
                });

                prefs_ui.collapsing("Projection", |mut prefs_ui| {
                    let speed = prefs_ui.current.scale / 100.0; // logarithmic speed
                    prefs_ui.num("Scale", access!(.scale), |dv| {
                        dv.fixed_decimals(2).clamp_range(0.1..=5.0_f32).speed(speed)
                    });

                    if puzzle_type.ndim() >= 4 {
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
                    if puzzle_type.ndim() == 3 {
                        prefs_ui.checkbox("Show frontfaces", access!(.show_frontfaces));
                        prefs_ui.checkbox("Show backfaces", access!(.show_backfaces));
                    }
                    if puzzle_type.ndim() >= 4 {
                        prefs_ui.checkbox("Clip 4D backfaces", access!(.clip_4d_backfaces));
                        prefs_ui.checkbox("Clip 4D behind camera", access!(.clip_4d_behind_camera));
                    }

                    if puzzle_type.ndim() == 3 {
                        prefs_ui.checkbox("Show internals", access!(.show_internals));
                    }

                    let showing_internals =
                        puzzle_type.ndim() == 3 && prefs_ui.current.show_internals;
                    prefs_ui.add_enabled_ui(
                        !showing_internals,
                        "Disabled when showing internals",
                        |mut prefs_ui| {
                            prefs_ui.num("Face shrink", access!(.facet_shrink), |dv| {
                                dv.fixed_decimals(2)
                                    .clamp_range(0.0..=0.95_f32)
                                    .speed(0.005)
                            })
                        },
                    );
                    prefs_ui.add_enabled_ui(
                        !showing_internals,
                        "Disabled when showing internals",
                        |mut prefs_ui| {
                            prefs_ui.num("Sticker shrink", access!(.sticker_shrink), |dv| {
                                dv.fixed_decimals(2)
                                    .clamp_range(0.0..=0.95_f32)
                                    .speed(0.005)
                            })
                        },
                    );

                    prefs_ui.num("Piece explode", access!(.piece_explode), |dv| {
                        dv.fixed_decimals(2).clamp_range(0.0..=5.0_f32).speed(0.01)
                    });
                });

                prefs_ui.collapsing("Lighting", |mut prefs_ui| {
                    prefs_ui.angle("Pitch", access!(.light_pitch), |dv| {
                        dv.clamp_range(-90.0..=90.0)
                    });
                    prefs_ui.angle("Yaw", access!(.light_yaw), |dv| {
                        dv.clamp_range(-180.0..=180.0)
                    });
                    prefs_ui.percent("Intensity (faces)", access!(.face_light_intensity));
                    prefs_ui.percent("Intensity (outlines)", access!(.outline_light_intensity));
                });

                prefs_ui.collapsing("Performance", |mut prefs_ui| {
                    prefs_ui.num("Downscale factor", access!(.downscale_rate), |dv| {
                        dv.clamp_range(1..=32).speed(0.1)
                    });
                    prefs_ui.checkbox("Downscale interpolation", access!(.downscale_interpolate));
                });

                prefs.needs_save |= changed;
            });
    });
}
