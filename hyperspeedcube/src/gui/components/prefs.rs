use std::ops::RangeInclusive;

use egui::NumExt;
use hyperpuzzle::Rgb;

use crate::gui::components::WidgetWithReset;
use crate::gui::ext::*;
use crate::gui::util::Access;
use crate::preferences::{
    InteractionPreferences, PuzzleViewPreferencesSet, StyleColorMode, ViewPreferences,
};

const FOV_4D_RANGE: RangeInclusive<f32> = -5.0..=120.0;
const FOV_3D_RANGE: RangeInclusive<f32> = -120.0..=120.0;

pub struct PartialPrefsUi<'a, T> {
    pub current: &'a mut T,
    pub defaults: Option<&'a T>,
    pub changed: &'a mut bool,
}
impl<'a, T> PartialPrefsUi<'a, T> {
    pub fn with<'b>(&'b mut self, ui: &'b mut egui::Ui) -> PrefsUi<'b, T>
    where
        'a: 'b,
    {
        PrefsUi {
            ui,
            current: self.current,
            defaults: self.defaults,
            changed: self.changed,
        }
    }
}

pub struct PrefsUi<'a, T> {
    pub ui: &'a mut egui::Ui,
    pub current: &'a mut T,
    pub defaults: Option<&'a T>,
    pub changed: &'a mut bool,
}
impl<'a, T> PrefsUi<'a, T> {
    fn get_default<U: Clone>(&self, access: &Access<T, U>) -> Option<U> {
        Some((access.get_ref)(self.defaults.as_ref()?).clone())
    }

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
        let (mut prefs, ui) = self.split();
        ui.add_enabled_ui(enabled, |ui| {
            ui.vertical(|ui| add_contents(prefs.with(ui)))
                .response
                .on_disabled_hover_text(explanation);
        });
    }

    /// Removes the `&mut egui::Ui` so that the same preferences can be modified
    /// in different UI scopes. Use [`PartialPrefsUi::with()`] to recombine it.
    pub fn split(&mut self) -> (PartialPrefsUi<'_, T>, &mut egui::Ui) {
        let partial = PartialPrefsUi {
            current: self.current,
            defaults: self.defaults,
            changed: self.changed,
        };
        (partial, self.ui)
    }

    pub fn group<R>(
        &mut self,
        add_contents: impl FnOnce(PrefsUi<'_, T>) -> R,
    ) -> egui::InnerResponse<R> {
        let (mut prefs, ui) = self.split();
        ui.group(|ui| add_contents(prefs.with(ui)))
    }
    pub fn collapsing<R>(
        &mut self,
        heading: impl Into<egui::WidgetText>,
        add_contents: impl FnOnce(PrefsUi<'_, T>) -> R,
    ) -> egui::CollapsingResponse<R> {
        let (mut prefs, ui) = self.split();
        egui::CollapsingHeader::new(heading)
            .default_open(true)
            .show(ui, |ui| add_contents(prefs.with(ui)))
    }

    pub fn checkbox(&mut self, label: &str, access: Access<T, bool>) -> egui::Response {
        let reset_value = self.get_default(&access);
        self.add(|current| WidgetWithReset {
            label: "",
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str: None,
            make_widget: |value| egui::Checkbox::new(value, label),
        })
    }

    pub fn num<N: egui::emath::Numeric + ToString>(
        &mut self,
        label: &str,
        access: Access<T, N>,
        modify_widget: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
    ) -> egui::Response {
        let reset_value = self.get_default(&access);
        let reset_value_str = reset_value.as_ref().map(|v| v.to_string());
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str,
            make_widget: |value| modify_widget(egui::DragValue::new(value)),
        })
    }

    pub fn percent(&mut self, label: &str, access: Access<T, f32>) -> egui::Response {
        let reset_value = self.get_default(&access);
        let reset_value_str = reset_value.as_ref().map(|v| v.to_string());
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str,
            make_widget: drag_value_percent,
        })
    }

    pub fn angle(
        &mut self,
        label: &str,
        access: Access<T, f32>,
        modify_widget: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
    ) -> egui::Response {
        let reset_value = self.get_default(&access);
        let reset_value_str = reset_value.map(|v| format!("{v}°"));
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

    pub fn color(&mut self, label: &str, access: Access<T, Rgb>) -> egui::Response {
        let reset_value = self.get_default(&access);
        let reset_value_str = reset_value.as_ref().map(|v| v.to_string());
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str,
            make_widget: |value| |ui: &mut egui::Ui| super::color_edit(ui, value, None::<fn()>),
        })
    }

    pub fn fixed_multi_color(
        &mut self,
        label: &str,
        access: Access<T, Vec<Rgb>>,
    ) -> egui::Response {
        let reset_value = self.get_default(&access);
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str: None,
            make_widget: |values| {
                |ui: &mut egui::Ui| {
                    let mut changed = false;
                    let mut r = ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = ui.spacing().item_spacing.y;
                        for value in values {
                            changed |= super::color_edit(ui, value, None::<fn()>).changed();
                        }
                    });
                    if changed {
                        r.response.mark_changed();
                    }
                    r.response
                }
            },
        })
    }

    pub fn color_mode(
        &mut self,
        access: Access<T, Option<StyleColorMode>>,
        allow_fallthrough: bool,
        allow_from_sticker_color: bool,
    ) -> egui::Response {
        let reset_value = self.get_default(&access);
        self.add(|current| WidgetWithReset {
            label: "",
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str: reset_value.map(|v| match v {
                Some(mode) => match mode {
                    StyleColorMode::FromSticker => "sticker color".to_string(),
                    StyleColorMode::FixedColor { color } => format!("fixed color {color}"),
                },
                None => "default color".to_string(),
            }),
            make_widget: |value| {
                move |ui: &mut egui::Ui| {
                    let mut changed = false;

                    let id = ui.next_auto_id();
                    ui.skip_ahead_auto_ids(1);

                    if !allow_fallthrough {
                        value.get_or_insert(StyleColorMode::FromSticker);
                    }
                    let mut r = ui.horizontal(|ui| {
                        // Assemble list of options
                        let mut options = vec![];
                        {
                            if allow_fallthrough {
                                options.push((None, "Default color".into()));
                            }

                            if allow_from_sticker_color {
                                let option = Some(StyleColorMode::FromSticker);
                                options.push((option, "Sticker color".into()));
                            }

                            let color = value.and_then(|v| v.fixed_color()).unwrap_or_default();
                            let option = Some(StyleColorMode::FixedColor { color });
                            options.push((option, "Fixed color".into()));
                        }

                        let r = ui.add(crate::gui::components::FancyComboBox {
                            combo_box: egui::ComboBox::from_id_source(id),
                            selected: value,
                            options,
                        });
                        changed |= r.changed();

                        if let Some(StyleColorMode::FixedColor { color }) = value {
                            let r = crate::gui::components::color_edit(ui, color, None::<fn()>);
                            changed |= r.changed();
                        }
                    });

                    if changed {
                        r.response.mark_changed();
                    }
                    r.response
                }
            },
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
    prefs_ui.collapsing("Dialogs", |mut prefs_ui| {
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
    });

    prefs_ui.collapsing("Reorientation", |mut prefs_ui| {
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
    });

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
    prefs_ui.collapsing("Projection", |mut prefs_ui| {
        if view_prefs_set == PuzzleViewPreferencesSet::Dim4D {
            prefs_ui.angle("4D FOV", access!(.fov_4d), |dv| {
                dv.clamp_range(FOV_4D_RANGE).speed(0.5)
            });
        }

        prefs_ui.angle(fov_3d_label(&prefs_ui), access!(.fov_3d), |dv| {
            dv.clamp_range(FOV_3D_RANGE).speed(0.5)
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

        if view_prefs_set == PuzzleViewPreferencesSet::Dim4D {
            prefs_ui.num("Gizmo scale", access!(.gizmo_scale), |dv| {
                dv.fixed_decimals(2).clamp_range(0.1..=5.0_f32).speed(0.01)
            });
        }
        prefs_ui.add_enabled_ui(
            !showing_internals,
            "Disabled when showing internals",
            |mut prefs_ui| {
                prefs_ui.num("Facet shrink", access!(.facet_shrink), |dv| {
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
        prefs_ui
            .percent("Intensity (outlines)", access!(.outline_light_intensity))
            .on_hover_explanation(
                "",
                "This is also enabled or disabled for each \
                 style in the style settings. For dark outline \
                 colors, it may have little or no effect.",
            );
    });

    prefs_ui.collapsing("Performance", |mut prefs_ui| {
        prefs_ui.num("Downscale factor", access!(.downscale_rate), |dv| {
            dv.clamp_range(1..=32).speed(0.1)
        });
        prefs_ui.checkbox("Downscale interpolation", access!(.downscale_interpolate));
    });

    prefs_ui.ui.add_space(prefs_ui.ui.spacing().item_spacing.y);
}

fn fov_3d_label(prefs_ui: &PrefsUi<'_, ViewPreferences>) -> &'static str {
    if prefs_ui.current.fov_3d == *FOV_3D_RANGE.start() {
        "ORP EKAUQ"
    } else if prefs_ui.current.fov_3d == *FOV_3D_RANGE.end() {
        "QUAKE PRO"
    } else {
        "3D FOV"
    }
}

pub fn drag_value_percent(value: &'_ mut f32) -> egui::DragValue<'_> {
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
}
