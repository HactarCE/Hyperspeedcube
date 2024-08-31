use egui::Widget;
use float_ord::FloatOrd;
use hyperpuzzle::Rgb;
use itertools::Itertools;
use rand::Rng;
use strum::{EnumIter, IntoEnumIterator};

use crate::app::App;
use crate::gui::components::{
    HelpHoverWidget, PlaintextYamlEditor, PrefsUi, TextEditPopup, TextEditPopupResponse,
    TextValidationResult,
};
use crate::gui::ext::ResponseExt;
use crate::gui::util::EguiTempValue;
use crate::preferences::{GlobalColorPalette, PrefsConvert, DEFAULT_PREFS};
use crate::L;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let yaml = PlaintextYamlEditor::new(ui);

    ui.set_min_width(200.0);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                yaml.show_edit_as_plaintext_button(ui, &app.prefs.color_palette.to_serde());
                HelpHoverWidget::show_right_aligned(ui, L.help.global_color_palette);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.strong(L.colors.global_palette);
                });
            });
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .id_source(ui.id().with(yaml.is_open(ui)))
            .show(ui, |ui| {
                match yaml.is_open(ui) {
                    true => {
                        if let Some(r) = yaml.show(ui) {
                            if r.changed() {
                                // Update value from YAML editor.
                                if let Some(Ok(deserialized)) = yaml.deserialize(ui) {
                                    app.prefs.color_palette.reload_from_serde(&(), deserialized);
                                    app.prefs.needs_save = true;
                                }
                            }
                        }
                    }
                    false => show_contents(ui, app),
                }
            });
    });
}

fn show_contents(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;

    let mut prefs_ui = PrefsUi {
        ui,
        current: &mut app.prefs.styles,
        defaults: Some(&DEFAULT_PREFS.styles),
        changed: &mut changed,
    };

    let l = &L.colors.misc;
    prefs_ui.collapsing(l.title, |mut prefs_ui| {
        prefs_ui.color(&l.dark_background, access!(.dark_background_color));
        prefs_ui.color(&l.light_background, access!(.light_background_color));
        prefs_ui
            .color(&l.internal_faces, access!(.internals_color))
            .on_i18n_hover_explanation(&L.styles.misc.internals.face_color);
        prefs_ui
            .color(
                &l.blocking_pieces_outlines,
                access!(.blocking_outline_color),
            )
            .on_i18n_hover_explanation(&L.styles.misc.blocking_pieces.outlines_color);
    });

    let mut prefs_ui = PrefsUi {
        ui,
        current: &mut app.prefs.color_palette,
        defaults: Some(&DEFAULT_PREFS.color_palette),
        changed: &mut changed,
    };

    prefs_ui.collapsing(L.colors.custom, show_custom_colors_section);
    prefs_ui.collapsing(L.colors.builtin, show_builtin_colors_section);
    prefs_ui.collapsing(L.colors.builtin_sets, show_builtin_color_sets_section);

    app.prefs.needs_save |= changed;
}

