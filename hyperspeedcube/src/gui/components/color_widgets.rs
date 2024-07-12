use std::collections::HashMap;

use hyperpuzzle::{DefaultColor, Rgb};

use crate::{
    gui::util::focus_and_select_all,
    preferences::{ColorScheme, DefaultColorGradient, GlobalColorPalette},
};

use super::BIG_ICON_BUTTON_SIZE;

/// Pixel resolution of gradients.
const GRADIENT_RESOLUTION: usize = 1;

/// Factor by which gradients are wider than single colors.
const GRADIENT_WIDTH_MULTIPLIER: f32 = 5.0;

/// Rounding of the colored box in the big color preview tooltip.
const TOOLTIP_COLOR_RECT_ROUNDING: f32 = 3.0;

#[derive(Debug, Default, Clone)]
pub struct ReverseColorMap {
    pub colors: HashMap<DefaultColor, String>,
    pub gradient_totals: HashMap<DefaultColorGradient, usize>,
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
    pub fn middle_color(self) -> egui::Color32 {
        match self {
            Self::Color(c) => c,
            Self::Gradient(g) => colorous_color_to_egui_color(g.eval_continuous(0.5)),
        }
    }
    pub fn constrasting_text_color(self) -> egui::Color32 {
        crate::util::contrasting_text_color(self.middle_color())
    }
}

pub fn display_single_color(
    ui: &mut egui::Ui,
    palette: &GlobalColorPalette,
    color_name: String,
    rev_map: &ReverseColorMap,
    dnd: &mut super::DragAndDrop<String, DefaultColor>,
) -> egui::Response {
    crate::gui::util::wrap_if_needed_for_color_button(ui);
    let tooltip_pos = ui.cursor().left_top();
    let default_color = DefaultColor::Single { name: color_name };
    let r = display_color(ui, &default_color, palette, rev_map, tooltip_pos, dnd);
    dnd.drop_zone(ui, &r, default_color);
    r
}

pub fn display_color_set(
    ui: &mut egui::Ui,
    palette: &GlobalColorPalette,
    color_set_name: &str,
    rev_map: &ReverseColorMap,
    dnd: &mut super::DragAndDrop<String, DefaultColor>,
) -> egui::InnerResponse<Vec<egui::Response>> {
    crate::gui::util::wrap_if_needed_for_color_button(ui);
    let tooltip_pos = ui.cursor().left_top();
    let Some(color_set) = palette.get_set(color_set_name) else {
        let r = super::error_label(ui, format!("missing color set {color_set_name:?}"));
        return egui::InnerResponse::new(vec![], r);
    };

    ui.horizontal(|ui| {
        (0..color_set.len())
            .map(|i| {
                let default_color = DefaultColor::Set {
                    set_name: color_set_name.to_string(),
                    index: i,
                };
                let r = display_color(ui, &default_color, palette, rev_map, tooltip_pos, dnd);
                dnd.drop_zone(ui, &r, default_color);
                r
            })
            .collect()
    })
}

pub fn display_color_gradient(
    ui: &mut egui::Ui,
    palette: &GlobalColorPalette,
    gradient: DefaultColorGradient,
    rev_map: &ReverseColorMap,
    dnd: &mut super::DragAndDrop<String, DefaultColor>,
) {
    let total = *rev_map.gradient_totals.get(&gradient).unwrap_or(&0);

    let r = ui.group(|ui| {
        ui.set_width(ui.available_width());
        let tooltip_pos = ui.cursor().left_top();

        let mut size = ui.spacing().interact_size;
        size.x = ui.available_width();
        if total == 0 {
            size.y *= 1.5;
        } else {
            size.y *= 0.5;
        }
        let r = color_button(ui, size, gradient, false, None, dnd);
        if r.hovered() || r.has_focus() || r.is_pointer_button_down_on() {
            let name = &gradient.to_string();
            display_color_tooltop(ui, gradient, tooltip_pos, name);
        }
        if total == 0 {
            return;
        }

        ui.horizontal_wrapped(|ui| {
            for index in 0..total {
                let default_color = DefaultColor::Gradient {
                    gradient_name: gradient.to_string(),
                    index,
                    total,
                };
                let r = display_color(ui, &default_color, palette, rev_map, tooltip_pos, dnd);
                dnd.reorder_drop_zone(ui, r, default_color);
            }
        });
    });

    if total == 0 {
        dnd.drop_zone(
            ui,
            &r.response,
            DefaultColor::Gradient {
                gradient_name: gradient.to_string(),
                index: usize::MAX,
                total: usize::MAX,
            },
        );
    }
}

fn display_color(
    ui: &mut egui::Ui,
    default_color: &DefaultColor,
    palette: &GlobalColorPalette,
    rev_map: &ReverseColorMap,
    tooltip_pos: egui::Pos2,
    dnd: &mut super::DragAndDrop<String, DefaultColor>,
) -> egui::Response {
    let Some(rgb) = palette.get(&default_color) else {
        return super::error_label(ui, format!("missing color {default_color}"));
    };
    let label = rev_map.colors.get(&default_color);

    let size = ui.spacing().interact_size;
    let r = color_button(ui, size, rgb, false, label, dnd);
    if (r.hovered() || r.has_focus() || r.is_pointer_button_down_on()) && !dnd.is_dragging() {
        let name = default_color.to_string();
        display_color_tooltop(ui, rgb, tooltip_pos, &name);
    }
    r
}

