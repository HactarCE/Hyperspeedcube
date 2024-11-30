use std::collections::HashMap;

use hyperprefs::{
    ext::reorderable::{BeforeOrAfter, DragAndDropResponse},
    ColorScheme, DefaultColorGradient, GlobalColorPalette,
};
use hyperpuzzle::{ColorSystem, DefaultColor, Rgb};
use strum::IntoEnumIterator;

use crate::{
    gui::{markdown::md, util::EguiTempFlag},
    puzzle::PuzzleView,
    L,
};

use super::{TextEditPopup, TextEditPopupResponse};

/// Pixel resolution of gradients.
const GRADIENT_RESOLUTION: usize = 1;

/// Factor by which gradients in tooltips are wider than single colors.
const GRADIENT_WIDTH_MULTIPLIER: f32 = 5.0;
/// Factor by which gradients in headers are taller than single colors.
const GRADIENT_HEIGHT_MULTIPLIER: f32 = 1.5;
/// Factor by which gradients in headers are taller than single colors when they
/// do not need to be interacted with.
const GRADIENT_COMPACT_HEIGHT_MULTIPLIER: f32 = 0.5;

/// Rounding of the colored box in the big color preview tooltip.
const TOOLTIP_COLOR_RECT_ROUNDING: f32 = 3.0;

const COLOR_PALETTE_POPUP_WIDTH: f32 = 600.0;

pub(in crate::gui) fn get_color_schemes_markdown(allow_dragging: bool) -> String {
    // TODO: customizable mousebinds!
    #[cfg(not(target_os = "macos"))]
    let color_reassign_mousebind = "ctrl + shift + right-click";
    #[cfg(target_os = "macos")]
    let color_reassign_mousebind = "cmd + shift + right-click";

    L.help
        .color_assignments
        .with(color_reassign_mousebind, allow_dragging)
}

