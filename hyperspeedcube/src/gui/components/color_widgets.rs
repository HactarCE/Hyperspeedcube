use std::collections::HashMap;

use hyperpuzzle::{DefaultColor, Rgb};
use itertools::Itertools;

use crate::preferences::{ColorScheme, DefaultColorGradient, GlobalColorPalette};

/// Pixel resolution of gradients.
const GRADIENT_RESOLUTION: usize = 1;

/// Factor by which gradients are wider than single colors.
const GRADIENT_WIDTH_MULTIPLIER: f32 = 5.0;

/// Rounding of the colored box in the big color preview tooltip.
const TOOLTIP_COLOR_RECT_ROUNDING: f32 = 3.0;

#[derive(Debug, Default, Clone)]
pub struct ReverseColorMap {
    colors: HashMap<DefaultColor, String>,
    gradient_totals: HashMap<DefaultColorGradient, usize>,
}
impl ReverseColorMap {
    pub fn from_color_scheme(scheme: &mut ColorScheme) -> Self {
        // This assumes that the color scheme is already valid.
        let mut ret = ReverseColorMap::default();
        for (color_name, default_color) in &*scheme {
            ret.colors.insert(default_color.clone(), color_name.clone());

            // Record gradient totals
            if let DefaultColor::Gradient {
                gradient_name,
                index: 0,
                total,
            } = default_color
            {
                if let Ok(g) = gradient_name.parse() {
                    ret.gradient_totals.insert(g, *total);
                }
            }
        }
        ret
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ColorOrGradient {
    Color(egui::Color32),
    Gradient(colorous::Gradient),
}
impl From<Rgb> for ColorOrGradient {
    fn from(value: Rgb) -> Self {
        Self::Color(crate::util::rgb_to_egui_color32(value))
    }
}
impl From<DefaultColorGradient> for ColorOrGradient {
    fn from(value: DefaultColorGradient) -> Self {
        Self::Gradient(value.to_colorous())
    }
}
impl ColorOrGradient {
    pub fn is_gradient(self) -> bool {
        matches!(self, Self::Gradient(_))
    }
}

#[derive(Debug, Default, Clone)]
pub struct ColorDragState {
    pub dragged_color_name: Option<String>,
}

pub fn display_single_color(
    ui: &mut egui::Ui,
    palette: &GlobalColorPalette,
    color_name: String,
    rev_map: &ReverseColorMap,
    drag_state: &mut ColorDragState,
) -> egui::Response {
    let tooltip_pos = ui.cursor().left_top();
    let default_color = DefaultColor::Single { name: color_name };
    display_color(ui, default_color, palette, rev_map, tooltip_pos, drag_state)
}

pub fn display_color_set(
    ui: &mut egui::Ui,
    palette: &GlobalColorPalette,
    color_set_name: &str,
    rev_map: &ReverseColorMap,
    drag_state: &mut ColorDragState,
) -> egui::InnerResponse<Vec<egui::Response>> {
    let tooltip_pos = ui.cursor().left_top();
    let Some(color_set) = palette.get_set(color_set_name) else {
        let r = error_label(ui, format!("missing color set {color_set_name:?}"));
        return egui::InnerResponse::new(vec![], r);
    };

    ui.horizontal(|ui| {
        (0..color_set.len())
            .map(|i| {
                let default_color = DefaultColor::Set {
                    set_name: color_set_name.to_string(),
                    index: i,
                };
                display_color(ui, default_color, palette, rev_map, tooltip_pos, drag_state)
            })
            .collect()
    })
}

pub fn display_color_gradient(
    ui: &mut egui::Ui,
    palette: &GlobalColorPalette,
    gradient: DefaultColorGradient,
    rev_map: &ReverseColorMap,
    drag_state: &mut ColorDragState,
) -> (egui::Response, egui::InnerResponse<Vec<egui::Response>>) {
    let tooltip_pos = ui.cursor().left_top();

    let r = color_button(ui, gradient, false, None);
    if r.hovered() || r.has_focus() || r.is_pointer_button_down_on() {
        let name = &gradient.to_string();
        let description = "Built-in gradient";
        display_color_tooltop(ui, gradient, tooltip_pos, name, description);
    }
    // let first_response = r;

    let seq_responses = ui.horizontal_wrapped(|ui| {
        let total = *rev_map.gradient_totals.get(&gradient).unwrap_or(&0);
        (0..total)
            .map(|index| {
                let default_color = DefaultColor::Gradient {
                    gradient_name: gradient.to_string(),
                    index,
                    total,
                };
                display_color(ui, default_color, palette, rev_map, tooltip_pos, drag_state)
            })
            .collect()
    });

    (r, seq_responses)
}

fn display_color(
    ui: &mut egui::Ui,
    default_color: DefaultColor,
    palette: &GlobalColorPalette,
    rev_map: &ReverseColorMap,
    tooltip_pos: egui::Pos2,
    drag_state: &mut ColorDragState,
) -> egui::Response {
    let Some(rgb) = palette.get(&default_color) else {
        return error_label(ui, format!("missing color {default_color}"));
    };
    let label = rev_map
        .colors
        .get(&default_color)
        .filter(|&s| drag_state.dragged_color_name.as_ref() != Some(s));

    let r = color_button(ui, rgb, false, label);
    if r.hovered() || r.has_focus() || r.is_pointer_button_down_on() {
        let name = default_color.to_string();
        let description = rgb.to_string();
        display_color_tooltop(ui, rgb, tooltip_pos, &name, &description);
    }
    r
}

fn color_button(
    ui: &mut egui::Ui,
    color: impl Into<ColorOrGradient>,
    open: bool,
    label: Option<&String>,
) -> egui::Response {
    // This function is mostly copied from `egui::color_picker`.

    let color = color.into();

    let mut size = ui.spacing().interact_size;
    if color.is_gradient() && label.is_none() {
        size.x = ui.available_width();
        // size.x *= GRADIENT_WIDTH_MULTIPLIER;
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

        paint_colored_rect(ui.painter(), rect, 0.0, color);

        let rounding = visuals.rounding.at_most(2.0);
        ui.painter()
            .rect_stroke(rect, rounding, (2.0, visuals.bg_fill)); // fill is intentional, because default style has no border
    }

    // Add label.
    if let Some(label) = label {
        let mid_color = match color {
            ColorOrGradient::Color(c) => c,
            ColorOrGradient::Gradient(g) => colorous_color_to_egui_color(g.eval_continuous(0.5)),
        };
        let text_color = crate::util::contrasting_text_color(mid_color);
        ui.put(
            rect,
            egui::Label::new(egui::RichText::new(label).color(text_color)).selectable(false),
        );
    }

    response
}

fn display_color_tooltop(
    ui: &mut egui::Ui,
    color: impl Into<ColorOrGradient>,
    tooltip_pos: egui::Pos2,
    top_text: &str,
    bottom_text: &str,
) {
    let id = ui.auto_id_with("hyperspeedcube::color_tooltip");

    let color = color.into();

    let mut color_square_size = egui::Vec2::splat(ui.spacing().interact_size.x);
    if color.is_gradient() {
        color_square_size.x *= GRADIENT_WIDTH_MULTIPLIER;
    }

    let left_bottom = tooltip_pos + egui::vec2(-ui.spacing().menu_margin.left, -4.0);
    egui::Area::new(id)
        .interactable(false)
        .fixed_pos(left_bottom)
        .constrain(true)
        .pivot(egui::Align2::LEFT_BOTTOM)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _response) =
                        ui.allocate_exact_size(color_square_size, egui::Sense::hover());

                    paint_colored_rect(ui.painter(), rect, TOOLTIP_COLOR_RECT_ROUNDING, color);

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

fn paint_colored_rect(
    painter: &egui::Painter,
    mut rect: egui::Rect,
    rounding: f32,
    color: ColorOrGradient,
) {
    match color {
        ColorOrGradient::Color(c) => {
            painter.rect_filled(rect, rounding, c);
        }
        ColorOrGradient::Gradient(g) => {
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

            let block_count = (rect.size().x * painter.ctx().pixels_per_point()
                / GRADIENT_RESOLUTION as f32)
                .round() as usize;
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

fn error_label(ui: &mut egui::Ui, text: impl Into<egui::RichText>) -> egui::Response {
    ui.colored_label(ui.visuals().error_fg_color, text)
}

fn colorous_color_to_egui_color(c: colorous::Color) -> egui::Color32 {
    let rgb = c.as_array();
    crate::util::rgb_to_egui_color32(Rgb { rgb })
}
