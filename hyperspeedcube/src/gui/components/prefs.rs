use egui::NumExt;

use crate::app::App;
use crate::gui::components::{
    big_icon_button, small_icon_button, with_reset_button, PresetsUi, WidgetWithReset,
};
use crate::gui::ext::*;
use crate::gui::util::Access;
use crate::preferences::{InteractionPreferences, PuzzleViewPreferencesSet, ViewPreferences};
use crate::serde_impl::hex_color;

pub struct PrefsUi<'a, T> {
    pub ui: &'a mut egui::Ui,
    pub current: &'a mut T,
    pub defaults: &'a T,
    pub changed: &'a mut bool,
}
impl<'a, T> PrefsUi<'a, T> {
    fn add<'s, 'w, W>(&'s mut self, make_widget: impl FnOnce(&'w mut T) -> W) -> egui::Response
    where
        's: 'w,
        T: 'w,
        W: 'w + egui::Widget,
    {
        let r = self.ui.add(make_widget(self.current));
        *self.changed |= r.changed();
        r
    }

    pub fn add_enabled_ui(
        &mut self,
        enabled: bool,
        explanation: impl Into<egui::WidgetText>,
        add_contents: impl FnOnce(PrefsUi<'_, T>) -> egui::Response,
    ) {
        self.ui.add_enabled_ui(enabled, |ui| {
            ui.vertical(|ui| {
                add_contents(PrefsUi {
                    ui,
                    current: self.current,
                    defaults: self.defaults,
                    changed: self.changed,
                })
            })
            .response
            .on_disabled_hover_text(explanation);
        });
    }

    pub fn collapsing<R>(
        &mut self,
        heading: impl Into<egui::WidgetText>,
        add_contents: impl FnOnce(PrefsUi<'_, T>) -> R,
    ) -> egui::CollapsingResponse<R> {
        egui::CollapsingHeader::new(heading)
            .default_open(true)
            .show(self.ui, |ui| {
                add_contents(PrefsUi {
                    ui,
                    current: self.current,
                    defaults: self.defaults,
                    changed: self.changed,
                })
            })
    }

    pub fn checkbox(&mut self, label: &str, access: Access<T, bool>) -> egui::Response {
        let reset_value = *(access.get_ref)(self.defaults);
        self.add(|current| {
            |ui: &mut egui::Ui| {
                let value = (access.get_mut)(current);
                with_reset_button(ui, value, reset_value, "", |ui, value| {
                    ui.checkbox(value, label)
                })
            }
        })
    }

    pub fn num<N: egui::emath::Numeric + ToString>(
        &mut self,
        label: &str,
        access: Access<T, N>,
        modify_widget: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
    ) -> egui::Response {
        let reset_value = *(access.get_ref)(self.defaults);
        let reset_value_str = reset_value.to_string();
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str,
            make_widget: |value| modify_widget(egui::DragValue::new(value)),
        })
    }

    pub fn percent(&mut self, label: &str, access: Access<T, f32>) -> egui::Response {
        let reset_value = *(access.get_ref)(self.defaults);
        let reset_value_str = reset_value.to_string();
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str,
            make_widget: |value| {
                egui::DragValue::from_get_set(|new_value| {
                    if let Some(x) = new_value {
                        *value = x as f32 / 100.0;
                    }
                    *value as f64 * 100.0
                })
                .suffix("%")
                .fixed_decimals(0)
                .clamp_range(0.0..=100.0_f32)
                .speed(0.5)
            },
        })
    }

    pub fn angle(
        &mut self,
        label: &str,
        access: Access<T, f32>,
        modify_widget: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
    ) -> egui::Response {
        let reset_value = *(access.get_ref)(self.defaults);
        let reset_value_str = format!("{}°", &reset_value);
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str,
            make_widget: |value| {
                modify_widget(egui::DragValue::new(value).suffix("°").fixed_decimals(0))
            },
        })
    }

    pub fn color(&mut self, label: &str, access: Access<T, egui::Color32>) -> egui::Response {
        let reset_value = *(access.get_ref)(self.defaults);
        let reset_value_str = hex_color::to_str(&reset_value);
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str,
            make_widget: |value| |ui: &mut egui::Ui| ui.color_edit_button_srgba(value),
        })
    }
}

