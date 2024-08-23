use strum::{EnumIter, IntoEnumIterator};

use crate::{
    app::App,
    gui::{
        self,
        markdown::{md, md_bold_user_text},
        util::EguiTempValue,
    },
    preferences::{PieceStyle, Preset, StylePreferences, DEFAULT_PREFS},
    L,
};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    let default_outline_lighting = app.prefs.styles.default.outline_lighting.unwrap_or(false);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.strong(L.styles.misc.title);
        });
        ui.separator();
        let mut prefs_ui = crate::gui::components::PrefsUi {
            ui,
            current: &mut app.prefs.styles,
            defaults: Some(&DEFAULT_PREFS.styles),
            changed: &mut changed,
        };
        let l = L.styles.misc.background;
        prefs_ui.collapsing(l.title, |mut prefs_ui| {
            prefs_ui.color(&l.dark_mode, access!(.dark_background_color));
            prefs_ui.color(&l.light_mode, access!(.light_background_color));
        });
        let l = L.styles.misc.internals;
        prefs_ui.collapsing(l.title, |mut prefs_ui| {
            prefs_ui.color(&l.face_color, access!(.internals_color));
        });
        let l = L.styles.misc.blocking_pieces;
        prefs_ui.collapsing(l.title, |mut prefs_ui| {
            prefs_ui.color(&l.outlines_color, access!(.blocking_outline_color));
            prefs_ui.num(&l.outlines_size, access!(.blocking_outline_size), |dv| {
                outline_size_drag_value(dv)
            });
        });
    });

    ui.add_space(ui.spacing().item_spacing.x);

    ui.group(|ui| {
        ui.strong(L.styles.builtin.title);
        ui.add_space(ui.spacing().item_spacing.y);
        let (name, piece_style_edit) = show_builtin_style_selector(ui, &mut app.prefs.styles);
        ui.add_space(ui.spacing().item_spacing.y);
        ui.separator();
        ui.add_space(ui.spacing().item_spacing.y);
        md(
            ui,
            L.presets
                .custom_styles
                .current
                .with(&md_bold_user_text(&name)),
        );
        changed |= ui.add(piece_style_edit).changed();
    });

    ui.add_space(ui.spacing().item_spacing.x);

    let help_contents = L.help.custom_piece_styles;
    let presets_ui = gui::components::PresetsUi {
        id: unique_id!(),
        presets: &mut app.prefs.styles.custom,
        changed: &mut changed,
        text: &L.presets.custom_styles,
        autosave: true,
        vscroll: false,
        help_contents: Some(&help_contents),
        extra_validation: Some(Box::new(|_, name| {
            if name == crate::DEFAULT_STYLE_NAME {
                Err(L.presets.custom_styles.errors.name_conflict.into())
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
    presets_ui.show(ui, None, get_backup_defaults, |mut prefs_ui| {
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
        fn name(self) -> &'static str {
            let l = L.styles.builtin;
            match self {
                BuiltInStyle::Default => l.default,
                BuiltInStyle::Gripped => l.gripped,
                BuiltInStyle::Ungripped => l.ungripped,
                BuiltInStyle::Hovered => l.hovered,
                BuiltInStyle::Selected => l.selected,
                BuiltInStyle::Blindfolded => l.blindfolded,
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
            };

            let l = &L.styles.custom;
            prefs_ui.collapsing(l.sticker_faces, |mut prefs_ui| {
                prefs_ui.checkbox(&l.interactable, access_option!(true, .interactable));
                prefs_ui.percent(&l.opacity, access_option!(1.0, .face_opacity));

                prefs_ui.color_mode(
                    access!(.face_color),
                    self.allow_fallthrough,
                    self.allow_from_sticker_color,
                );
            });

            prefs_ui.collapsing(l.sticker_outlines, |mut prefs_ui| {
                prefs_ui.percent(&l.opacity, access_option!(1.0, .outline_opacity));
                prefs_ui.num(&l.outline_size, access_option!(1.0, .outline_size), |dv| {
                    outline_size_drag_value(dv)
                });

                prefs_ui.color_mode(
                    access!(.outline_color),
                    self.allow_fallthrough,
                    self.allow_from_sticker_color,
                );

                let (mut prefs, ui) = prefs_ui.split();
                prefs.with(ui).checkbox(
                    &l.lighting,
                    match self.default_lighting {
                        true => access_option!(true, .outline_lighting),
                        false => access_option!(false, .outline_lighting),
                    },
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
    dv.fixed_decimals(1).range(0.0..=5.0).speed(0.01)
}
