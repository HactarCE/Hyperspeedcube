use itertools::Itertools;
use std::{
    collections::{hash_map, HashMap, HashSet},
    sync::Arc,
};

use empfindung::ToLab;
use float_ord::FloatOrd;
use hyperpuzzle::{Color, ColorSystem, DefaultColor, Rgb};
use indexmap::IndexMap;

use crate::{
    app::App,
    gui::util::text_width,
    preferences::{ColorPreferences, GlobalColorPalette, Preferences, Preset, WithPresets},
};

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
enum ColorsTab {
    #[default]
    ByColor,
    ByFacet,
    ContrastMatrix,
}

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let active_puzzle_ty = app.active_puzzle_type();
    let has_active_puzzle = active_puzzle_ty.is_some();
    ui.set_enabled(has_active_puzzle);

    let color_system = match &active_puzzle_ty {
        Some(puz) => Arc::clone(&puz.colors),
        None => Arc::new(ColorSystem::new_empty()),
    };

    let get_color_name = |id| color_system.list[id].name.clone();

    let mut changed = false;

    let color_system_prefs = app.prefs.color_schemes.color_system_mut(&color_system);
    let mut active_colors = HashMap::<Option<DefaultColor>, String>::new();
    for (name, default_color) in &color_system_prefs.schemes.current {
        match active_colors.entry(default_color.clone()) {
            hash_map::Entry::Occupied(mut e) => {
                *e.get_mut() += ", ";
                *e.get_mut() += name;
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(name.clone());
            }
        }
    }
    let mut presets_ui = crate::gui::components::PresetsUi {
        id: unique_id!(),
        presets: &mut color_system_prefs.schemes,
        changed: &mut changed,
        text: crate::gui::components::PresetsUiText {
            presets_set: Some(&color_system.name),
            preset: "color scheme",
            presets: "color schemes",
            what: "color scheme",
        },
    };
    presets_ui.show_presets_selector(ui);
    let mut default_preset = None;
    presets_ui.show_current_prefs_ui(
        ui,
        |_| {
            default_preset = Some(Preset {
                name: color_system.default_scheme.clone(),
                value: color_system
                    .default_scheme()
                    .iter()
                    .map(|(id, default_color)| (get_color_name(id), default_color.clone()))
                    .collect(),
            });
            default_preset.as_ref()
        },
        |prefs_ui| {
            let ui = prefs_ui.ui;
            let tab = ui
                .horizontal_wrapped(|ui| {
                    let id = unique_id!();
                    let mut tab = ui.data(|data| data.get_temp(id)).unwrap_or_default();
                    ui.selectable_value(&mut tab, ColorsTab::ByColor, "By color");
                    ui.selectable_value(&mut tab, ColorsTab::ByFacet, "By facet");
                    ui.selectable_value(&mut tab, ColorsTab::ContrastMatrix, "Contrast matrix");
                    ui.data_mut(|data| data.insert_temp(id, tab));
                    tab
                })
                .inner;
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.separator()
            });

            ui.set_enabled(has_active_puzzle);

            match tab {
                ColorsTab::ByColor => {
                    show_color_palette(ui, None, &active_colors, &app.prefs.color_palette)
                }
                ColorsTab::ByFacet => ui.label("This is a different UI"),
                ColorsTab::ContrastMatrix => ui.label("TOTALLY DIFFERENT"),
            };
            if tab == ColorsTab::ByFacet {
                ui.group(|ui| {});
            }

            // app.with_active_puzzle_view(|p| {
            //     let mut active_colors = HashMap::<DefaultColor, Vec<String>>::new();
            //     for (color, default_color) in &p.view.colors.value {
            //         if let Some(default_color) = default_color {
            //             active_colors
            //                 .entry(default_color.clone())
            //                 .or_default()
            //                 .push(color.clone());
            //         }
            //     }
            //     active_colors
            // });
            let scheme = ui.group(|ui| {
                ui.collapsing("Puzzle colors", |ui| {
                    if let Some(ty) = active_puzzle_ty {
                        for (id, face_color) in &ty.colors.list {
                            ui.horizontal(|ui| {
                                ui.label(&face_color.display);
                                // if let Some(default_color) = &face_color.default_color {
                                //     let popup_id = unique_id!(&ty.name, id);
                                //     let rgb = app.prefs.colors.get(default_color).unwrap_or_default();
                                //     let r = color_button(ui, rgb, false, None)
                                //         .on_hover_text(default_color.to_string());
                                //     if r.clicked() {
                                //         ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                                //     }
                                //     let mut new_default_color = default_color.clone();
                                //     egui::popup_below_widget(ui, popup_id, &r, |ui| {
                                //         let r = show_color_palette(
                                //             ui,
                                //             &mut new_default_color,
                                //             &active_colors,
                                //             &app.prefs,
                                //         );
                                //         if r.changed() {
                                //             ui.memory_mut(|mem| mem.close_popup());
                                //             dbg!("color selected!");
                                //         }
                                //     });

                                //     ui.label(default_color.to_string());
                                // }
                            });
                        }
                    } else {
                        ui.label("No puzzle loaded");
                    }
                });
            });
        },
    );

    // Copy settings back to active puzzle.
    if changed {
        let current_color_scheme = app
            .prefs
            .color_schemes
            .color_system_mut(&color_system)
            .schemes
            .current_preset();
        app.with_active_puzzle_view(|p| {
            p.view.colors = current_color_scheme;
        });
    }

    app.prefs.needs_save |= changed;
}

