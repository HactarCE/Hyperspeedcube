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
            ui.set_enabled(has_active_puzzle);
            show_color_palette(ui, None, &active_colors, &app.prefs.color_palette);
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

fn color_button(
    ui: &mut egui::Ui,
    color: impl Into<ColorButtonDisplay>,
    open: bool,
    label: Option<&str>,
) -> egui::Response {
    // This function is mostly copied from `egui::color_picker`.

    let color = color.into();

    let mut size = ui.spacing().interact_size;
    match color {
        ColorButtonDisplay::Single(_) => (),
        ColorButtonDisplay::Gradient(_) => size.x *= 5.0,
    }
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

        display_color_rect_segments(ui.painter(), rect, 0.0, color);

        let rounding = visuals.rounding.at_most(2.0);
        ui.painter()
            .rect_stroke(rect, rounding, (2.0, visuals.bg_fill)); // fill is intentional, because default style has no border
    }

    // Add label.
    if let Some(label) = label {
        let mid_color = match color {
            ColorButtonDisplay::Single(c) => c,
            ColorButtonDisplay::Gradient(g) => colorous_color_to_egui_color(g.eval_continuous(0.5)),
        };
        let text_color = crate::util::contrasting_text_color(mid_color);
        ui.put(
            rect,
            egui::Label::new(egui::RichText::new(label).color(text_color)).selectable(false),
        );
    }

    response
}

#[derive(Debug, Copy, Clone)]
enum ColorButtonDisplay {
    Single(egui::Color32),
    Gradient(colorous::Gradient),
}
impl From<Rgb> for ColorButtonDisplay {
    fn from(value: Rgb) -> Self {
        Self::Single(crate::util::rgb_to_egui_color32(value))
    }
}

fn show_color_palette(
    ui: &mut egui::Ui,
    mut selected_color: Option<&mut DefaultColor>,
    color_labels: &HashMap<Option<DefaultColor>, String>,
    palette: &GlobalColorPalette,
) -> egui::Response {
    let mut changed = false;

    ui.add(egui::Label::new(egui::RichText::from("Single colors").strong()).wrap(false));
    ui.add_space(ui.spacing().item_spacing.x - ui.spacing().item_spacing.y);
    ui.horizontal_wrapped(|ui| {
        for (color_name, &rgb) in &palette.singles {
            let tooltip_pos = ui.cursor().left_top();
            let default_color = DefaultColor::Single {
                name: color_name.clone(),
            };
            if display_color(ui, rgb, &default_color, color_labels, tooltip_pos).clicked() {
                if let Some(selected) = &mut selected_color {
                    **selected = default_color;
                }
                changed = true;
            }
        }
    });

    ui.separator();

    ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
    // ui.style_mut().spacing.scroll.foreground_color = true;

    let mut r = egui::ScrollArea::horizontal()
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut is_first = true;
                for (group_name, sets) in palette.groups_of_sets() {
                    if is_first {
                        is_first = false;
                    } else {
                        ui.separator();
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = ui.spacing_mut().item_spacing.yx();
                        ui.add(
                            egui::Label::new(egui::RichText::from(group_name).strong()).wrap(false),
                        );
                        for (set_name, set) in sets {
                            let mut hovered = None;
                            let tooltip_pos = ui.cursor().left_top();
                            ui.horizontal(|ui| {
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
            .response
        })
        .inner;

    ui.separator();

    ui.strong("Gradients");
    for (name, g) in [
        ("Rainbow", colorous::RAINBOW),
        ("Sinebow", colorous::SINEBOW),
        ("Spectral", colorous::SPECTRAL),
        ("Cividis", colorous::CIVIDIS),
        ("Cool", colorous::COOL),
        ("Warm", colorous::WARM),
        ("Plasma", colorous::PLASMA),
        ("Turbo", colorous::TURBO),
        ("Viridis", colorous::VIRIDIS),
    ] {
        let tooltip_pos = ui.cursor().left_top();
        let r = color_button(ui, ColorButtonDisplay::Gradient(g), false, None);
        if r.hovered() || r.has_focus() || r.is_pointer_button_down_on() {
            display_color_tooltop(
                ui,
                unique_id!(name),
                ColorButtonDisplay::Gradient(g),
                tooltip_pos,
                name,
                "Built-in gradient",
            );
        }
    }

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
        display_color_tooltop(
            ui,
            unique_id!(default_color),
            rgb,
            tooltip_pos,
            &default_color.to_string(),
            &rgb.to_string(),
        );
    }
    r
}

fn display_color_tooltop(
    ui: &mut egui::Ui,
    id: egui::Id,
    color: impl Into<ColorButtonDisplay>,
    tooltip_pos: egui::Pos2,
    top_text: &str,
    bottom_text: &str,
) {
    let color = color.into();

    let mut color_square_size = egui::Vec2::splat(ui.spacing().interact_size.x);
    match color {
        ColorButtonDisplay::Single(_) => (),
        ColorButtonDisplay::Gradient(_) => color_square_size.x *= 5.0,
    }

    let left_bottom = tooltip_pos + egui::vec2(-ui.spacing().menu_margin.left, -4.0);
    egui::Area::new(id)
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

                    display_color_rect_segments(ui.painter(), rect, 3.0, color);

                    ui.vertical(|ui| {
                        ui.style_mut().wrap = Some(false);
                        ui.strong(top_text);
                        ui.label(bottom_text);
                    });
                });
                // })
            });
        });
}

fn display_color_rect_segments(
    painter: &egui::Painter,
    mut rect: egui::Rect,
    rounding: f32,
    color: ColorButtonDisplay,
) {
    match color {
        ColorButtonDisplay::Single(c) => {
            painter.rect_filled(rect, rounding, c);
        }
        ColorButtonDisplay::Gradient(g) => {
            if rounding > 0.0 {
                let mut left = rect;
                left.max.x = left.min.x + rounding * 2.0;
                let left_color = colorous_color_to_egui_color(g.eval_continuous(0.0));
                painter.rect_filled(left, rounding, left_color);

                let mut right = rect;
                right.min.x = right.max.x - rounding * 2.0;
                let right_color = colorous_color_to_egui_color(g.eval_continuous(1.0));
                painter.rect_filled(right, rounding, right_color);

                rect.min.x += rounding;
                rect.max.x -= rounding;
            }

            const RESOLUTION: usize = 1;
            let block_count = (rect.size().x / RESOLUTION as f32).round() as usize;
            for i in 0..block_count {
                let sliver = egui::Rect::from_x_y_ranges(
                    egui::Rangef {
                        min: hypermath::util::lerp(
                            rect.min.x,
                            rect.max.x,
                            i as f32 / block_count as f32,
                        ),
                        max: hypermath::util::lerp(
                            rect.min.x,
                            rect.max.x,
                            (i + 1) as f32 / block_count as f32,
                        ),
                    },
                    rect.y_range(),
                );
                let rgb = g.eval_rational(i, block_count - 1).as_array();
                let c = crate::util::rgb_to_egui_color32(Rgb { rgb });
                egui::color_picker::show_color_at(painter, c, sliver);
            }
        }
    }
}

fn colorous_color_to_egui_color(c: colorous::Color) -> egui::Color32 {
    let rgb = c.as_array();
    crate::util::rgb_to_egui_color32(Rgb { rgb })
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