#[derive(Debug)]
pub struct ColorsUi<'a> {
    default_color_to_puzzle_color: HashMap<DefaultColor, String>,
    gradient_totals: HashMap<DefaultColorGradient, usize>,
    palette: &'a GlobalColorPalette,

    pub clickable: bool,
    pub show_puzzle_colors: bool,
    dnd: Option<super::DragAndDrop<String, DefaultColor>>,

    hovered_color: Option<DefaultColor>,
    clicked_color: Option<DefaultColor>,
}
impl<'a> ColorsUi<'a> {
    pub fn new(palette: &'a GlobalColorPalette) -> Self {
        Self {
            default_color_to_puzzle_color: HashMap::new(),
            gradient_totals: HashMap::new(),
            palette,

            show_puzzle_colors: false,
            clickable: false,
            dnd: None,

            hovered_color: None,
            clicked_color: None,
        }
    }

    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }
    pub fn drag_puzzle_colors(mut self, ui: &mut egui::Ui, drag_puzzle_colors: bool) -> Self {
        if drag_puzzle_colors {
            self.dnd = Some(super::DragAndDrop::new(ui));
            self.show_puzzle_colors(true)
        } else {
            self.dnd = None;
            self
        }
    }
    pub fn show_puzzle_colors(mut self, show_puzzle_colors: bool) -> Self {
        self.show_puzzle_colors = show_puzzle_colors;
        self
    }

    fn click_zone(&mut self, r: &egui::Response, color: &DefaultColor) {
        if self.clickable {
            if r.hovered() {
                self.hovered_color = Some(color.clone());
            }
            if r.clicked() {
                self.clicked_color = Some(color.clone());
            }
        }
    }
    fn drag_drop_zone(&mut self, ui: &mut egui::Ui, r: &egui::Response, color: &DefaultColor) {
        if let Some(dnd) = &mut self.dnd {
            dnd.drop_zone(ui, r, color.clone());
        }
    }
    fn reorder_drag_drop_zone(
        &mut self,
        ui: &mut egui::Ui,
        r: &egui::Response,
        color: &DefaultColor,
    ) {
        if let Some(dnd) = &mut self.dnd {
            dnd.reorder_drop_zone(ui, r, color.clone());
        }
    }

    fn is_dragging(&self) -> bool {
        self.dnd.as_ref().is_some_and(|dnd| dnd.is_dragging())
    }

    fn update_reverse_color_map(&mut self, color_scheme: &ColorScheme) {
        // Construct a reverse map from default color to puzzle color. This
        // assumes that the color scheme is already valid.
        for (color_name, default_color) in color_scheme {
            self.default_color_to_puzzle_color
                .insert(default_color.clone(), color_name.clone());

            // Record gradient totals
            if let DefaultColor::Gradient {
                gradient_name,
                index: 0,
                total,
            } = default_color
            {
                if let Ok(g) = gradient_name.parse() {
                    self.gradient_totals.insert(g, *total);
                }
            }
        }
    }

    /// Shows a compact view of the global color palette, with optional labels
    /// that can be dragged to reassign colors.
    ///
    /// Returns a boolean indicating whether any modification was made to the
    /// color scheme, along with an optional temporary color scheme to allow for
    /// just the next frame.
    pub fn show_compact_palette(
        &mut self,
        ui: &mut egui::Ui,
        current_colors: Option<(&mut ColorScheme, &ColorSystem)>,
        puzzle_color_to_modify: Option<String>,
    ) -> (bool, Option<ColorScheme>) {
        self.default_color_to_puzzle_color = HashMap::new();
        self.gradient_totals = HashMap::new();
        if let Some((color_scheme, _color_system)) = &current_colors {
            self.update_reverse_color_map(color_scheme);
        }

        let large_space = ui.spacing().item_spacing.x;
        let small_space = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = large_space;
        ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();

        if !self.palette.custom_colors.is_empty() {
            ui.group(|ui| {
                ui.strong(L.colors.custom);
                ui.add_space(ui.spacing().item_spacing.x - ui.spacing().item_spacing.x);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.y = ui.spacing().item_spacing.x;
                    for color in self.palette.custom_colors.user_presets() {
                        self.show_single_color(ui, color.name().clone());
                    }
                });
            });
        }

        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong(L.colors.singles);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.y = ui.spacing().item_spacing.x;
                for color_name in self.palette.builtin_colors.keys() {
                    self.show_single_color(ui, color_name.clone());
                }
            });
        });

        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                fn get_group_name(set_size: usize) -> String {
                    match set_size {
                        1 => L.colors.set_sizes._1.to_string(),
                        2 => L.colors.set_sizes._2.to_string(),
                        3 => L.colors.set_sizes._3.to_string(),
                        4 => L.colors.set_sizes._4.to_string(),
                        5 => L.colors.set_sizes._5.to_string(),
                        6 => L.colors.set_sizes._6.to_string(),
                        7 => L.colors.set_sizes._7.to_string(),
                        8 => L.colors.set_sizes._8.to_string(),
                        9 => L.colors.set_sizes._9.to_string(),
                        10 => L.colors.set_sizes._10.to_string(),
                        n => L.colors.set_sizes.n.with(&n.to_string()),
                    }
                }

                for (group_name, sets) in self.palette.groups_of_sets(get_group_name) {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.add(
                                egui::Label::new(egui::RichText::from(group_name).strong())
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            );
                            ui.spacing_mut().item_spacing.x = small_space;
                            for (set_name, _set) in sets {
                                self.show_color_set(ui, set_name);
                            }
                        });
                    });
                }
            })
            .response
        });

        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong(L.colors.gradients);
            for gradient in DefaultColorGradient::iter() {
                self.show_color_gradient(ui, gradient);
            }
        });

        let mut temp_modification = None;
        let mut modification = None;

        if let Some((color_scheme, color_system)) = current_colors {
            if let Some(mut dnd) = self.dnd.take() {
                temp_modification = dnd.mid_drag(ui).cloned();
                modification = dnd.end_drag(ui);
            }
            if let Some(color_to_modify) = puzzle_color_to_modify {
                if let Some(hovered_color) = self.hovered_color.take() {
                    temp_modification = Some(DragAndDropResponse {
                        payload: color_to_modify.clone(),
                        end: hovered_color,
                        before_or_after: None,
                    });
                }
                if let Some(clicked_color) = self.clicked_color.take() {
                    modification = Some(DragAndDropResponse {
                        payload: color_to_modify,
                        end: clicked_color,
                        before_or_after: None,
                    });
                }
            }

            let changed = modification.is_some();
            if let Some(drag) = modification {
                self.apply_drag(color_scheme, color_system, drag);
            }

            let temp_scheme = temp_modification.map(|drag| {
                let mut temp = color_scheme.clone();
                self.apply_drag(&mut temp, color_system, drag);
                temp
            });

            (changed, temp_scheme)
        } else {
            (false, None)
        }
    }

    fn apply_drag(
        &self,
        map: &mut ColorScheme,
        color_system: &ColorSystem,
        drag: DragAndDropResponse<String, DefaultColor>,
    ) {
        match drag.before_or_after {
            Some(before_or_after) => {
                self.reorder_color_to(map, drag.payload, drag.end, before_or_after);
            }
            None => self.swap_color_to(map, drag.payload, drag.end),
        }
        let _ = self
            .palette
            .ensure_color_scheme_is_valid_for_color_system(map, color_system);
    }
    fn reorder_color_to(
        &self,
        map: &mut ColorScheme,
        name: String,
        mut new_default_color: DefaultColor,
        before_or_after: BeforeOrAfter,
    ) {
        let DefaultColor::Gradient {
            gradient_name,
            index,
            total: _,
        } = &mut new_default_color
        else {
            log::error!("attempt to reorder color to something other than a gradient");
            return;
        };

        if before_or_after == BeforeOrAfter::After {
            *index += 1;
        }

        let Ok(gradient) = gradient_name.parse::<DefaultColorGradient>() else {
            log::error!("unknown gradient name {gradient_name:?}");
            return;
        };

        // Shift other colors up by one.
        let total = *self.gradient_totals.get(&gradient).unwrap_or(&0);
        for i in *index..total {
            if let Some(name) = self
                .default_color_to_puzzle_color
                .get(&DefaultColor::Gradient {
                    gradient_name: gradient_name.clone(),
                    index: i,
                    total,
                })
            {
                map.insert(
                    name.clone(),
                    DefaultColor::Gradient {
                        gradient_name: gradient_name.clone(),
                        index: i.saturating_add(1),
                        total: total.saturating_add(1),
                    },
                );
            }
        }

        // Insert the new color.
        map.insert(name, new_default_color);
    }
    fn swap_color_to(&self, map: &mut ColorScheme, name: String, new_default_color: DefaultColor) {
        let old_name = self.default_color_to_puzzle_color.get(&new_default_color);
        let old_default_color = map.insert(name, new_default_color);

        if let Some(old_default_color) = old_default_color {
            if let Some(old_name) = old_name {
                map.insert(old_name.clone(), old_default_color);
            }
        }
    }

    fn show_single_color(&mut self, ui: &mut egui::Ui, color_name: String) {
        crate::gui::util::wrap_if_needed_for_color_button(ui);

        let tooltip_pos = ui.cursor().left_top();
        let default_color = DefaultColor::Single { name: color_name };
        let r = self.show_generic_color(ui, &default_color, tooltip_pos);
        self.click_zone(&r, &default_color);
        self.drag_drop_zone(ui, &r, &default_color);
    }

    fn show_color_set(&mut self, ui: &mut egui::Ui, color_set_name: &str) {
        let tooltip_pos = ui.cursor().left_top();
        let Some(color_set) = self.palette.get_set(color_set_name) else {
            super::error_label(ui, format!("missing color set {color_set_name:?}"));
            return;
        };

        ui.horizontal(|ui| {
            set_tight_spacing(ui);

            for i in 0..color_set.len() {
                let default_color = DefaultColor::Set {
                    set_name: color_set_name.to_string(),
                    index: i,
                };
                let r = self.show_generic_color(ui, &default_color, tooltip_pos);
                self.click_zone(&r, &default_color);
                self.drag_drop_zone(ui, &r, &default_color);
            }
        });
    }

    fn show_color_gradient(&mut self, ui: &mut egui::Ui, gradient: DefaultColorGradient) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            set_tight_spacing(ui);

            let total = *self.gradient_totals.get(&gradient).unwrap_or(&0);

            let tooltip_pos = ui.cursor().left_top();
            let mut size = ui.spacing().interact_size;
            size.x = ui.available_width();
            if total == 0 || self.clickable {
                size.y *= GRADIENT_HEIGHT_MULTIPLIER;
            } else {
                size.y *= GRADIENT_COMPACT_HEIGHT_MULTIPLIER;
            }

            let r = ColorButton {
                size,
                tooltip_pos,

                color_name: gradient.to_string(),
                color: gradient.into(),
                puzzle_color: None,
            }
            .show(ui, self);
            self.click_zone(&r, &gradient.default_color_at_end());

            if total == 0 {
                self.drag_drop_zone(ui, &r, &gradient.default_color_at(0, 1));
            } else {
                ui.horizontal_wrapped(|ui| {
                    for index in 0..total {
                        let default_color = gradient.default_color_at(index, total);
                        let r = self.show_generic_color(ui, &default_color, tooltip_pos);
                        self.click_zone(&r, &default_color);
                        self.reorder_drag_drop_zone(ui, &r, &default_color);
                    }
                });
            }
        });
    }

    fn show_generic_color(
        &mut self,
        ui: &mut egui::Ui,
        default_color: &DefaultColor,
        tooltip_pos: egui::Pos2,
    ) -> egui::Response {
        let size = ui.spacing().interact_size;
        let Some(rgb) = self.palette.get(default_color) else {
            return super::error_label(ui, format!("missing color {default_color}"));
        };
        let puzzle_color = if self.show_puzzle_colors {
            self.default_color_to_puzzle_color
                .get(default_color)
                .cloned()
        } else {
            None
        };

        ColorButton {
            size,
            tooltip_pos,

            color_name: default_color.to_string(),
            color: rgb.into(),
            puzzle_color,
        }
        .show(ui, self)
    }
}

