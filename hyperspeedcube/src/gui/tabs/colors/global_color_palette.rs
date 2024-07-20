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
};
use crate::gui::util::{set_widget_spacing_to_space_width, EguiTempValue};
use crate::preferences::{GlobalColorPalette, DEFAULT_PREFS};

fn show_global_color_palette_help_ui(ui: &mut egui::Ui) {
    // TODO: markdown renderer
    ui.spacing_mut().item_spacing.y = 9.0;
    ui.heading("Global color palette");
    ui.horizontal_wrapped(|ui| {
        set_widget_spacing_to_space_width(ui);
        ui.label(
            "The global color palette provides a way to change colors \
             across all puzzles at once. For example, you can select a \
             particular shade of red to use on every puzzle with red \
             stickers.\n\
             \n\
             Some colors are organized into sets of colors that are \
             similar but still contrast with each other. For example, \
             a puzzle with two different shades of red needs those \
             shades to be distinguishable, so it uses the \"red dyad\" \
             from the global color palette.\n\
             \n\
             The color scheme for any particular puzzle can be customized in the",
        );
        ui.strong("color scheme");
        ui.label("settings.");
    });
}

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let yaml = PlaintextYamlEditor::new(ui);

    ui.set_min_width(200.0);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                yaml.show_edit_as_plaintext_button(ui, &app.prefs.color_palette);
                HelpHoverWidget::show_right_aligned(ui, show_global_color_palette_help_ui);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.strong("Global color palette");
                });
            });
        });

        ui.separator();

        let mut changed = false;

        let mut prefs_ui = PrefsUi {
            ui,
            current: &mut app.prefs.color_palette,
            defaults: Some(&DEFAULT_PREFS.color_palette),
            changed: &mut changed,
        };

        prefs_ui.collapsing("Custom colors", |mut prefs_ui| {
            let (prefs, ui) = prefs_ui.split();

            let mut dnd = crate::gui::components::DragAndDrop::new(ui);
            let mut to_rename = None;
            let mut to_delete = None;

            ui.horizontal(|ui| {
                if ui.button("Add color").clicked() {
                    let name = crate::util::find_unused_autoname(&prefs.current.custom_colors);
                    let rgb = rand::thread_rng().gen();
                    prefs
                        .current
                        .custom_colors
                        .shift_insert(0, name, Rgb { rgb });
                    *prefs.changed = true;
                }
                ui.menu_button("Sort colors", |ui| {
                    if ui.button("Sort by name").clicked() {
                        sort_map_by_key_or_reverse(&mut prefs.current.custom_colors, |name, _| {
                            name.clone()
                        });
                        *prefs.changed = true;
                    }
                    if ui.button("Sort by lightness (Oklab)").clicked() {
                        sort_map_by_key_or_reverse(
                            &mut prefs.current.custom_colors,
                            |_, &color| FloatOrd(crate::util::rgb_to_oklab(color).l),
                        );
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

                dnd.vertical_reorder_by_handle(ui, i, i, |ui, _is_dragging| {
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
                            .over(ui, &label_response, 3.0) // overwrite width
                            .confirm_button_validator(Box::new(|new_name| {
                                validate_single_color_name(&prefs.current, new_name, "Rename")
                            }))
                            .delete_button_validator(Box::new(|_| {
                                Ok(Some("Delete color".to_string()))
                            }))
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

            dnd.paint_reorder_drop_lines(ui);
            if let Some(r) = dnd.end_drag() {
                if let Some(before_or_after) = r.before_or_after {
                    crate::util::reorder_map(
                        &mut prefs.current.custom_colors,
                        r.payload,
                        r.end,
                        before_or_after,
                    );
                    *prefs.changed = true;
                }
            }
        });

        prefs_ui.collapsing("Built-in colors", |mut prefs_ui| {
            for (i, color_name) in DEFAULT_PREFS
                .color_palette
                .builtin_colors
                .keys()
                .enumerate()
            {
                prefs_ui.color(&color_name, access!(.builtin_colors[i]));
            }
        });

        prefs_ui.collapsing("Built-in color sets", |mut prefs_ui| {
            let (mut prefs, ui) = prefs_ui.split();
            egui::ScrollArea::horizontal().show(ui, |ui| {
                let mut default_sets = DEFAULT_PREFS
                    .color_palette
                    .builtin_color_sets
                    .iter()
                    .collect_vec();

                #[derive(Debug, Default, EnumIter, AsRefStr, Copy, Clone, PartialEq, Eq, Hash)]
                enum ColorSetsSortMethod {
                    #[default]
                    #[strum(serialize = "Sort by count")]
                    ByCount,
                    #[strum(serialize = "Sort by color")]
                    ByColor,
                }
                let sort_method = EguiTempValue::new(ui);
                let mut sort = sort_method.get().unwrap_or_default();
                ui.horizontal(|ui| {
                    for s in ColorSetsSortMethod::iter() {
                        if ui.selectable_label(sort == s, s.as_ref()).clicked() {
                            sort = s;
                        }
                    }
                });
                sort_method.set(Some(sort));

                if sort == ColorSetsSortMethod::ByCount {
                    default_sets.sort_by_cached_key(|(_, v)| v.len()); // stable sort
                }

                for (set_name, _set_values) in default_sets {
                    prefs
                        .with(ui)
                        .fixed_multi_color(&set_name, access!(.builtin_color_sets[set_name]));
                }
            });
        });

        app.prefs.needs_save |= changed;
    });
}

fn validate_single_color_name(
    palette: &GlobalColorPalette,
    new_name: &str,
    verb: &str,
) -> Result<Option<String>, Option<String>> {
    if new_name.is_empty() {
        Err(Some("Name cannot be empty".to_string()))
    } else if palette.builtin_colors.contains_key(new_name)
        || palette.custom_colors.contains_key(new_name)
    {
        Err(Some("There is already a color with this name".to_string()))
    } else {
        Ok(Some(format!("{verb} color")))
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
