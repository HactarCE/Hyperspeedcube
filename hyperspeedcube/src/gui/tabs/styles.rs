use std::borrow::Cow;

use strum::{EnumIter, IntoEnumIterator};

use crate::{
    app::App,
    gui::{
        self,
        ext::ResponseExt,
        markdown::md,
        util::{set_widget_spacing_to_space_width, EguiTempValue},
    },
    preferences::{PieceStyle, Preset, StylePreferences, DEFAULT_PREFS},
};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    let default_outline_lighting = app.prefs.styles.default.outline_lighting.unwrap_or(false);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.strong(t!("prefs.styles.misc.title"));
        });
        ui.separator();
        let mut prefs_ui = crate::gui::components::PrefsUi {
            ui,
            current: &mut app.prefs.styles,
            defaults: Some(&DEFAULT_PREFS.styles),
            changed: &mut changed,
            i18n_prefix: "prefs.styles.misc",
        };
        prefs_ui.collapsing("background", |mut prefs_ui| {
            prefs_ui.color("dark_mode", access!(.dark_background_color));
            prefs_ui.color("light_mode", access!(.light_background_color));
        });
        prefs_ui.collapsing("internals", |mut prefs_ui| {
            prefs_ui.color("face_color", access!(.internals_color));
        });
        prefs_ui.collapsing("blocking_pieces", |mut prefs_ui| {
            prefs_ui.color("outlines_color", access!(.blocking_outline_color));
            prefs_ui.num("outlines_size", access!(.blocking_outline_size), |dv| {
                outline_size_drag_value(dv)
            });
        });
    });

    ui.add_space(ui.spacing().item_spacing.x);

    ui.group(|ui| {
        ui.strong(t!("prefs.styles.builtin.title"));
        ui.add_space(ui.spacing().item_spacing.y);
        let (name, piece_style_edit) = show_builtin_style_selector(ui, &mut app.prefs.styles);
        ui.add_space(ui.spacing().item_spacing.y);
        ui.separator();
        ui.add_space(ui.spacing().item_spacing.y);
        md(ui, t!("presets.custom_styles._current", current = name));
        changed |= ui.add(piece_style_edit).changed();
    });

    ui.add_space(ui.spacing().item_spacing.x);

    let help_contents = t!("help.custom_piece_styles");
    let presets_ui = gui::components::PresetsUi {
        id: unique_id!(),
        presets: &mut app.prefs.styles.custom,
        changed: &mut changed,
        text: gui::components::PresetsUiText {
            i18n_key: "custom_styles",
            presets_set: None,
        },
        autosave: true,
        vscroll: false,
        help_contents: Some(&help_contents),
        extra_validation: Some(Box::new(|_, name| {
            if name == crate::DEFAULT_STYLE_NAME {
                Err(t!("presets.custom_styles.errors._name_conflict"))
            } else {
                Ok(())
            }
        })),
    };
    let get_backup_defaults = |_| {
        Some(Preset {
            name: crate::DEFAULT_STYLE_NAME.to_string(),
            value: app.prefs.styles.default,
        })
    };
    presets_ui.show(ui, "prefs.styles", get_backup_defaults, |mut prefs_ui| {
        let (prefs, ui) = prefs_ui.split();
        let r = ui.add(
            PieceStyleEdit::new(prefs.current).default_outline_lighting(default_outline_lighting),
        );
        *prefs.changed |= r.changed();
    });

    app.prefs.needs_save |= changed;
}