struct ColorButton {
    pub size: egui::Vec2,
    pub tooltip_pos: egui::Pos2,

    pub color_name: String,
    pub color: ColorOrGradient,
    pub puzzle_color: Option<String>,
}
impl ColorButton {
    fn show(self, ui: &mut egui::Ui, colors_ui: &mut ColorsUi<'_>) -> egui::Response {
        // This function is based on [`egui::color_picker`]

        // Colored rectangle
        let sense = egui::Sense {
            click: colors_ui.clickable,
            drag: colors_ui.dnd.is_some(),
            focusable: true,
        };
        let r = show_color_button(ui, self.color, false, self.size, sense);

        // Draggable label
        if let Some(puzzle_color) = self.puzzle_color.filter(|_| colors_ui.show_puzzle_colors) {
            let put_puzzle_color_label = |ui: &mut egui::Ui, is_dragging: bool| {
                let text_color = if is_dragging {
                    ui.painter().rect_filled(
                        r.rect.expand(2.0),
                        3.0,
                        ui.visuals().window_fill.linear_multiply(0.75),
                    );
                    ui.visuals().strong_text_color()
                } else {
                    self.color.constrasting_text_color()
                };

                ui.put(
                    r.rect,
                    egui::Label::new(egui::RichText::new(&puzzle_color).color(text_color))
                        .selectable(false),
                );

                egui::InnerResponse::new((), r.clone())
            };

            ui.allocate_ui_at_rect(r.rect, |ui| {
                if let Some(dnd) = &mut colors_ui.dnd {
                    dnd.draggable(ui, puzzle_color.clone(), put_puzzle_color_label);
                } else {
                    put_puzzle_color_label(ui, false);
                }
            });
        }

        // Tooltip
        if !colors_ui.is_dragging()
            && (r.hovered() || r.has_focus() || r.is_pointer_button_down_on())
        {
            let id = ui.auto_id_with("hyperspeedcube::color_tooltip");

            let mut color_square_size = egui::Vec2::splat(ui.spacing().interact_size.x);
            if self.color.is_gradient() {
                color_square_size.x *= GRADIENT_WIDTH_MULTIPLIER;
            }

            let left_bottom = self.tooltip_pos + egui::vec2(-ui.spacing().menu_margin.left, -5.0);
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

                                paint_colored_rect(
                                    ui.painter(),
                                    rect,
                                    TOOLTIP_COLOR_RECT_ROUNDING,
                                    self.color,
                                );

                                ui.vertical(|ui| {
                                    ui.strong(self.color_name);
                                    match self.color {
                                        ColorOrGradient::Color(rgb) => {
                                            let [r, g, b, _a] = rgb.to_array();
                                            ui.monospace(Rgb { rgb: [r, g, b] }.to_string());
                                        }
                                        ColorOrGradient::Gradient(_) => {
                                            ui.label(L.colors.builtin_gradient);
                                        }
                                    }
                                });
                            });
                        });
                });
        }

        r.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::ColorButton));
        r
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ColorOrGradient {
    Color(egui::Color32),
    Gradient(DefaultColorGradient),
}
impl From<Rgb> for ColorOrGradient {
    fn from(value: Rgb) -> Self {
        Self::Color(value.to_egui_color32())
    }
}
impl From<DefaultColorGradient> for ColorOrGradient {
    fn from(value: DefaultColorGradient) -> Self {
        Self::Gradient(value)
    }
}
impl ColorOrGradient {
    pub fn is_gradient(self) -> bool {
        matches!(self, Self::Gradient(_))
    }
    pub fn middle_color(self) -> egui::Color32 {
        match self {
            Self::Color(c) => c,
            Self::Gradient(g) => g.eval_continuous(0.5).to_egui_color32(),
        }
    }
    pub fn constrasting_text_color(self) -> egui::Color32 {
        crate::util::contrasting_text_color(self.middle_color())
    }
}