fn show_custom_colors_section(mut prefs_ui: PrefsUi<'_, GlobalColorPalette>) {
    let (prefs, ui) = prefs_ui.split();

    let mut dnd = crate::gui::components::DragAndDrop::new(ui);
    let mut to_rename = None;
    let mut to_delete = None;

    ui.horizontal(|ui| {
        if ui.button(L.colors.actions.add).clicked() {
            let custom_colors = &mut prefs.current.custom_colors;
            let name = custom_colors.make_nonconflicting_funny_name();
            let rgb = rand::thread_rng().gen();
            custom_colors.save_preset(name, Rgb { rgb });
            custom_colors.move_index(custom_colors.len() - 1, 0);
            *prefs.changed = true;
        }

        ui.menu_button(L.colors.actions.sort, |ui| {
            let custom_colors = &mut prefs.current.custom_colors;

            let text = L.colors.actions.sort_by_name;
            if ui.button(text).clicked() {
                custom_colors.sort_by_key_or_reverse(|name, _| name.clone());
                *prefs.changed = true;
            }

            let text = L.colors.actions.sort_by_lightness;
            if ui.button(text).clicked() {
                custom_colors.sort_by_key_or_reverse(|_, preset| {
                    FloatOrd(crate::util::rgb_to_oklab(preset.value).l)
                });
                *prefs.changed = true;
            }
        });
    });

    for i in 0..prefs.current.custom_colors.len() {
        let Some((name, _)) = prefs.current.custom_colors.nth_user_preset(i) else {
            log::error!("missing custom color {i}");
            continue;
        };
        let name = name.clone();
        dnd.vertical_reorder_by_handle(ui, name.clone(), |ui, _is_dragging| {
            let (_, preset) = prefs.current.custom_colors.nth_user_preset_mut(i).unwrap();
            let color = &mut preset.value;

            let on_delete = Some(|| to_delete = Some(name.clone()));
            let r = crate::gui::components::color_edit(ui, color, on_delete);
            *prefs.changed |= r.changed();
            let label_response = egui::Label::new(&name)
                .selectable(false)
                .sense(egui::Sense::click())
                .ui(ui);

            let mut popup = TextEditPopup::new(ui);
            if label_response.clicked() || label_response.secondary_clicked() {
                popup.open(name.clone());
            }
            let popup_response = popup.if_open(|popup| {
                popup
                    .text_edit_width(150.0)
                    .over(ui, &label_response, 2.75) // overwrite width if wider
                    .confirm_button_validator(&|new_name| {
                        validate_single_color_name(
                            &prefs.current,
                            new_name,
                            L.colors.actions.rename,
                        )
                    })
                    .delete_button_validator(&|_| Ok(Some(L.colors.actions.delete.into())))
                    .show(ui)
            });
            if let Some(r) = popup_response {
                match r {
                    TextEditPopupResponse::Confirm(new_name) => {
                        to_rename = Some((name.clone(), new_name));
                    }
                    TextEditPopupResponse::Delete => to_delete = Some(name.clone()),
                    TextEditPopupResponse::Cancel => (),
                }
            }
        });
    }

    if let Some((old_name, new_name)) = to_rename {
        prefs.current.custom_colors.rename(&old_name, &new_name);
        *prefs.changed = true;
    }

    if let Some(name) = to_delete {
        prefs.current.custom_colors.remove(&name);
        *prefs.changed = true;
    }

    *prefs.changed |= dnd.end_reorder(ui, &mut prefs.current.custom_colors);
}

fn show_builtin_colors_section(mut prefs_ui: PrefsUi<'_, GlobalColorPalette>) {
    for (i, color_name) in DEFAULT_PREFS
        .color_palette
        .builtin_colors
        .keys()
        .enumerate()
    {
        prefs_ui.color_with_label(color_name, access!(.builtin_colors[i]));
    }
}

fn show_builtin_color_sets_section(mut prefs_ui: PrefsUi<'_, GlobalColorPalette>) {
    let (mut prefs, ui) = prefs_ui.split();

    let mut default_sets = DEFAULT_PREFS
        .color_palette
        .builtin_color_sets
        .iter()
        .collect_vec();

    #[derive(Debug, Default, EnumIter, Copy, Clone, PartialEq, Eq, Hash)]
    enum ColorSetsSortMethod {
        #[default]
        ByCount,
        ByColor,
    }
    impl ColorSetsSortMethod {
        fn label(self) -> &'static str {
            match self {
                ColorSetsSortMethod::ByCount => L.colors.actions.sort_by_count,
                ColorSetsSortMethod::ByColor => L.colors.actions.sort_by_color,
            }
        }
    }

    let sort_method = EguiTempValue::new(ui);
    let mut sort = sort_method.get().unwrap_or_default();
    ui.horizontal(|ui| {
        for s in ColorSetsSortMethod::iter() {
            if ui.selectable_label(sort == s, s.label()).clicked() {
                sort = s;
            }
        }
    });
    sort_method.set(Some(sort));

    if sort == ColorSetsSortMethod::ByCount {
        default_sets.sort_by_cached_key(|(_, v)| v.len()); // stable sort
    }

    egui::ScrollArea::horizontal().show(ui, |ui| {
        for (set_name, _set_values) in default_sets {
            prefs
                .with(ui)
                .fixed_multi_color(set_name, access!(.builtin_color_sets[set_name]));
        }
    });
}

fn validate_single_color_name<'a>(
    palette: &GlobalColorPalette,
    new_name: &str,
    ok: &'a str,
) -> TextValidationResult<'a> {
    if new_name.is_empty() {
        Err(Some(L.colors.errors.empty_name.into()))
    } else if palette.builtin_colors.contains_key(new_name)
        || palette.custom_colors.contains_key(new_name)
    {
        Err(Some(L.colors.errors.name_conflict.into()))
    } else {
        Ok(Some(ok.into()))
    }
}