// pub fn build_colors_section(ui: &mut egui::Ui, app: &mut App) {
//     let prefs = &mut app.prefs;

//     let mut changed = false;
//     let mut prefs_ui = PrefsUi {
//         ui,
//         current: &mut prefs.colors,
//         defaults: &DEFAULT_PREFS.colors,
//         changed: &mut changed,
//     };

//     // prefs_ui.ui.strong("Faces");
//     // for (i, &face) in puzzle_type.faces().iter().enumerate() {
//     //     prefs_ui.color(face.name, access!([(puzzle_type, Face(i as _))]));
//     // }

//     // prefs_ui.ui.separator();

//     prefs_ui.ui.strong("Special");
//     prefs_ui.color("Background", access!(.background));
//     prefs_ui.color("Blindfolded stickers", access!(.blind_face));
//     prefs_ui.checkbox("Blindfold mode", access!(.blindfold));

//     prefs.needs_save |= changed;
//     if changed {
//         app.request_redraw_puzzle();
//     }
// }
// pub fn build_graphics_section(ui: &mut egui::Ui, app: &mut App) {
//     let prefs = &mut app.prefs;

//     let mut changed = false;
//     let mut prefs_ui = PrefsUi {
//         ui,
//         current: &mut prefs.gfx,
//         defaults: &DEFAULT_PREFS.gfx,
//         changed: &mut changed,
//     };

//     let speed = prefs_ui.current.fps_limit as f64 / 1000.0; // logarithmic speed
//     prefs_ui
//         .num("FPS limit", access!(.fps_limit), |dv| {
//             dv.fixed_decimals(0).clamp_range(30..=1000).speed(speed)
//         })
//         .on_hover_explanation("Frames Per Second", "Limits framerate to save power");

//     let is_msaa_disabled = cfg!(target_arch = "wasm32");
//     prefs_ui.ui.add_enabled_ui(!is_msaa_disabled, |ui| {
//         PrefsUi { ui, ..prefs_ui }
//             .checkbox("MSAA", access!(.msaa))
//             .on_hover_explanation(
//                 "Multisample Anti-Aliasing",
//                 "Makes edges less jagged, \
//                  but may worsen performance.",
//             )
//             .on_disabled_hover_text(
//                 "Multisample anti-aliasing \
//                  is not supported on web.",
//             );
//     });

//     prefs.needs_save |= changed;
// }
pub fn build_interaction_section(mut prefs_ui: PrefsUi<'_, InteractionPreferences>) {
    prefs_ui
        .checkbox(
            "Confirm discard only when scrambled",
            access!(.confirm_discard_only_when_scrambled),
        )
        .on_hover_explanation(
            "",
            "When enabled, a confirmation dialog before \
             destructive actions (like resetting the puzzle) \
             is only shown when the puzzle has been fully \
             scrambled.",
        );

    prefs_ui.ui.separator();

    prefs_ui.num("Drag sensitivity", access!(.drag_sensitivity), |dv| {
        dv.fixed_decimals(2).clamp_range(0.0..=3.0_f32).speed(0.01)
    });
    prefs_ui
        .checkbox("Realign puzzle on release", access!(.realign_on_release))
        .on_hover_explanation(
            "",
            "When enabled, the puzzle snaps back immediately when \
             the mouse is released after dragging to rotate it.",
        );
    prefs_ui
        .checkbox("Realign puzzle on keypress", access!(.realign_on_keypress))
        .on_hover_explanation(
            "",
            "When enabled, the puzzle snaps back immediately when \
             the keyboard is used to grip or do a move.",
        );
    prefs_ui
        .checkbox("Smart realign", access!(.smart_realign))
        .on_hover_explanation(
            "",
            "When enabled, the puzzle snaps to the nearest \
             similar orientation, not the original. This \
             adds a full-puzzle rotation to the undo history.",
        );

    prefs_ui.ui.separator();

    prefs_ui.collapsing("Animations", |mut prefs_ui| {
        prefs_ui
            .checkbox("Dynamic twist speed", access!(.dynamic_twist_speed))
            .on_hover_explanation(
                "",
                "When enabled, the puzzle twists faster when \
                 many moves are queued up. When all queued \
                 moves are complete, the twist speed resets.",
            );

        let speed = prefs_ui.current.twist_duration.at_least(0.1) / 100.0; // logarithmic speed
        prefs_ui.num("Twist duration", access!(.twist_duration), |dv| {
            dv.fixed_decimals(2).clamp_range(0.0..=5.0_f32).speed(speed)
        });

        let speed = prefs_ui.current.other_anim_duration.at_least(0.1) / 100.0; // logarithmic speed
        prefs_ui
            .num("Other animations", access!(.other_anim_duration), |dv| {
                dv.fixed_decimals(2).clamp_range(0.0..=1.0_f32).speed(speed)
            })
            .on_hover_explanation(
                "",
                "Number of seconds for other animations, \
                 such as hiding a piece.",
            );
    });
}
// pub fn build_outlines_section(ui: &mut egui::Ui, app: &mut App) {
//     let prefs = &mut app.prefs;