pub fn show_color_button(
    ui: &mut egui::Ui,
    color: impl Into<ColorOrGradient>,
    is_open: bool,
    size: egui::Vec2,
    sense: egui::Sense,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let visuals = if is_open {
            &ui.visuals().widgets.open
        } else {
            ui.style().interact(&response)
        };
        let rect = rect.expand(visuals.expansion);
        paint_colored_rect(ui.painter(), rect, 0.0, color.into());

        let rounding = visuals.rounding.at_most(2.0);
        ui.painter()
            .rect_stroke(rect, rounding, (2.0, visuals.bg_fill)); // fill is intentional, because default style has no border
    }
    response
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
                let left_color = g.eval_continuous(0.0).to_egui_color32();
                painter.rect_filled(left, rounding, left_color);

                let mut right = rect;
                right.min.x = right.max.x - rounding * 2.0;
                let right_color = g.eval_continuous(1.0).to_egui_color32();
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
                let c = g.eval_rational(i, block_count - 1).to_egui_color32();
                egui::color_picker::show_color_at(painter, c, sliver);
            }
        }
    }
}

pub fn color_edit(
    ui: &mut egui::Ui,
    color: &mut Rgb,
    on_delete: Option<impl FnOnce()>,
) -> egui::Response {
    let mut changed = false;

    let mut size = ui.spacing().interact_size;
    size.x *= 1.5;
    let mut r = show_color_button(ui, *color, false, size, egui::Sense::click());

    let contrasting_text_color = crate::util::contrasting_text_color(color.to_egui_color32());
    ui.put(
        r.rect,
        egui::Label::new(
            egui::RichText::new(color.to_string())
                .color(contrasting_text_color)
                .monospace(),
        )
        .selectable(false),
    );

    // Right-click to copy
    let text_to_copy = r.secondary_clicked().then(|| color.to_string());
    if !crate::gui::components::copy_on_click(ui, &r, text_to_copy) {
        r = r.on_hover_ui(|ui| {
            md(ui, L.click_to.edit.with(L.inputs.click));
            md(ui, L.click_to.copy_hex.with(L.inputs.right_click));
            crate::gui::md_middle_click_to_delete(ui);
        });
    }

    // Alt+click to delete
    if let Some(on_delete) = on_delete {
        if crate::gui::middle_clicked(ui, &r) {
            on_delete();
        }
    }

    // Left-click to edit
    let reopen = EguiTempFlag::new(ui);
    let mut hex_edit_popup = TextEditPopup::new(ui);
    if r.clicked() {
        hex_edit_popup.open(color.to_string());
    }
    if reopen.reset() {
        hex_edit_popup.keep_open(color.to_string());
    }
    let popup_response = hex_edit_popup.if_open(|popup| {
        popup
            .over(ui, &r, 1.0)
            .text_edit_align(egui::Align::Center)
            .text_edit_monospace()
            .auto_confirm(true)
            .confirm_button_validator(&|s| s.parse::<Rgb>().map(|_| None).map_err(|_| None))
            .show_with(ui, |ui| {
                // TODO: custom color picker
                let mut egui_color = color.to_egui_color32();
                let alpha = egui::color_picker::Alpha::Opaque;
                ui.spacing_mut().slider_width = 220.0;
                if egui::color_picker::color_picker_color32(ui, &mut egui_color, alpha) {
                    *color = egui_color.into();
                    reopen.set();
                    changed = true;
                }
            })
            .0
    });
    if let Some(r) = popup_response.filter(|_| !reopen.get()) {
        match r {
            TextEditPopupResponse::Confirm(new_hex_string) => {
                if let Ok(new_color) = new_hex_string.parse() {
                    *color = new_color;
                    changed = true;
                }
            }
            _ => (),
        }
    }

    if changed {
        r.mark_changed();
    }
    r
}

