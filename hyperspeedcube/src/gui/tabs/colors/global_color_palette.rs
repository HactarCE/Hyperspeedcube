use float_ord::FloatOrd;
use hyperpuzzle::Rgb;
use indexmap::IndexMap;
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::app::App;
use crate::gui::components::{
    HelpHoverWidget, PlaintextYamlEditor, PrefsUi, TextEditPopup, TextEditPopupResponse,
};
use crate::gui::util::set_widget_spacing_to_space_width;
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
                HelpHoverWidget::show(ui, show_global_color_palette_help_ui);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.strong("Global color palette");
                });
            });
        });

        let mut changed = false;

        let mut prefs_ui = PrefsUi {
            ui,
            current: &mut app.prefs.color_palette,
            defaults: Some(&DEFAULT_PREFS.color_palette),
            changed: &mut changed,
        };
        let (mut prefs, ui) = prefs_ui.split();

        ui.separator();
        ui.horizontal(|ui| {
            ui.strong("Single colors");
        });

        for (i, color_name) in DEFAULT_PREFS.color_palette.singles.keys().enumerate() {
            prefs.with(ui).color(&color_name, access!(.singles[i]));
        }

        ui.separator();
        ui.strong("Custom single colors");

        ui.horizontal(|ui| {
            if ui.button("Add color").clicked() {
                let name = find_autoname(|s| !prefs.current.custom_singles.contains_key(s));
                let rgb = rand::thread_rng().gen();
                prefs
                    .current
                    .custom_singles
                    .shift_insert(0, name, Rgb { rgb });
                *prefs.changed = true;
            }
            ui.menu_button("Sort colors", |ui| {
                if ui.button("Sort by name").clicked() {
                    sort_map_by_key_or_reverse(&mut prefs.current.custom_singles, |name, _| {
                        name.clone()
                    });
                    *prefs.changed = true;
                }
                if ui.button("Sort by lightness (Oklab)").clicked() {
                    sort_map_by_key_or_reverse(&mut prefs.current.custom_singles, |_, &color| {
                        FloatOrd(crate::util::rgb_to_oklab(color).l)
                    });
                    *prefs.changed = true;
                }
            });
        });

        let mut dnd = crate::gui::components::DragAndDrop::new(ui);
        let mut to_rename = None;
        let mut to_delete = None;

        for i in 0..prefs.current.custom_singles.len() {
            let Some((name, color)) = prefs.current.custom_singles.get_index(i) else {
                continue;
            };
            let name = name.clone();
            let mut color = *color;

            dnd.vertical_reorder_by_handle(ui, i, i, |ui, _is_dragging| {
                let on_delete = Some(|| to_delete = Some(i));
                let r = crate::gui::components::color_edit(ui, &mut color, &name, true, on_delete);
                *prefs.changed |= r.response.changed();
                let label_response = r.inner;
                if let Some((_, v)) = prefs.current.custom_singles.get_index_mut(i) {
                    *v = color;
                }

                let mut popup = TextEditPopup::new(ui);
                if label_response.clicked() || label_response.secondary_clicked() {
                    popup.open(name.clone());
                }
                let popup_response = popup.if_open(|popup| {
                    popup
                        .text_edit_width(150.0)
                        .over(&label_response) // override width
                        .confirm_button_validator(Box::new(|new_name| {
                            validate_single_color_name(&prefs.current, new_name, "Rename")
                        }))
                        .delete_button_validator(Box::new(|_| Ok(Some("Delete color".to_string()))))
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
            if let Some((_, v)) = prefs.current.custom_singles.swap_remove_index(i) {
                let (j, _) = prefs.current.custom_singles.insert_full(new_name, v);
                prefs.current.custom_singles.swap_indices(i, j);
                *prefs.changed = true;
            }
        }

        if let Some(i) = to_delete {
            prefs.current.custom_singles.shift_remove_index(i);
            *prefs.changed = true;
        }

        dnd.paint_reorder_drop_lines(ui);
        if let Some(r) = dnd.end_drag() {
            if let Some(before_or_after) = r.before_or_after {
                crate::util::reorder_map(
                    &mut prefs.current.custom_singles,
                    r.payload,
                    r.end,
                    before_or_after,
                );
                *prefs.changed = true;
            }
        }

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
    } else if palette.singles.contains_key(new_name)
        || palette.custom_singles.contains_key(new_name)
    {
        Err(Some("There is already a color with this name".to_string()))
    } else {
        Ok(Some(format!("{verb} color")))
    }
}

fn find_autoname(criteria: impl FnMut(&String) -> bool) -> String {
    color_autonames()
        .find(criteria)
        .expect("ran out of autonames!")
}

fn color_autonames() -> impl Iterator<Item = String> {
    std::iter::from_fn(move || {
        Some(if rand::thread_rng().gen_bool(0.2) {
            format!("{} {}", gen_adjective(), gen_noun())
        } else {
            gen_noun()
        })
    })
}

fn gen_adjective() -> String {
    hyperpuzzle::util::titlecase(names::ADJECTIVES.choose(&mut rand::thread_rng()).unwrap())
}
fn gen_noun() -> String {
    hyperpuzzle::util::titlecase(names::NOUNS.choose(&mut rand::thread_rng()).unwrap())
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