//     let mut changed = false;
//     let mut prefs_ui = PrefsUi {
//         ui,
//         current: &mut prefs.outlines,
//         defaults: &DEFAULT_PREFS.outlines,
//         changed: &mut changed,
//     };

//     prefs_ui.collapsing("Outline colors", |mut prefs_ui| {
//         prefs_ui
//             .checkbox("Use sticker colors", access!(.use_sticker_colors))
//             .on_hover_explanation(
//                 "",
//                 "No effect when internals are visible and \
//                  stickers have some spacing between them.",
//             );

//         prefs_ui.color("Default", access!(.default_color));
//         prefs_ui.color("Hidden", access!(.hidden_color));
//         prefs_ui.color("Hovered", access!(.hovered_color));
//         prefs_ui.color("Sel. sticker", access!(.selected_sticker_color));
//         prefs_ui.color("Sel. piece", access!(.selected_piece_color));
//     });

//     prefs_ui.collapsing("Outline sizes", |mut prefs_ui| {
//         fn outline_size_dv(drag_value: egui::DragValue<'_>) -> egui::DragValue<'_> {
//             drag_value
//                 .fixed_decimals(1)
//                 .clamp_range(0.0..=5.0_f32)
//                 .speed(0.01)
//         }
//         prefs_ui.num("Default", access!(.default_size), outline_size_dv);
//         prefs_ui.num("Hidden", access!(.hidden_size), outline_size_dv);
//         prefs_ui.num("Hovered", access!(.hovered_size), outline_size_dv);
//         prefs_ui.num("Selected", access!(.selected_size), outline_size_dv);
//     });

//     prefs.needs_save |= changed;
//     if changed {
//         app.request_redraw_puzzle();
//     }
// }
// pub fn build_opacity_section(ui: &mut egui::Ui, app: &mut App) {
//     let prefs = &mut app.prefs;

//     let mut changed = false;
//     let mut prefs_ui = PrefsUi {
//         ui,
//         current: &mut prefs.opacity,
//         defaults: &DEFAULT_PREFS.opacity,
//         changed: &mut changed,
//     };

//     prefs_ui.percent("Base", access!(.base));
//     prefs_ui.percent("Ungripped", access!(.ungripped));
//     prefs_ui.percent("Hidden", access!(.hidden));
//     prefs_ui.percent("Selected", access!(.selected));
//     build_unhide_grip_checkbox(&mut prefs_ui);

//     prefs.needs_save |= changed;
//     if changed {
//         app.request_redraw_puzzle();
//     }
// }

pub fn build_view_section(
    view_prefs_set: PuzzleViewPreferencesSet,
    mut prefs_ui: PrefsUi<'_, ViewPreferences>,
) {
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

        if view_prefs_set == PuzzleViewPreferencesSet::Dim4D {
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
        prefs_ui.checkbox("Show frontfaces", access!(.show_frontfaces));
        prefs_ui.checkbox("Show backfaces", access!(.show_backfaces));
        if view_prefs_set == PuzzleViewPreferencesSet::Dim4D {
            prefs_ui.checkbox("Show behind 4D camera", access!(.show_behind_4d_camera));
        } else {
            prefs_ui.current.show_behind_4d_camera = false;
        }

        if view_prefs_set == PuzzleViewPreferencesSet::Dim3D {
            prefs_ui.checkbox("Show internals", access!(.show_internals));
        } else {
            prefs_ui.current.show_internals = false;
        }
        let showing_internals = prefs_ui.current.show_internals;

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

    prefs_ui.ui.add_space(prefs_ui.ui.spacing().item_spacing.y);
}