fn color_button(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    color: impl Into<ColorOrGradient>,
    open: bool,
    label: Option<&String>,
    dnd: &mut super::DragAndDrop<String, DefaultColor>,
) -> egui::Response {
    // This function is mostly copied from `egui::color_picker`.

    let color = color.into();

    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());
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
        ui.allocate_ui_at_rect(rect, |ui| {
            dnd.draggable(ui, label.clone(), |ui, is_dragging| {
                let text_color = if is_dragging {
                    ui.painter().rect_filled(
                        rect,
                        2.0,
                        ui.visuals().window_fill.linear_multiply(0.75),
                    );
                    ui.visuals().strong_text_color()
                } else {
                    color.constrasting_text_color()
                };

                ui.put(
                    rect,
                    egui::Label::new(egui::RichText::new(label).color(text_color))
                        .selectable(false),
                );

                response.clone()
            });
        });
    }

    response
}

fn display_color_tooltop(
    ui: &mut egui::Ui,
    color: impl Into<ColorOrGradient>,
    tooltip_pos: egui::Pos2,
    top_text: &str,
) {
    let id = ui.auto_id_with("hyperspeedcube::color_tooltip");

    let color = color.into();

    let mut color_square_size = egui::Vec2::splat(ui.spacing().interact_size.x);
    if color.is_gradient() {
        color_square_size.x *= GRADIENT_WIDTH_MULTIPLIER;
    }

    let left_bottom = tooltip_pos + egui::vec2(-ui.spacing().menu_margin.left, -5.0);
    egui::Area::new(id)
        .interactable(false)
        .fixed_pos(left_bottom)
        .constrain(true)
        .pivot(egui::Align2::LEFT_BOTTOM)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .shadow(egui::epaint::Shadow::NONE)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _response) =
                            ui.allocate_exact_size(color_square_size, egui::Sense::hover());

                        paint_colored_rect(ui.painter(), rect, TOOLTIP_COLOR_RECT_ROUNDING, color);

                        ui.vertical(|ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.strong(top_text);
                            match color {
                                ColorOrGradient::Color(rgb) => {
                                    let [r, g, b, _a] = rgb.to_array();
                                    ui.monospace(Rgb { rgb: [r, g, b] }.to_string());
                                }
                                ColorOrGradient::Gradient(_) => {
                                    ui.label("Built-in gradient");
                                }
                            }
                        });
                    });
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

fn colorous_color_to_egui_color(c: colorous::Color) -> egui::Color32 {
    let rgb = c.as_array();
    crate::util::rgb_to_egui_color32(Rgb { rgb })
}

pub fn color_hex_editor(ui: &mut egui::Ui, color: &mut Rgb) -> egui::Response {
    let mut r = ui.add(
        egui::Label::new(egui::RichText::new(color.to_string()).monospace())
            .selectable(false)
            .sense(egui::Sense::click_and_drag()), // for when it's draggable
    );

    // Right click to copy
    let has_been_copied = crate::gui::util::EguiTempFlag::from_ui(ui);
    if r.secondary_clicked() {
        ui.ctx().copy_text(color.to_string());
        has_been_copied.set();
    }
    if has_been_copied.get() {
        if r.hovered() {
            // Show the tooltip immediately, with no delay
            egui::show_tooltip_for(ui.ctx(), r.id, &r.rect, |ui| ui.label("Copied!"));
        } else {
            // Hide the tooltip when the mouse leaves
            has_been_copied.clear();
        }
    } else {
        r = r.on_hover_text("Click to edit\nRight click to copy");
    }

    // Left click to edit
    let popup_data = crate::gui::util::EguiTempValue::from_ui(ui);
    let mut is_first_frame_of_popup = false;
    if r.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_data.id));
        popup_data.set(Some(color.to_string()));
        is_first_frame_of_popup = true;
    }
    crate::gui::util::fake_popup(
        ui,
        popup_data.id,
        is_first_frame_of_popup,
        egui::Rect::from_min_size(r.rect.left_top(), egui::Vec2::ZERO)
            .translate(egui::vec2(-10.0, -8.0)), // approx
        |ui| {
            // Text edit
            let mut s = popup_data.get().unwrap_or_default();
            let text_response = egui::TextEdit::singleline(&mut s)
                .font(egui::TextStyle::Monospace)
                .desired_width(r.rect.width())
                .show(ui);
            if is_first_frame_of_popup {
                focus_and_select_all(ui, text_response);
            }
            popup_data.set(Some(s.clone()));

            // Check button
            let parsed = s.parse();
            ui.add_enabled_ui(parsed.is_ok(), |ui| {
                if ui
                    .add(egui::Button::new("✔").min_size(BIG_ICON_BUTTON_SIZE))
                    .clicked()
                    || ui.input(|input| input.key_pressed(egui::Key::Enter))
                {
                    if let Ok(new_value) = parsed {
                        *color = new_value;
                        r.mark_changed();
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
            })
        },
    );
    if !ui.memory(|mem| mem.any_popup_open()) {
        popup_data.set(None);
    }

    r
}

pub fn color_edit(
    ui: &mut egui::Ui,
    color: &mut Rgb,
    label_on_color: Option<&String>,
    label_beside_color: &str,
) -> egui::Response {
    let mut changed = false;

    let mut r = ui.horizontal(|ui| {
        changed |= super::color_hex_editor(ui, color).changed();

        let r = ui.color_edit_button_srgb(&mut color.rgb);
        changed |= r.changed();

        // Label on the color edit button
        if let Some(color_name) = label_on_color {
            let text_color =
                crate::util::contrasting_text_color(crate::util::rgb_to_egui_color32(*color));
            ui.put(
                r.rect,
                egui::Label::new(egui::RichText::new(color_name).color(text_color))
                    .selectable(false),
            );
        }

        ui.label(label_beside_color);
    });

    if changed {
        r.response.mark_changed();
    }
    r.response
}
