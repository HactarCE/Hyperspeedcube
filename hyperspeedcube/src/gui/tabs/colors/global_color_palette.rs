use egui::Widget;
use float_ord::FloatOrd;
use hyperpuzzle::Rgb;
use indexmap::IndexMap;
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
use crate::preferences::{GlobalColorPalette, DEFAULT_PREFS};
use crate::L;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let yaml = PlaintextYamlEditor::new(ui);

    ui.set_min_width(200.0);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                yaml.show_edit_as_plaintext_button(ui, &app.prefs.color_palette);
                HelpHoverWidget::show_right_aligned(ui, L.help.global_color_palette);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.strong(L.colors.global_palette);
                });
            });
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let mut changed = false;

                let mut prefs_ui = PrefsUi {
                    ui,
                    current: &mut app.prefs.styles,
                    defaults: Some(&DEFAULT_PREFS.styles),
                    changed: &mut changed,
                };

                let l = &L.styles.misc;
                prefs_ui.collapsing(l.title, |mut prefs_ui| {
                    prefs_ui.color(&l.background.dark_mode, access!(.dark_background_color));
                    prefs_ui.color(&l.background.light_mode, access!(.light_background_color));
                    prefs_ui
                        .color(&l.internals.face_color, access!(.internals_color))
                        .on_i18n_hover_explanation(&l.internals.face_color);
                    prefs_ui.color(
                        &l.blocking_pieces.outlines_color,
                        access!(.blocking_outline_color),
                    );
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
            });
    });
}

fn show_custom_colors_section(mut prefs_ui: PrefsUi<'_, GlobalColorPalette>) {
    let (prefs, ui) = prefs_ui.split();

    let mut dnd = crate::gui::components::DragAndDrop::new(ui);
    let mut to_rename = None;
    let mut to_delete = None;

    ui.horizontal(|ui| {
        if ui.button(L.colors.actions.add).clicked() {
            let name = crate::util::find_unused_autoname(&prefs.current.custom_colors);
            let rgb = rand::thread_rng().gen();
            prefs
                .current
                .custom_colors
                .shift_insert(0, name, Rgb { rgb });
            *prefs.changed = true;
        }

        ui.menu_button(L.colors.actions.sort, |ui| {
            let custom_colors = &mut prefs.current.custom_colors;

            let text = L.colors.actions.sort_by_name;
            if ui.button(text).clicked() {
                sort_map_by_key_or_reverse(custom_colors, |name, _| name.clone());
                *prefs.changed = true;
            }

            let text = L.colors.actions.sort_by_lightness;
            if ui.button(text).clicked() {
                sort_map_by_key_or_reverse(custom_colors, |_, &color| {
                    FloatOrd(crate::util::rgb_to_oklab(color).l)
                });
                *prefs.changed = true;
            }
        });
    });

    for i in 0..prefs.current.custom_colors.len() {
        let Some((name, color)) = prefs.current.custom_colors.get_index(i) else {
            continue;
        };
        let name = name.clone();
        let mut color = *color;

        dnd.vertical_reorder_by_handle(ui, i, |ui, _is_dragging| {
            let on_delete = Some(|| to_delete = Some(i));
            let r = crate::gui::components::color_edit(ui, &mut color, on_delete);
            *prefs.changed |= r.changed();
            let label_response = egui::Label::new(&name)
                .selectable(false)
                .sense(egui::Sense::click())
                .ui(ui);
            if let Some((_, v)) = prefs.current.custom_colors.get_index_mut(i) {
                *v = color;
            }

            let mut popup = TextEditPopup::new(ui);
            if label_response.clicked() || label_response.secondary_clicked() {
                popup.open(name.clone());
            }
            let popup_response = popup.if_open(|popup| {
                popup
                    .text_edit_width(150.0)
                    .over(ui, &label_response, 3.0) // overwrite width if wider
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
                        to_rename = Some((i, new_name));
                    }
                    TextEditPopupResponse::Delete => to_delete = Some(i),
                    TextEditPopupResponse::Cancel => (),
                }
            }
        });
    }

    if let Some((i, new_name)) = to_rename {
        if let Some((_, v)) = prefs.current.custom_colors.swap_remove_index(i) {
            let (j, _) = prefs.current.custom_colors.insert_full(new_name, v);
            prefs.current.custom_colors.swap_indices(i, j);
            *prefs.changed = true;
        }
    }

    if let Some(i) = to_delete {
        prefs.current.custom_colors.shift_remove_index(i);
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

fn sort_map_by_key_or_reverse<K: Clone + PartialEq, V, T: Ord>(
    map: &mut IndexMap<K, V>,
    mut sort_key: impl FnMut(&K, &V) -> T,
) {
    let old_order = map.keys().cloned().collect_vec();
    map.sort_by_cached_key(&mut sort_key);
    if map.keys().eq(&old_order) {
        map.sort_by_cached_key(|k, v| std::cmp::Reverse(sort_key(k, v)));
    }
}
