use egui::NumExt;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FilterCheckboxAllowedStates {
    /// Three states: blank, check, X
    NeutralShowHide,
    /// Two states: blank, X
    NeutralHide,
}
pub enum FilterCheckboxState<'a> {
    /// Indeterminate state representing a mixed group, shown using a dash.
    Mixed,
    /// Coherent state, shown using a check for `Some(true)`, an X for
    /// `Some(false)`, or a blank for `None`.
    Coherent(&'a mut Option<bool>),
}
impl<'a> From<Option<&'a mut Option<bool>>> for FilterCheckboxState<'a> {
    fn from(value: Option<&'a mut Option<bool>>) -> Self {
        value.map_or(FilterCheckboxState::Mixed, FilterCheckboxState::Coherent)
    }
}
impl FilterCheckboxState<'_> {
    pub fn cycle_state_forward(&mut self, allowed_states: FilterCheckboxAllowedStates) {
        if let FilterCheckboxState::Coherent(state) = self {
            **state = match allowed_states {
                FilterCheckboxAllowedStates::NeutralShowHide => match state {
                    None => Some(true),
                    Some(true) => Some(false),
                    Some(false) => None,
                },
                FilterCheckboxAllowedStates::NeutralHide => state.is_none().then_some(false),
            };
        }
    }
    pub fn cycle_state_backward(&mut self, allowed_states: FilterCheckboxAllowedStates) {
        if let FilterCheckboxState::Coherent(state) = self {
            **state = match allowed_states {
                FilterCheckboxAllowedStates::NeutralShowHide => match state {
                    None => Some(false),
                    Some(false) => Some(true),
                    Some(true) => None,
                },
                FilterCheckboxAllowedStates::NeutralHide => state.is_none().then_some(false),
            };
        }
    }
}

/// Checkbox with _four_ states (check, X, dash, empty), an optional RGB color,
/// and a label.
pub struct FilterCheckbox<'a> {
    allowed_states: FilterCheckboxAllowedStates,
    state: FilterCheckboxState<'a>,
    color: Option<egui::Color32>,
    text: egui::WidgetText,
    indent: bool,
}
impl<'a> FilterCheckbox<'a> {
    /// Constructs a new multi-state piece filter checkbox.
    pub fn new(
        allowed_states: FilterCheckboxAllowedStates,
        state: impl Into<FilterCheckboxState<'a>>,
        text: impl Into<egui::WidgetText>,
    ) -> Self {
        Self {
            allowed_states,
            state: state.into(),
            color: None,
            text: text.into(),
            indent: false,
        }
    }
    /// Adds a color display to the checkbox.
    pub fn color(mut self, color: egui::Color32) -> Self {
        self.color = Some(color);
        self
    }
    /// Indents the checkbox.
    pub fn indent(mut self) -> Self {
        self.indent = true;
        self
    }
}
/// This implementation is modified from [`egui::Checkbox::ui()`].
impl egui::Widget for FilterCheckbox<'_> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let spacing = &ui.spacing();
        let indent = if self.indent { spacing.indent } else { 0.0 };
        let icon_width = spacing.icon_width;
        let color_spacing = spacing.item_spacing.x;
        let color_size = egui::vec2(spacing.icon_width * 1.5, spacing.icon_width);
        let label_spacing = spacing.icon_spacing;

        let mut width = indent + icon_width + label_spacing;
        if self.color.is_some() {
            width += color_size.x + color_spacing;
        }
        let wrap_width = ui.available_width() - width;
        let galley = self
            .text
            .into_galley(ui, Some(true), wrap_width, egui::TextStyle::Button);
        let text_width = galley.size().x;
        width += text_width;
        let height = f32::max(icon_width, galley.size().y);

        let desired_size = egui::Vec2::new(width, height).at_least(spacing.interact_size);

        let (mut rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if response.clicked() {
            self.state.cycle_state_forward(self.allowed_states);
            response.mark_changed();
        } else if response.secondary_clicked() {
            self.state.cycle_state_backward(self.allowed_states);
            response.mark_changed();
        }
        response.widget_info(|| match &self.state {
            FilterCheckboxState::Mixed | FilterCheckboxState::Coherent(None) => {
                egui::WidgetInfo::labeled(egui::WidgetType::Checkbox, galley.text())
            }
            FilterCheckboxState::Coherent(Some(state)) => {
                egui::WidgetInfo::selected(egui::WidgetType::Checkbox, *state, galley.text())
            }
        });

        if ui.is_rect_visible(rect) {
            rect.min.x += indent;

            // Icon
            let visuals = ui.style().interact(&response);
            let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
            ui.painter().add(egui::epaint::RectShape::new(
                big_icon_rect.expand(visuals.expansion),
                visuals.rounding,
                visuals.bg_fill,
                visuals.bg_stroke,
            ));
            match self.state {
                // Horizontal line
                FilterCheckboxState::Mixed => {
                    ui.painter().add(egui::Shape::hline(
                        small_icon_rect.x_range(),
                        small_icon_rect.center().y,
                        visuals.fg_stroke,
                    ));
                }

                // Blank
                FilterCheckboxState::Coherent(None) => (),

                // Check
                FilterCheckboxState::Coherent(Some(true)) => {
                    ui.painter().add(egui::Shape::line(
                        vec![
                            egui::pos2(small_icon_rect.left(), small_icon_rect.center().y),
                            egui::pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                            egui::pos2(small_icon_rect.right(), small_icon_rect.top()),
                        ],
                        visuals.fg_stroke,
                    ));
                }

                // X
                FilterCheckboxState::Coherent(Some(false)) => {
                    ui.painter().add(egui::Shape::line_segment(
                        [small_icon_rect.left_top(), small_icon_rect.right_bottom()],
                        visuals.fg_stroke,
                    ));
                    ui.painter().add(egui::Shape::line_segment(
                        [small_icon_rect.left_bottom(), small_icon_rect.right_top()],
                        visuals.fg_stroke,
                    ));
                }
            }

            let mut x = rect.min.x + icon_width;

            // Color
            if let Some(color) = self.color {
                x += color_spacing;

                let color_pos = egui::pos2(x, rect.center().y - 0.5 * color_size.y);
                let color_rect =
                    egui::Rect::from_min_size(color_pos, color_size).expand(visuals.expansion);

                egui::color_picker::show_color_at(ui.painter(), color, color_rect);

                let rounding = visuals.rounding.at_most(2.0);
                ui.painter()
                    .rect_stroke(color_rect, rounding, (2.0, visuals.bg_fill));

                x += color_size.x;
            }

            x += label_spacing;

            // Label
            let text_pos = egui::pos2(x, rect.center().y - 0.5 * galley.size().y);
            ui.painter().galley(text_pos, galley, visuals.text_color());

            // `egui::Checkbox` has special handling for checkboxes without a
            // label, but we don't bother.
        }

        response
    }
}