fn color_button(ui: &mut egui::Ui, color: Rgb, open: bool, label: Option<&str>) -> egui::Response {
    // This function is mostly copied from `egui::color_picker`.

    let color = crate::util::rgb_to_egui_color32(color);

    let size = ui.spacing().interact_size;
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let mut ui = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center));
    response.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::ColorButton));

    if ui.is_rect_visible(rect) {
        let visuals = if open {
            &ui.visuals().widgets.open
        } else {
            ui.style().interact(&response)
        };
        let rect = rect.expand(visuals.expansion);

        egui::color_picker::show_color_at(ui.painter(), color, rect);

        let rounding = visuals.rounding.at_most(2.0);
        ui.painter()
            .rect_stroke(rect, rounding, (2.0, visuals.bg_fill)); // fill is intentional, because default style has no border
    }

    // Add label.
    if let Some(label) = label {
        let text_color = crate::util::contrasting_text_color(color);
        ui.put(
            rect,
            egui::Label::new(egui::RichText::new(label).color(text_color)).selectable(false),
        );
    }

    response
}

fn show_color_palette(
    ui: &mut egui::Ui,
    mut selected_color: Option<&mut DefaultColor>,
    color_labels: &HashMap<Option<DefaultColor>, String>,
    palette: &GlobalColorPalette,
) -> egui::Response {
    let mut changed = false;

    let mut r = ui
        .horizontal(|ui| {
            ui.vertical(|ui| {
                ui.add(egui::Label::new(egui::RichText::from("Singles").strong()).wrap(false));
                ui.add_space(ui.spacing().item_spacing.x - ui.spacing().item_spacing.y);
                for (color_name, &rgb) in &palette.singles {
                    let tooltip_pos = ui.cursor().left_top();
                    ui.horizontal(|ui| {
                        let default_color = DefaultColor::Single {
                            name: color_name.clone(),
                        };
                        if display_color(ui, rgb, &default_color, color_labels, tooltip_pos)
                            .clicked()
                        {
                            if let Some(selected) = &mut selected_color {
                                **selected = default_color;
                            }
                            changed = true;
                        }
                    });
                }
            });

            for (group_name, sets) in palette.groups_of_sets() {
                ui.separator();
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = ui.spacing_mut().item_spacing.yx();
                    ui.add(egui::Label::new(egui::RichText::from(group_name).strong()).wrap(false));
                    for (set_name, set) in sets {
                        let mut hovered = None;
                        let tooltip_pos = ui.cursor().left_top();
                        let r = ui.horizontal(|ui| {
                            for (i, &rgb) in set.iter().enumerate() {
                                let default_color = DefaultColor::Set {
                                    set_name: set_name.clone(),
                                    index: i,
                                };
                                let r = display_color(
                                    ui,
                                    rgb,
                                    &default_color,
                                    color_labels,
                                    tooltip_pos,
                                );
                                if r.hovered() || r.has_focus() || r.dragged() {
                                    if hovered.is_some() {
                                        hovered = Some(set_name.clone());
                                    } else {
                                        hovered = Some(default_color.to_string());
                                    }
                                }
                                if r.clicked() {
                                    if let Some(selected) = &mut selected_color {
                                        **selected = default_color;
                                    }
                                    changed = true;
                                }
                            }
                        });
                    }
                });
            }
        })
        .response;

    ui.separator();

    if changed {
        r.mark_changed();
    }
    r
}

fn display_color(
    ui: &mut egui::Ui,
    rgb: Rgb,
    default_color: &DefaultColor,
    active_colors: &HashMap<Option<DefaultColor>, String>,
    tooltip_pos: egui::Pos2,
) -> egui::Response {
    let label = active_colors
        .get(&Some(default_color.clone()))
        .map(|s| s.as_str());
    let r = color_button(ui, rgb, false, label);
    if r.hovered() || r.has_focus() || r.is_pointer_button_down_on() {
        let color_square_size = egui::Vec2::splat(ui.spacing().interact_size.x);
        let left_bottom = tooltip_pos + egui::vec2(-ui.spacing().menu_margin.left, -4.0);
        egui::Area::new(unique_id!(default_color))
            .interactable(false)
            .fixed_pos(left_bottom)
            .constrain(true)
            .pivot(egui::Align2::LEFT_BOTTOM)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    // ui.allocate_ui_at_rect(desired_rect, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _response) =
                            ui.allocate_exact_size(color_square_size, egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, 3.0, crate::util::rgb_to_egui_color32(rgb));
                        ui.vertical(|ui| {
                            ui.style_mut().wrap = Some(false);
                            // egui::text::LayoutJob::single_section(default_color.to_string(), egui::TextFormat::simple(font_id, color))
                            // egui::WidgetText::from(default_color.to_string())
                            ui.strong(default_color.to_string());
                            ui.label(rgb.to_string());
                            // ui.set_width(ui.available_width());
                        });
                    });
                    // })
                });
            });
    }
    r
}

fn strip_set_suffix(s: &str) -> &str {
    None.or_else(|| s.strip_suffix(" Dyad"))
        .or_else(|| s.strip_suffix(" Triad"))
        .or_else(|| s.strip_suffix(" Tetrad"))
        .or_else(|| s.strip_suffix(" Pentad"))
        .or_else(|| s.strip_suffix(" Hexad"))
        .or_else(|| s.strip_suffix(" Heptad"))
        .or_else(|| s.strip_suffix(" Octad"))
        .or_else(|| s.strip_suffix(" Nonad"))
        .or_else(|| s.strip_suffix(" Decad"))
        .unwrap_or(s)
}
