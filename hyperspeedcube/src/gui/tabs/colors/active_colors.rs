use std::collections::{HashMap, HashSet};

use empfindung::ToLab;
use float_ord::FloatOrd;
use hyperpuzzle::{DefaultColor, Rgb};

use crate::{app::App, preferences::Preferences};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.set_enabled(app.has_active_puzzle());

    let active_colors = match app.active_puzzle_type() {
        Some(p) => p
            .colors
            .iter_values()
            .filter_map(|c| Some((c.default_color.clone()?, c.short_name.clone())))
            .collect(),
        None => HashMap::new(),
    };

    ui.group(|ui| {
        ui.collapsing("Puzzle colors", |ui| {
            if let Some(ty) = app.active_puzzle_type() {
                for (id, face_color) in &ty.colors {
                    ui.horizontal(|ui| {
                        ui.label(&face_color.long_name);
                        if let Some(default_color) = &face_color.default_color {
                            let popup_id = unique_id!(&ty.name, id);
                            let r = color_button(
                                ui,
                                app.prefs.colors.get(default_color).unwrap_or_default(),
                                false,
                                false,
                                &String::new(),
                            )
                            .on_hover_text(default_color.to_string());
                            if r.clicked() {
                                ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                            }
                            let mut new_default_color = default_color.clone();
                            egui::popup_below_widget(ui, popup_id, &r, |ui| {
                                let r = show_color_palette(
                                    ui,
                                    &mut new_default_color,
                                    &active_colors,
                                    &app.prefs,
                                );
                                if r.changed() {
                                    ui.memory_mut(|mem| mem.close_popup());
                                    dbg!("color selected!");
                                }
                            });

                            ui.label(default_color.to_string());
                        }
                    });
                }
            } else {
                ui.label("No puzzle loaded");
            }
        });
    });

    let mut changed = false;

    ui.group(|ui| {});
}

fn color_button(
    ui: &mut egui::Ui,
    color: Rgb,
    open: bool,
    is_active: bool,
    label: &str,
) -> egui::Response {
    let color = crate::util::rgb_to_egui_color32(color);

    let size = ui.spacing().interact_size;
    let (mut rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let mut ui = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center));
    if is_active && false {
        // ui.spacing_mut().item_spacing = egui::Vec2::splat(ui.spacing().item_spacing.min_elem());
        ui.add(
            egui::Label::new("▶")
                .selectable(false)
                .sense(egui::Sense::hover()),
        )
        .on_hover_text("This color is used by the active puzzle");
        ui.add_space(-ui.spacing().item_spacing.x);
        rect = ui.available_rect_before_wrap();
    }
    response.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::ColorButton));

    let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());

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

    let text_color = [egui::Color32::BLACK, egui::Color32::WHITE]
        .into_iter()
        .max_by_key(|&text_color| FloatOrd(crate::util::egui_color_distance(text_color, color)))
        .unwrap_or_default();
    // let text_color = if color.r() as u32 + color.g() as u32 + color.b() as u32 > 255 * 3 / 2 {
    //     egui::Color32::BLACK
    // } else {
    //     egui::Color32::WHITE
    // };
    let label = "ABC";
    ui.put(
        rect,
        // egui::Label::new(egui::RichText::new("✔").color(text_color)).selectable(false),
        egui::Label::new(egui::RichText::new(label).color(text_color)).selectable(false),
    );
    // }

    response
}

fn show_color_palette(
    ui: &mut egui::Ui,
    selected_color: &mut DefaultColor,
    active_colors: &HashMap<DefaultColor, String>,
    prefs: &Preferences,
) -> egui::Response {
    let mut changed = false;

    let mut r = ui
        .horizontal(|ui| {
            ui.vertical(|ui| {
                // ui.spacing_mut().item_spacing = ui.spacing_mut().item_spacing.yx();
                ui.add(egui::Label::new(egui::RichText::from("Singles").strong()).wrap(false));
                ui.add_space(ui.spacing().item_spacing.x - ui.spacing().item_spacing.y);
                for (color_name, &rgb) in &prefs.colors.singles {
                    ui.horizontal(|ui| {
                        let default_color = DefaultColor::Single {
                            name: color_name.clone(),
                        };
                        let label = active_colors.get(&default_color);
                        if color_button(
                            ui,
                            rgb,
                            false,
                            label.is_some(),
                            label.unwrap_or(&String::new()),
                        )
                        .on_hover_text(default_color.to_string())
                        .clicked()
                        {
                            *selected_color = default_color;
                            changed = true;
                        }
                    });
                }
            });

            for (group_name, sets) in prefs.colors.groups_of_sets() {
                ui.separator();
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = ui.spacing_mut().item_spacing.yx();
                    ui.add(egui::Label::new(egui::RichText::from(group_name).strong()).wrap(false));
                    for (set_name, set) in sets {
                        ui.horizontal(|ui| {
                            for (i, &rgb) in set.iter().enumerate() {
                                let default_color = DefaultColor::Set {
                                    set_name: set_name.clone(),
                                    index: i,
                                };
                                let label = active_colors.get(&default_color);
                                if color_button(
                                    ui,
                                    rgb,
                                    false,
                                    label.is_some(),
                                    label.unwrap_or(&String::new()),
                                )
                                .on_hover_text(default_color.to_string())
                                .clicked()
                                {
                                    *selected_color = default_color;
                                    changed = true;
                                }
                            }
                        });
                    }
                });
            }
        })
        .response;

    if changed {
        r.mark_changed();
    }
    r
}