fn set_tight_spacing(ui: &mut egui::Ui) {
    let item_spacing = &mut ui.spacing_mut().item_spacing;
    *item_spacing = egui::Vec2::splat(item_spacing.min_elem());
}

pub fn color_assignment_popup(
    ui: &mut egui::Ui,
    puzzle_view: &mut PuzzleView,
    color_palette: &GlobalColorPalette,
    editing_color: Option<hyperpuzzle::Color>,
) {
    let puzzle = puzzle_view.puzzle();

    let Some(color_data) = editing_color.and_then(|id| puzzle.colors.list.get(id).ok()) else {
        ui.colored_label(ui.visuals().error_fg_color, "error: no such color");
        return;
    };

    ui.set_max_width(COLOR_PALETTE_POPUP_WIDTH);
    ui.horizontal(|ui| {
        ui.heading(L.colors.puzzle_color_popup_title.with(&color_data.display));
        crate::gui::components::HelpHoverWidget::show_right_aligned(
            ui,
            &get_color_schemes_markdown(true),
        );
    });
    ui.colored_label(ui.visuals().warn_fg_color, L.colors.warning_save_changes);
    ui.separator();
    let (changed, temp_colors) = crate::gui::components::ColorsUi::new(color_palette)
        .clickable(true)
        .drag_puzzle_colors(ui, true)
        .show_compact_palette(
            ui,
            Some((&mut puzzle_view.colors.value, &puzzle.colors)),
            Some(color_data.name.clone()),
        );
    if changed {
        // the user has no way to save the settings in this UI,
        // so there's not much we can do
    }
    puzzle_view.temp_colors = temp_colors;
}
