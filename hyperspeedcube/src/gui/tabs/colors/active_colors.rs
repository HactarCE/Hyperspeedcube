use std::collections::{HashMap, HashSet};

use empfindung::ToLab;
use float_ord::FloatOrd;
use hyperpuzzle::{DefaultColor, Rgb};

use crate::{
    app::App,
    gui::components::PresetsUi,
    preferences::{ColorPreferences, Preferences, WithPresets},
};

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
enum ColorsTab {
    #[default]
    ByColor,
    ByFacet,
    ContrastMatrix,
}

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let tab = ui
        .horizontal(|ui| {
            let id = unique_id!();
            let mut tab = ui.data(|data| data.get_temp(id)).unwrap_or_default();
            ui.selectable_value(&mut tab, ColorsTab::ByColor, "Show by color");
            ui.selectable_value(&mut tab, ColorsTab::ByFacet, "Show by facet");
            ui.selectable_value(&mut tab, ColorsTab::ContrastMatrix, "Show contrast matrix");
            ui.data_mut(|data| data.insert_temp(id, tab));
            tab
        })
        .inner;
    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
        ui.separator()
    });

    ui.set_enabled(app.has_active_puzzle());

    match tab {
        ColorsTab::ByColor => ui.label("this is one UI"),
        ColorsTab::ByFacet => ui.label("This is a different UI"),
        ColorsTab::ContrastMatrix => ui.label("TOTALLY DIFFERENT"),
    };
    if tab == ColorsTab::ByFacet {
        ui.group(|ui| {});
    }

    if tab != ColorsTab::ByColor {
        return;
    }

    let active_colors = app.with_active_puzzle_view(|p| p.view.colors.color_to_name_mapping());
    let scheme = ui.group(|ui| {
        ui.collapsing("Puzzle colors", |ui| {
            if let Some(ty) = app.active_puzzle_type() {
                for (id, face_color) in &ty.colors {
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

    let mut changed = false;
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
    selected_color: &mut DefaultColor,
    active_colors: &HashMap<DefaultColor, String>,
    prefs: &Preferences,
) -> egui::Response {
    let mut changed = false;

    let mut r = ui
        .horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = ui.spacing_mut().item_spacing.yx();
                ui.add(egui::Label::new(egui::RichText::from("Singles").strong()).wrap(false));
                for (color_name, &rgb) in &prefs.colors.singles {
                    ui.horizontal(|ui| {
                        let default_color = DefaultColor::Single {
                            name: color_name.clone(),
                        };
                        if display_color(ui, rgb, &default_color, active_colors).clicked() {
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
                                if display_color(ui, rgb, &default_color, active_colors).clicked() {
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

fn display_color(
    ui: &mut egui::Ui,
    rgb: Rgb,
    default_color: &DefaultColor,
    active_colors: &HashMap<DefaultColor, String>,
) -> egui::Response {
    let label = active_colors.get(&default_color).map(|s| s.as_str());
    color_button(ui, rgb, false, label).on_hover_text(default_color.to_string())
}
