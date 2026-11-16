use egui::Widget;
use float_ord::FloatOrd;
use hcegui::dnd::Dnd;
use hsc_prefs::{DEFAULT_PREFS, GlobalColorPalette, PrefsConvert};
use itertools::Itertools;
use rand::RngExt;
use strum::{EnumIter, IntoEnumIterator};

use crate::L;
use crate::app::App;
use crate::gui::EguiValue;
use crate::gui::components::{
    HelpHoverWidget, PlaintextYamlEditor, PrefsUi, TextEditPopup, TextEditPopupResponse,
    TextValidationResult,
};
use crate::gui::ext::{DndReorderExt, ResponseExt};

pub fn show(ui: &mut egui::Ui, id: egui::Id, app: &mut App) {
    let yaml = PlaintextYamlEditor::new(ui, id);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                yaml.show_edit_as_plaintext_button(ui, &app.prefs.color_palette.to_serde());
                HelpHoverWidget::show(ui, L.help.global_color_palette);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.strong(L.colors.global_palette);
                });
            });
        });

        ui.separator();

        egui::ScrollArea::both()
            .auto_shrink(false)
            .id_salt(unique_id!(id, yaml.is_open(ui)))
            .show(ui, |ui| {
                match yaml.is_open(ui) {
                    true => {
                        if let Some(r) = yaml.show(ui)
                            && r.changed()
                        {
                            // Update value from YAML editor.
                            if let Some(Ok(deserialized)) = yaml.deserialize(ui) {
                                app.prefs.color_palette.reload_from_serde(&(), deserialized);
                                app.prefs.needs_save = true;
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
        prefs_ui.color(
            unique_id!(),
            &l.dark_background,
            access!(.dark_background_color),
        );
        prefs_ui.color(
            unique_id!(),
            &l.light_background,
            access!(.light_background_color),
        );
        prefs_ui
            .color(unique_id!(), &l.internal_faces, access!(.internals_color))
            .on_i18n_hover_explanation(&L.styles.misc.internals.face_color);
        prefs_ui
            .color(
                unique_id!(),
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

    let mut dnd = Dnd::new(ui, unique_id!());
    let mut to_rename = None;
    let mut to_delete = None;

    ui.horizontal(|ui| {
        if ui.button((mdi!(ui, PLUS), L.colors.actions.add)).clicked() {
            let custom_colors = &mut prefs.current.custom_colors;
            let name = custom_colors.make_nonconflicting_funny_name(crate::util::funny_autonames());
            let rgb = rand::rng().random();
            custom_colors.save_preset(name, hyperpuzzle::Rgb { rgb });
            custom_colors.move_index(custom_colors.len() - 1, 0);
            *prefs.changed = true;
        }

        if prefs.current.custom_colors.is_empty() {
            ui.disable();
        }
        ui.menu_button((mdi!(ui, SORT), L.colors.actions.sort), |ui| {
            let custom_colors = &mut prefs.current.custom_colors;

            let text = L.colors.actions.sort_by_name;
            if ui.button(text).clicked() {
                custom_colors.sort_by_key_or_reverse(|name, _| name.clone());
                *prefs.changed = true;
            }

            let text = L.colors.actions.sort_by_lightness;
            if ui.button(text).clicked() {
                custom_colors
                    .sort_by_key_or_reverse(|_, preset| FloatOrd(preset.value.to_oklab().l));
                *prefs.changed = true;
            }
        });
    });

    for i in 0..prefs.current.custom_colors.len() {
        let Some((name, _)) = prefs.current.custom_colors.nth_user_preset(i) else {
            log::error!("Missing custom color {i}");
            continue;
        };
        let name = name.clone();
        dnd.reorderable_with_handle(ui, name.clone(), |ui, idx| {
            let (_, preset) = prefs
                .current
                .custom_colors
                .nth_user_preset_mut(i)
                .expect("user preset disappeared");
            let color = &mut preset.value;

            let on_delete = Some(|| to_delete = Some(name.clone()));
            let r = crate::gui::components::color_edit(ui, unique_id!(idx), color, on_delete);
            *prefs.changed |= r.changed();
            let label_response = egui::Label::new(&name)
                .selectable(false)
                .sense(egui::Sense::click())
                .ui(ui);

            let mut popup = TextEditPopup::new(ui, unique_id!(idx));
            if label_response.clicked() || label_response.secondary_clicked() {
                popup.open(name.clone());
            }
            let popup_response = popup.if_open(|popup| {
                popup
                    .text_edit_width(150.0)
                    .over(ui, &label_response, egui::vec2(-1.0, -1.0)) // overwrites width if wider
                    .confirm_button(&|new_name| {
                        validate_single_color_name(prefs.current, new_name, L.colors.actions.rename)
                    })
                    .delete_button(&|_| Ok(Some(L.colors.actions.delete.into())))
                    .show()
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

    if let Some(r) = dnd.finish(ui).if_done_dragging() {
        r.reorder_collection(&mut prefs.current.custom_colors);
        *prefs.changed = true;
    }
}

fn show_builtin_colors_section(mut prefs_ui: PrefsUi<'_, GlobalColorPalette>) {
    for (i, color_name) in DEFAULT_PREFS
        .color_palette
        .builtin_colors
        .keys()
        .enumerate()
    {
        prefs_ui.color_with_label(unique_id!(i), color_name, access!(.builtin_colors[i]));
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

    let mut sort_method = EguiValue::load_or_default(ui, unique_id!());
    ui.horizontal(|ui| {
        for s in ColorSetsSortMethod::iter() {
            ui.selectable_value(&mut *sort_method, s, s.label());
        }
    });

    if *sort_method == ColorSetsSortMethod::ByCount {
        default_sets.sort_by_cached_key(|(_, v)| v.len()); // stable sort
    }

    for (set_name, _set_values) in default_sets {
        prefs.with(ui).fixed_multi_color(
            unique_id!(set_name),
            set_name,
            access!(.builtin_color_sets[set_name]),
        );
    }
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
