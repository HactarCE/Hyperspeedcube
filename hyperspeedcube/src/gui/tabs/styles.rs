use strum::{EnumIter, IntoEnumIterator};

use crate::{
    app::App,
    gui::{
        self,
        ext::ResponseExt,
        util::{set_widget_spacing_to_space_width, EguiTempValue},
    },
    preferences::{PieceStyle, Preset, StylePreferences, DEFAULT_PREFS},
};

fn show_custom_piece_styles_help_ui(ui: &mut egui::Ui) {
    // TODO: markdown renderer
    ui.spacing_mut().item_spacing.y = 9.0;
    ui.heading("Custom piece styles");
    ui.horizontal_wrapped(|ui| {
        set_widget_spacing_to_space_width(ui);
        ui.label("Custom styles can be applied to pieces using the");
        ui.strong("piece filters");
        ui.label("tool.");
    });
}

// TODO: markdown bold ("piece explode" and "view settings")
pub const INTERNAL_FACES_COLOR_EXPLANATION: &str = "For 3D puzzles, it's sometimes possible to \
                                                    view the internal faces of pieces, particularly \
                                                    mid-turn or using piece explode. You can \
                                                    configure whether internal faces are visible \
                                                    in view settings.";
pub const BLOCKING_PIECES_OUTLINES_COLOR: &str = "Outline color for pieces blocking a move. \
                                                  This is only visible for puzzles that bandage.";

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    let default_outline_lighting = app.prefs.styles.default.outline_lighting.unwrap_or(false);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.strong("Miscellaneous styling");
        });
        ui.separator();
        let mut prefs_ui = crate::gui::components::PrefsUi {
            ui,
            current: &mut app.prefs.styles,
            defaults: Some(&DEFAULT_PREFS.styles),
            changed: &mut changed,
        };
        prefs_ui.collapsing("Background", |mut prefs_ui| {
            prefs_ui.color("Dark mode", access!(.dark_background_color));
            prefs_ui.color("Light mode", access!(.light_background_color));
        });
        prefs_ui.collapsing("Internals", |mut prefs_ui| {
            prefs_ui
                .color("Face color", access!(.internals_color))
                .on_hover_explanation("Internal faces color", INTERNAL_FACES_COLOR_EXPLANATION);
        });
        prefs_ui.collapsing("Blocking pieces", |mut prefs_ui| {
            prefs_ui
                .color("Outlines color", access!(.blocking_outline_color))
                .on_hover_explanation(
                    "Blocking pieces outlines color",
                    BLOCKING_PIECES_OUTLINES_COLOR,
                );
            prefs_ui
                .num("Outlines size", access!(.blocking_outline_size), |dv| {
                    outline_size_drag_value(dv)
                })
                .on_hover_explanation(
                    "Blocking pieces outlines size",
                    "Outline size for pieces blocking a move. \
                     This is only visible for puzzles that bandage.",
                );
        });
    });

    ui.add_space(ui.spacing().item_spacing.x);

    ui.group(|ui| {
        ui.horizontal(|ui| ui.strong("Built-in styles"));
        ui.add_space(ui.spacing().item_spacing.y);
        let (name, piece_style_edit) = show_builtin_style_selector(ui, &mut app.prefs.styles);
        ui.add_space(ui.spacing().item_spacing.y);
        ui.separator();
        ui.add_space(ui.spacing().item_spacing.y);
        ui.horizontal(|ui| {
            set_widget_spacing_to_space_width(ui);
            ui.strong(name);
            ui.label("style");
        });
        changed |= ui.add(piece_style_edit).changed();
    });

    ui.add_space(ui.spacing().item_spacing.x);

    let presets_ui = gui::components::PresetsUi {
        id: unique_id!(),
        presets: &mut app.prefs.styles.custom,
        changed: &mut changed,
        text: gui::components::PresetsUiText {
            presets_set: None,
            preset: "style",
            saved_presets: "Custom styles",
            what: "style",
        },
        autosave: true,
        vscroll: false,
        help_contents: Some(Box::new(show_custom_piece_styles_help_ui)),
        extra_validation: Some(Box::new(|_, name| {
            if name == crate::DEFAULT_STYLE_NAME {
                Err("There is already a style with this name".to_string())
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
    presets_ui.show(ui, get_backup_defaults, |mut prefs_ui| {
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
) -> (&'static str, PieceStyleEdit<'a>) {
    let default_outline_lighting = style_prefs.default.outline_lighting.unwrap_or(false);

    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, EnumIter, IntoStaticStr)]
    enum CurrentStyle {
        #[default]
        Default,
        Gripped,
        Ungripped,
        Hovered,
        Selected,
        Blindfolded,
    }
    let selected_style_tmp_val = EguiTempValue::new(ui);
    let mut selected_style = selected_style_tmp_val.get().unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        for e in CurrentStyle::iter() {
            ui.selectable_value(&mut selected_style, e, <&str>::from(e));
        }
    });
    selected_style_tmp_val.set(Some(selected_style));
    let current = match selected_style {
        CurrentStyle::Default => &mut style_prefs.default,
        CurrentStyle::Gripped => &mut style_prefs.gripped,
        CurrentStyle::Ungripped => &mut style_prefs.ungripped,
        CurrentStyle::Hovered => &mut style_prefs.hovered_piece,
        CurrentStyle::Selected => &mut style_prefs.selected_piece,
        CurrentStyle::Blindfolded => &mut style_prefs.blind,
    };
    let default = match selected_style {
        CurrentStyle::Default => &DEFAULT_PREFS.styles.default,
        CurrentStyle::Gripped => &DEFAULT_PREFS.styles.gripped,
        CurrentStyle::Ungripped => &DEFAULT_PREFS.styles.ungripped,
        CurrentStyle::Hovered => &DEFAULT_PREFS.styles.hovered_piece,
        CurrentStyle::Selected => &DEFAULT_PREFS.styles.selected_piece,
        CurrentStyle::Blindfolded => &DEFAULT_PREFS.styles.blind,
    };
    let mut piece_style_edit = PieceStyleEdit::new(current).reset_value(default);
    match selected_style {
        CurrentStyle::Default => piece_style_edit = piece_style_edit.no_fallthrough(),
        CurrentStyle::Blindfolded => piece_style_edit = piece_style_edit.blind(),
        _ => (),
    }
    (
        <&str>::from(selected_style),
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