fn show_builtin_style_selector<'a>(
    ui: &mut egui::Ui,
    style_prefs: &'a mut StylePreferences,
) -> (Cow<'static, str>, PieceStyleEdit<'a>) {
    let default_outline_lighting = style_prefs.default.outline_lighting.unwrap_or(false);

    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, EnumIter)]
    enum BuiltInStyle {
        #[default]
        Default,
        Gripped,
        Ungripped,
        Hovered,
        Selected,
        Blindfolded,
    }
    impl BuiltInStyle {
        fn name(self) -> Cow<'static, str> {
            match self {
                BuiltInStyle::Default => t!("prefs.styles.builtin.default"),
                BuiltInStyle::Gripped => t!("prefs.styles.builtin.gripped"),
                BuiltInStyle::Ungripped => t!("prefs.styles.builtin.ungripped"),
                BuiltInStyle::Hovered => t!("prefs.styles.builtin.hovered"),
                BuiltInStyle::Selected => t!("prefs.styles.builtin.selected"),
                BuiltInStyle::Blindfolded => t!("prefs.styles.builtin.blindfolded"),
            }
        }
    }

    let selected_style_tmp_val = EguiTempValue::new(ui);
    let mut selected_style = selected_style_tmp_val.get().unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        for style in BuiltInStyle::iter() {
            ui.selectable_value(&mut selected_style, style, style.name());
        }
    });
    selected_style_tmp_val.set(Some(selected_style));
    let current = match selected_style {
        BuiltInStyle::Default => &mut style_prefs.default,
        BuiltInStyle::Gripped => &mut style_prefs.gripped,
        BuiltInStyle::Ungripped => &mut style_prefs.ungripped,
        BuiltInStyle::Hovered => &mut style_prefs.hovered_piece,
        BuiltInStyle::Selected => &mut style_prefs.selected_piece,
        BuiltInStyle::Blindfolded => &mut style_prefs.blind,
    };
    let default = match selected_style {
        BuiltInStyle::Default => &DEFAULT_PREFS.styles.default,
        BuiltInStyle::Gripped => &DEFAULT_PREFS.styles.gripped,
        BuiltInStyle::Ungripped => &DEFAULT_PREFS.styles.ungripped,
        BuiltInStyle::Hovered => &DEFAULT_PREFS.styles.hovered_piece,
        BuiltInStyle::Selected => &DEFAULT_PREFS.styles.selected_piece,
        BuiltInStyle::Blindfolded => &DEFAULT_PREFS.styles.blind,
    };
    let mut piece_style_edit = PieceStyleEdit::new(current).reset_value(default);
    match selected_style {
        BuiltInStyle::Default => piece_style_edit = piece_style_edit.no_fallthrough(),
        BuiltInStyle::Blindfolded => piece_style_edit = piece_style_edit.blind(),
        _ => (),
    }
    (
        selected_style.name(),
        piece_style_edit.default_outline_lighting(default_outline_lighting),
    )
}

pub struct PieceStyleEdit<'a> {
    style: &'a mut PieceStyle,
    allow_fallthrough: bool,
    allow_from_sticker_color: bool,
    default_lighting: bool,
    reset_value: Option<&'a PieceStyle>,
}
impl<'a> PieceStyleEdit<'a> {
    pub fn new(style: &'a mut PieceStyle) -> Self {
        Self {
            style,
            allow_fallthrough: true,
            allow_from_sticker_color: true,
            default_lighting: false,
            reset_value: None,
        }
    }
    pub fn blind(mut self) -> Self {
        self.allow_from_sticker_color = false;
        self.no_fallthrough()
    }
    pub fn no_fallthrough(mut self) -> Self {
        self.allow_fallthrough = false;
        self
    }
    pub fn default_outline_lighting(mut self, default_lighting: bool) -> Self {
        self.default_lighting = default_lighting;
        self
    }
    pub fn reset_value(mut self, reset_value: &'a PieceStyle) -> Self {
        self.reset_value = Some(reset_value);
        self
    }
}
impl egui::Widget for PieceStyleEdit<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;

        let mut r = ui.vertical(|ui| {
            let mut prefs_ui = crate::gui::components::PrefsUi {
                ui,
                current: self.style,
                defaults: self.reset_value,
                changed: &mut changed,
                i18n_prefix: "prefs.styles.custom",
            };

            prefs_ui.collapsing("Faces", |mut prefs_ui| {
                prefs_ui.checkbox("Interactable", access_option!(true, .interactable));
                prefs_ui.percent("Opacity", access_option!(1.0, .face_opacity));

                prefs_ui.color_mode(
                    access!(.face_color),
                    self.allow_fallthrough,
                    self.allow_from_sticker_color,
                );
            });

            prefs_ui.collapsing("Outlines", |mut prefs_ui| {
                prefs_ui.percent("Opacity", access_option!(1.0, .outline_opacity));
                prefs_ui.num("Size", access_option!(1.0, .outline_size), |dv| {
                    outline_size_drag_value(dv)
                });

                prefs_ui.color_mode(
                    access!(.outline_color),
                    self.allow_fallthrough,
                    self.allow_from_sticker_color,
                );

                let (mut prefs, ui) = prefs_ui.split();
                prefs
                    .with(ui)
                    .checkbox(
                        "Lighting",
                        match self.default_lighting {
                            true => access_option!(true, .outline_lighting),
                            false => access_option!(false, .outline_lighting),
                        },
                    )
                    .on_hover_explanation(
                        "",
                        "Lighting intensity can be configured in the view settings.", // TODO: markdown renderer
                    );
            });
        });

        if changed {
            r.response.mark_changed();
        }
        r.response
    }
}

fn outline_size_drag_value(dv: egui::DragValue<'_>) -> egui::DragValue<'_> {
    dv.fixed_decimals(1).clamp_range(0.0..=5.0).speed(0.01)
}
