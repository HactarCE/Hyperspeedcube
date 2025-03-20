use std::ops::RangeInclusive;

use egui::NumExt;
use hyperprefs::{
    AnimationPreferences, InteractionPreferences, InterpolateFn, Preferences, StyleColorMode,
    ViewPreferences,
};
use hyperpuzzle_core::{PerspectiveDim, PuzzleViewPreferencesSet, Rgb};
use strum::VariantArray;

use crate::L;
use crate::gui::components::WidgetWithReset;
use crate::gui::ext::*;
use crate::gui::util::Access;
use crate::locales::HoverStrings;

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
impl<T> PrefsUi<'_, T> {
    fn get_default<U: Clone>(&self, access: &Access<T, U>) -> Option<U> {
        Some(access.get(self.defaults.as_ref()?).clone())
    }

    pub fn map_prefs<U>(&mut self, access: Access<T, U>) -> PrefsUi<'_, U> {
        PrefsUi {
            ui: self.ui,
            current: access.get_mut(self.current),
            defaults: self.defaults.map(|defaults| access.get(defaults)),
            changed: self.changed,
        }
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
        explanation: &str,
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

    pub fn collapsing<R>(
        &mut self,
        title: impl Into<egui::WidgetText>,
        add_contents: impl FnOnce(PrefsUi<'_, T>) -> R,
    ) -> egui::CollapsingResponse<R> {
        let (mut prefs, ui) = self.split();
        egui::CollapsingHeader::new(title)
            .default_open(true)
            .show(ui, |ui| add_contents(prefs.with(ui)))
    }

    pub fn checkbox(&mut self, strings: &HoverStrings, access: Access<T, bool>) -> egui::Response {
        let reset_value = self.get_default(&access);
        self.add(|current| WidgetWithReset {
            label: "".into(),
            value: access.get_mut(current),
            reset_value,
            reset_value_str: None,
            make_widget: |value| egui::Checkbox::new(value, strings.label),
        })
        .on_i18n_hover_explanation(strings)
    }

    pub fn num<N: egui::emath::Numeric + ToString>(
        &mut self,
        strings: &HoverStrings,
        access: Access<T, N>,
        modify_widget: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
    ) -> egui::Response {
        let reset_value = self.get_default(&access);
        let reset_value_str = reset_value.as_ref().map(|v| v.to_string().into());
        self.add(|current| WidgetWithReset {
            label: strings.label.into(),
            value: access.get_mut(current),
            reset_value,
            reset_value_str,
            make_widget: |value| modify_widget(egui::DragValue::new(value)),
        })
        .on_i18n_hover_explanation(strings)
    }

    pub fn percent(&mut self, strings: &HoverStrings, access: Access<T, f32>) -> egui::Response {
        let reset_value = self.get_default(&access);
        let reset_value_str = reset_value
            .as_ref()
            .map(|v| format!("{}%", v * 100.0).into());
        self.add(|current| WidgetWithReset {
            label: strings.label.into(),
            value: access.get_mut(current),
            reset_value,
            reset_value_str,
            make_widget: drag_value_percent,
        })
        .on_i18n_hover_explanation(strings)
    }

    pub fn animation_duration(
        &mut self,
        strings: &HoverStrings,
        access: Access<T, f32>,
    ) -> egui::Response {
        let range = 0.0..=5.0_f32;
        let speed = access.get(self.current).at_least(0.1) / 100.0; // logarithmic speed
        self.num(strings, access, |dv| {
            dv.fixed_decimals(2).range(range).speed(speed)
        })
    }

    pub fn angle(
        &mut self,
        strings: &HoverStrings,
        access: Access<T, f32>,
        modify_widget: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
    ) -> egui::Response {
        self.angle_with_raw_label(strings.label, access, modify_widget)
            .on_i18n_hover_explanation(strings)
    }
    pub fn angle_with_raw_label(
        &mut self,
        label: impl Into<egui::WidgetText>,
        access: Access<T, f32>,
        modify_widget: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
    ) -> egui::Response {
        let reset_value = self.get_default(&access);
        let reset_value_str = reset_value.map(|v| format!("{v}°").into());
        self.add(|current| WidgetWithReset {
            label: label.into(),
            value: access.get_mut(current),
            reset_value,
            reset_value_str,
            make_widget: |value| {
                modify_widget(egui::DragValue::new(value).suffix("°").fixed_decimals(0))
            },
        })
    }

    pub fn color(&mut self, strings: &HoverStrings, access: Access<T, Rgb>) -> egui::Response {
        self.color_with_label(strings.label, access)
            .on_i18n_hover_explanation(strings)
    }

    pub fn color_with_label(
        &mut self,
        label: impl Into<egui::WidgetText>,
        access: Access<T, Rgb>,
    ) -> egui::Response {
        let reset_value = self.get_default(&access);
        let reset_value_str = reset_value.as_ref().map(|v| v.to_string().into());
        self.add(|current| WidgetWithReset {
            label: label.into(),
            value: access.get_mut(current),
            reset_value,
            reset_value_str,
            make_widget: |value| |ui: &mut egui::Ui| super::color_edit(ui, value, None::<fn()>),
        })
    }

    pub fn fixed_multi_color(
        &mut self,
        label: impl Into<egui::WidgetText>,
        access: Access<T, Vec<Rgb>>,
    ) -> egui::Response {
        let reset_value = self.get_default(&access);
        self.add(|current| WidgetWithReset {
            label: label.into(),
            value: access.get_mut(current),
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
            label: "".into(),
            value: access.get_mut(current),
            reset_value,
            reset_value_str: reset_value.map(|v| match v {
                Some(mode) => match mode {
                    StyleColorMode::FromSticker => L.styles.color_mode_reset.sticker.into(),
                    StyleColorMode::FixedColor { color } => {
                        let rgb = color.to_string();
                        L.styles.color_mode_reset.fixed.with(&rgb).into()
                    }
                    StyleColorMode::Rainbow => L.styles.color_mode_reset.rainbow.into(),
                },
                None => L.styles.color_mode_reset.default.into(),
            }),
            make_widget: |value| {
                move |ui: &mut egui::Ui| {
                    let mut changed = false;

                    let id = ui.next_auto_id();
                    ui.skip_ahead_auto_ids(1);

                    if !allow_fallthrough {
                        value.get_or_insert(StyleColorMode::FromSticker);
                    }
                    let l = L.styles.color_mode;
                    let mut r = ui.horizontal(|ui| {
                        // Assemble list of options
                        let mut options = vec![];
                        {
                            if allow_fallthrough {
                                options.push((None, l.default.into()));
                            }

                            if allow_from_sticker_color {
                                let option = Some(StyleColorMode::FromSticker);
                                options.push((option, l.sticker.into()));
                            }

                            let color = value.and_then(|v| v.fixed_color()).unwrap_or_default();
                            let option = Some(StyleColorMode::FixedColor { color });
                            options.push((option, l.fixed.into()));

                            let option = Some(StyleColorMode::Rainbow);
                            options.push((option, l.rainbow.into()));
                        }

                        let r = ui.add(crate::gui::components::FancyComboBox {
                            combo_box: egui::ComboBox::from_id_salt(id),
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

    pub fn interpolation_fn(
        &mut self,
        strings: &HoverStrings,
        access: Access<T, InterpolateFn>,
    ) -> egui::Response {
        /// Returns the human-friendly strings for the interpolation function.
        fn get_strings(f: InterpolateFn) -> &'static HoverStrings {
            let l = &L.prefs.animations.twists.interpolations;
            match f {
                InterpolateFn::Lerp => &l.lerp,
                InterpolateFn::Cosine => &l.cosine,
                InterpolateFn::Cubic => &l.cubic,
                InterpolateFn::Circular => &l.circular,
                InterpolateFn::Bounce => &l.bounce,
                InterpolateFn::Overshoot => &l.overshoot,
                InterpolateFn::Underdamped => &l.underdamped,
                InterpolateFn::CriticallyDamped => &l.critically_damped,
                InterpolateFn::CriticallyDried => &l.critically_dried,
                InterpolateFn::Random => &l.random,
            }
        }

        /// Returns the D&D alignment of the interpolation function.
        pub fn get_dnd_alignment(f: InterpolateFn) -> &'static str {
            let l = &L.prefs.animations.twists.interpolations.alignments;
            match f {
                InterpolateFn::Lerp => l.true_neutral,
                InterpolateFn::Cosine => l.neutral_good,
                InterpolateFn::Cubic => l.lawful_neutral,
                InterpolateFn::Circular => l.neutral_evil,
                InterpolateFn::Bounce => l.chaotic_neutral,
                InterpolateFn::Overshoot => l.chaotic_good,
                InterpolateFn::Underdamped => l.lawful_evil,
                InterpolateFn::CriticallyDamped => l.lawful_good,
                InterpolateFn::CriticallyDried => l.chaotic_evil,
                InterpolateFn::Random => l.eldritch,
            }
        }

        let reset_value = self.get_default(&access);
        self.add(|current| WidgetWithReset {
            label: strings.label.into(),
            value: access.get_mut(current),
            reset_value,
            reset_value_str: reset_value.map(|v| get_strings(v).label.into()),
            make_widget: |value| {
                move |ui: &mut egui::Ui| {
                    let mut changed = false;

                    let id = ui.next_auto_id();
                    ui.skip_ahead_auto_ids(1);
                    let mut r = egui::ComboBox::from_id_salt(id)
                        .width_to_fit(
                            ui,
                            InterpolateFn::VARIANTS
                                .iter()
                                .map(|f| get_strings(*f).label),
                        )
                        .selected_text(get_strings(*value).label)
                        .show_ui(ui, |ui| {
                            for &f in InterpolateFn::VARIANTS {
                                let desc = get_strings(f).desc;
                                let alignment_str = L
                                    .prefs
                                    .animations
                                    .twists
                                    .interpolations
                                    .alignment
                                    .with(get_dnd_alignment(f));
                                if ui
                                    .selectable_label(*value == f, get_strings(f).label)
                                    .on_hover_explanation("", format!("{alignment_str}\n\n{desc}"))
                                    .clicked()
                                {
                                    *value = f;
                                    changed = true;
                                }
                            }
                        })
                        .response;

                    if changed {
                        r.mark_changed();
                    }

                    r
                }
            },
        })
        .on_i18n_hover_explanation(strings)
    }
}

pub fn build_interaction_section(mut prefs_ui: PrefsUi<'_, InteractionPreferences>) {
    let l = &L.prefs.interaction.dialogs;
    prefs_ui.collapsing(l.title, |mut prefs_ui| {
        prefs_ui.checkbox(
            &l.confirm_discard_only_when_scrambled,
            access!(.confirm_discard_only_when_scrambled),
        );
    });

    let l = &L.prefs.interaction.reorientation;
    prefs_ui.collapsing(l.title, |mut prefs_ui| {
        prefs_ui.num(&l.drag_sensitivity, access!(.drag_sensitivity), |dv| {
            dv.fixed_decimals(2).range(0.0..=3.0_f32).speed(0.01)
        });
        prefs_ui.checkbox(&l.realign_puzzle_on_release, access!(.realign_on_release));
        prefs_ui.checkbox(&l.realign_puzzle_on_keypress, access!(.realign_on_keypress));
        prefs_ui.checkbox(&l.smart_realign, access!(.smart_realign));
    });

    let l = &L.prefs.interaction.ui;
    prefs_ui.collapsing(l.title, |mut prefs_ui| {
        prefs_ui.checkbox(&l.middle_click_delete, access!(.middle_click_delete));
        prefs_ui.checkbox(&l.reverse_filter_rules, access!(.reverse_filter_rules));
    });
}
pub fn build_animation_section(mut prefs_ui: PrefsUi<'_, AnimationPreferences>) {
    let l = &L.prefs.animations.twists;
    prefs_ui.collapsing(l.title, |mut prefs_ui| {
        prefs_ui.checkbox(&l.dynamic_twist_speed, access!(.dynamic_twist_speed));
        prefs_ui.animation_duration(&l.twist_duration, access!(.twist_duration));
        prefs_ui.interpolation_fn(&l.twist_interpolation, access!(.twist_interpolation));
    });
    let l = &L.prefs.animations.other;
    prefs_ui.collapsing(l.title, |mut prefs_ui| {
        prefs_ui.animation_duration(
            &l.blocking_animation_duration,
            access!(.blocking_anim_duration),
        );
    });
}

pub fn build_perspective_dim_view_section(
    dim: PerspectiveDim,
    mut prefs_ui: PrefsUi<'_, ViewPreferences>,
) {
    let l = &L.prefs.view.projection;
    prefs_ui.collapsing(l.title, |mut prefs_ui| {
        if dim == PerspectiveDim::Dim4D {
            prefs_ui.angle(&l.fov_4d, access!(.fov_4d), |dv| {
                dv.range(FOV_4D_RANGE).speed(0.5)
            });
        }

        prefs_ui.angle_with_raw_label(fov_3d_label(&prefs_ui), access!(.fov_3d), |dv| {
            dv.range(FOV_3D_RANGE).speed(0.5)
        });
    });

    let l = &L.prefs.view.geometry;
    prefs_ui.collapsing(l.title, |mut prefs_ui| {
        prefs_ui.checkbox(&l.show_frontfaces, access!(.show_frontfaces));
        prefs_ui.checkbox(&l.show_backfaces, access!(.show_backfaces));
        if dim == PerspectiveDim::Dim4D {
            prefs_ui.checkbox(&l.show_behind_4d_camera, access!(.show_behind_4d_camera));
        } else {
            prefs_ui.current.show_behind_4d_camera = false;
        }

        if dim == PerspectiveDim::Dim3D {
            prefs_ui.checkbox(&l.show_internals, access!(.show_internals));
        } else {
            prefs_ui.current.show_internals = false;
        }
        let showing_internals = prefs_ui.current.show_internals;

        prefs_ui.num(&l.gizmo_scale, access!(.gizmo_scale), |dv| {
            dv.fixed_decimals(2).range(0.1..=5.0_f32).speed(0.01)
        });
        prefs_ui.add_enabled_ui(
            !showing_internals,
            l.disabled_when_showing_internals,
            |mut prefs_ui| {
                prefs_ui.num(&l.facet_shrink, access!(.facet_shrink), |dv| {
                    dv.fixed_decimals(2).range(0.0..=0.95_f32).speed(0.005)
                })
            },
        );
        prefs_ui.add_enabled_ui(
            !showing_internals,
            l.disabled_when_showing_internals,
            |mut prefs_ui| {
                prefs_ui.num(&l.sticker_shrink, access!(.sticker_shrink), |dv| {
                    dv.fixed_decimals(2).range(0.0..=0.95_f32).speed(0.005)
                })
            },
        );

        prefs_ui.num(&l.piece_explode, access!(.piece_explode), |dv| {
            dv.fixed_decimals(2).range(0.0..=5.0_f32).speed(0.01)
        });
    });

    let l = &L.prefs.view.lighting;
    prefs_ui.collapsing(l.title, |mut prefs_ui| {
        prefs_ui.angle(&l.pitch, access!(.light_pitch), |dv| dv.range(-90.0..=90.0));
        prefs_ui.angle(&l.yaw, access!(.light_yaw), |dv| dv.range(-180.0..=180.0));
        prefs_ui.percent(&l.intensity.faces, access!(.face_light_intensity));
        prefs_ui.percent(&l.intensity.outlines, access!(.outline_light_intensity));
    });

    let l = &L.prefs.view.performance;
    prefs_ui.collapsing(l.title, |mut prefs_ui| {
        prefs_ui.num(&l.downscale_factor, access!(.downscale_rate), |dv| {
            dv.range(1..=32).speed(0.1)
        });
        prefs_ui.checkbox(&l.downscale_interpolation, access!(.downscale_interpolate));
    });

    prefs_ui.ui.add_space(prefs_ui.ui.spacing().item_spacing.y);
}

fn fov_3d_label(prefs_ui: &PrefsUi<'_, ViewPreferences>) -> &'static str {
    if prefs_ui.current.fov_3d == *FOV_3D_RANGE.start() {
        L.prefs.view.projection.fov_3d.orp_ekauq
    } else if prefs_ui.current.fov_3d == *FOV_3D_RANGE.end() {
        L.prefs.view.projection.fov_3d.quake_pro
    } else {
        L.prefs.view.projection.fov_3d.label
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
    .range(0.0..=100.0_f32)
    .speed(0.5)
}
