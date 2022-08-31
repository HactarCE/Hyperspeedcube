use egui::NumExt;
use itertools::Itertools;
use std::borrow::Cow;
use std::hash::Hash;

use crate::puzzle::{rubiks_3d, rubiks_4d, traits::*, PuzzleTypeEnum};
use crate::serde_impl::hex_color;

const NONE_TEXT: &str = "-";
const NONE_TOOLTIP: &str = "Use the current grip";

pub(super) const EXPLANATION_TOOLTIP_WIDTH: f32 = 200.0;

pub(super) fn puzzle_select_menu(ui: &mut egui::Ui) -> Option<PuzzleTypeEnum> {
    let mut ret = None;

    let default = PuzzleTypeEnum::Rubiks3D {
        layer_count: rubiks_3d::DEFAULT_LAYER_COUNT,
    };
    let r = ui.menu_button(default.family_display_name(), |ui| {
        for layer_count in rubiks_3d::MIN_LAYER_COUNT..=rubiks_3d::MAX_LAYER_COUNT {
            let ty = PuzzleTypeEnum::Rubiks3D { layer_count };
            if ui.button(ty.name()).clicked() {
                ui.close_menu();
                ret = Some(ty);
            }
        }
    });
    if r.response.clicked() {
        ui.close_menu();
        ret = Some(default);
    }

    let default = PuzzleTypeEnum::Rubiks4D {
        layer_count: rubiks_4d::DEFAULT_LAYER_COUNT,
    };
    let r = ui.menu_button(default.family_display_name(), |ui| {
        for layer_count in rubiks_4d::LAYER_COUNT_RANGE {
            let ty = PuzzleTypeEnum::Rubiks4D { layer_count };
            if ui.button(ty.name()).clicked() {
                ui.close_menu();
                ret = Some(ty);
            }
        }
    });
    if r.response.clicked() {
        ui.close_menu();
        ret = Some(default);
    }

    ret
}

pub(super) trait ResponseExt {
    fn on_hover_explanation(self, strong_text: &str, detailed_message: &str) -> Self;
}
impl ResponseExt for egui::Response {
    fn on_hover_explanation(self, strong_text: &str, detailed_message: &str) -> Self {
        self.on_hover_ui(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(EXPLANATION_TOOLTIP_WIDTH, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    if !strong_text.is_empty() {
                        ui.strong(strong_text);
                    }
                    if !detailed_message.is_empty() {
                        ui.label(detailed_message);
                    }
                },
            );
        })
    }
}

pub(super) struct FancyComboBox<'a, T> {
    pub(super) combo_box: egui::ComboBox,
    pub(super) selected: &'a mut T,
    pub(super) options: Vec<(T, Cow<'a, str>)>,
}
impl<'a> FancyComboBox<'a, String> {
    pub(super) fn new<O: 'a + AsRef<str>>(
        id_source: impl Hash,
        selected: &'a mut String,
        options: impl IntoIterator<Item = &'a O>,
    ) -> Self {
        Self {
            combo_box: egui::ComboBox::from_id_source(id_source),
            selected,
            options: options
                .into_iter()
                .map(|s| s.as_ref())
                .map(|s| (s.to_owned(), s.into()))
                .collect(),
        }
    }
}
impl<'a> FancyComboBox<'a, Option<String>> {
    pub(super) fn new_optional<O: 'a + AsRef<str>>(
        id_source: impl Hash,
        selected: &'a mut Option<String>,
        options: impl IntoIterator<Item = &'a O>,
    ) -> Self {
        let mut options = options
            .into_iter()
            .map(|s| s.as_ref())
            .map(|s| (Some(s.to_owned()), s.into()))
            .collect_vec();
        options.insert(0, (None, NONE_TEXT.into()));
        Self {
            combo_box: egui::ComboBox::from_id_source(id_source),
            selected,
            options,
        }
    }
}
impl<T: Clone + PartialEq> egui::Widget for FancyComboBox<'_, T> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;

        let selected_text = self
            .options
            .iter()
            .find(|(opt, _)| opt == self.selected)
            .map(|(_, string)| string.as_ref())
            .unwrap_or(crate::util::INVALID_STR);

        let mut r = self
            .combo_box
            .selected_text(selected_text)
            .width_to_fit(ui, self.options.iter().map(|(_, string)| string.as_ref()))
            .show_ui(ui, |ui| {
                for (opt, string) in &self.options {
                    let is_selected = opt == self.selected;
                    let mut r = ui.selectable_label(is_selected, string.as_ref());
                    if string == NONE_TEXT {
                        r = r.on_hover_explanation("", NONE_TOOLTIP);
                    }
                    if r.clicked() {
                        *self.selected = opt.clone();
                        changed = true;
                    }
                }
            });

        if changed {
            r.response.mark_changed();
        }
        r.response
    }
}

pub(super) trait ComboBoxExt {
    /// Workaround for egui being *not fabulous* at sizing combo boxes.
    fn width_to_fit(
        self,
        ui: &egui::Ui,
        options: impl IntoIterator<Item = impl Into<egui::WidgetText>>,
    ) -> Self;
}
impl ComboBoxExt for egui::ComboBox {
    fn width_to_fit(
        self,
        ui: &egui::Ui,
        options: impl IntoIterator<Item = impl Into<egui::WidgetText>>,
    ) -> Self {
        let spacing = ui.spacing();

        let text_width = options
            .into_iter()
            .map(|option| {
                option
                    .into()
                    .into_galley(ui, Some(false), f32::INFINITY, egui::TextStyle::Button)
                    .size()
                    .x
            })
            .reduce(f32::max)
            .unwrap_or(0.0);

        let mut desired_width = text_width
            + spacing.item_spacing.x
            + f32::max(
                spacing.icon_width,
                spacing.window_margin.left
                    + spacing.scroll_bar_width
                    + spacing.window_margin.right
                    + 1.0, // not sure why, but text wraps without the +1.0
            );

        if ui.layout().horizontal_justify() {
            desired_width = desired_width.at_least(ui.available_width() - spacing.item_spacing.x);
        }

        desired_width = desired_width.at_least(spacing.interact_size.x - spacing.item_spacing.x);

        self.width(desired_width)
    }
}

macro_rules! enum_combobox {
    (
        $ui:expr,
        $id_source:expr,
        match ($mut_value:expr) {
            $($name:expr => $variant_start:ident $(:: $variant_cont:ident)* $(($($paren_args:tt)*))? $({$($brace_args:tt)*})?),*
            $(,)?
        }
    ) => {
        {
            let mut changed = false;

            let mut response = egui::ComboBox::from_id_source($id_source)
                .width_to_fit($ui, vec![$($name),*])
                .selected_text(match $mut_value {
                    $($variant_start $(:: $variant_cont)* { .. } => $name),*
                })
                .show_ui($ui, |ui| {
                    $(
                        let is_selected = matches!($mut_value, $variant_start $(:: $variant_cont)* { .. });
                        if ui.selectable_label(is_selected, $name).clicked() {
                            *($mut_value) = $variant_start $(:: $variant_cont)* $(($($paren_args)*))? $({$($brace_args)*})?;
                            changed = true;
                        }
                    )*
                })
                .response;

            if changed {
                response.mark_changed();
            }

            response
        }
    };
}

#[must_use]
pub struct WidgetWithReset<'a, V, W: 'a + egui::Widget, F: FnOnce(&'a mut V) -> W> {
    pub label: &'a str,
    pub value: &'a mut V,
    pub reset_value: V,
    pub reset_value_str: String,
    pub make_widget: F,
}
impl<'a, V, W, F> egui::Widget for WidgetWithReset<'a, V, W, F>
where
    V: PartialEq,
    W: 'a + egui::Widget,
    F: FnOnce(&'a mut V) -> W,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        with_reset_button(
            ui,
            self.value,
            self.reset_value,
            &self.reset_value_str,
            |ui, value| {
                let widget_resp =
                    ui.add_sized(ui.spacing().interact_size, (self.make_widget)(value));
                let mut label_resp = ui.label(self.label);

                // Return the label response so that the caller can add hover
                // text to the label if they want.
                if widget_resp.changed() {
                    label_resp.mark_changed();
                }
                label_resp
            },
        )
    }
}

fn with_reset_button<'a, T: PartialEq>(
    ui: &mut egui::Ui,
    value: &'a mut T,
    reset_value: T,
    reset_value_str: &str,
    widget: impl FnOnce(&mut egui::Ui, &'a mut T) -> egui::Response,
) -> egui::Response {
    ui.horizontal(|ui| {
        let reset_resp = reset_button(ui, value, reset_value, reset_value_str);
        let mut r = widget(ui, value);
        if reset_resp.clicked() {
            r.mark_changed();
        }
        r
    })
    .inner
}

pub(super) fn reset_button<T: PartialEq>(
    ui: &mut egui::Ui,
    value: &mut T,
    reset_value: T,
    reset_value_str: &str,
) -> egui::Response {
    let hover_text = match reset_value_str {
        "" => "Reset".to_owned(),
        s => format!("Reset to {}", s),
    };
    let r = ui
        .add_enabled(*value != reset_value, egui::Button::new("⟲"))
        .on_hover_text(&hover_text);
    if r.clicked() {
        *value = reset_value;
    }
    r
}

pub(super) struct PrefsUi<'a, T> {
    pub ui: &'a mut egui::Ui,
    pub current: &'a mut T,
    pub defaults: &'a T,

    pub changed: &'a mut bool,
}
impl<T> PrefsUi<'_, T> {
    fn add<'s, 'w, W>(&'s mut self, make_widget: impl FnOnce(&'w mut T) -> W) -> egui::Response
    where
        's: 'w,
        T: 'w,
        W: 'w + egui::Widget,
    {
        let r = self.ui.add(make_widget(self.current));
        *self.changed |= r.changed();
        r
    }

    pub fn collapsing<R>(
        &mut self,
        heading: impl Into<egui::WidgetText>,
        add_contents: impl FnOnce(PrefsUi<'_, T>) -> R,
    ) -> egui::CollapsingResponse<R> {
        self.ui.collapsing(heading, |ui| {
            add_contents(PrefsUi {
                ui,
                current: self.current,
                defaults: self.defaults,
                changed: self.changed,
            })
        })
    }

    pub fn checkbox(&mut self, label: &str, access: Access<T, bool>) -> egui::Response {
        let reset_value = *(access.get_ref)(self.defaults);
        self.add(|current| {
            |ui: &mut egui::Ui| {
                let value = (access.get_mut)(current);
                with_reset_button(ui, value, reset_value, "", |ui, value| {
                    ui.checkbox(value, label)
                })
            }
        })
    }

    pub fn float(
        &mut self,
        label: &str,
        access: Access<T, f32>,
        modify_widget: impl FnOnce(egui::DragValue) -> egui::DragValue,
    ) -> egui::Response {
        let reset_value = *(access.get_ref)(self.defaults);
        let reset_value_str = reset_value.to_string();
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str,
            make_widget: |value| modify_widget(egui::DragValue::new(value)),
        })
    }

    pub fn percent(&mut self, label: &str, access: Access<T, f32>) -> egui::Response {
        let reset_value = *(access.get_ref)(self.defaults);
        let reset_value_str = reset_value.to_string();
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str,
            make_widget: |value| {
                egui::DragValue::from_get_set(|new_value| {
                    if let Some(x) = new_value {
                        *value = x as f32 / 100.0;
                    }
                    *value as f64 * 100.0
                })
                .suffix("%")
                .fixed_decimals(0)
                .clamp_range(0.0..=100.0_f32)
                .speed(0.5)
            },
        })
    }

    pub fn angle(
        &mut self,
        label: &str,
        access: Access<T, f32>,
        modify_widget: impl FnOnce(egui::DragValue) -> egui::DragValue,
    ) -> egui::Response {
        let reset_value = *(access.get_ref)(self.defaults);
        let reset_value_str = format!("{}°", &reset_value);
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str,
            make_widget: |value| {
                modify_widget(egui::DragValue::new(value).suffix("°").fixed_decimals(0))
            },
        })
    }

    pub fn color(&mut self, label: &str, access: Access<T, egui::Color32>) -> egui::Response {
        let reset_value = *(access.get_ref)(self.defaults);
        let reset_value_str = hex_color::to_str(&reset_value);
        self.add(|current| WidgetWithReset {
            label,
            value: (access.get_mut)(current),
            reset_value,
            reset_value_str,
            make_widget: |value| |ui: &mut egui::Ui| ui.color_edit_button_srgba(value),
        })
    }
}

pub(super) struct Access<T, U> {
    pub get_ref: Box<dyn Fn(&T) -> &U>,
    pub get_mut: Box<dyn Fn(&mut T) -> &mut U>,
}
macro_rules! access {
    ($($suffix_tok:tt)*) => {
        crate::gui::util::Access {
            get_ref: Box::new(move |t| &t $($suffix_tok)*),
            get_mut: Box::new(move |t| &mut t $($suffix_tok)*),
        }
    }
}

pub(super) fn set_widget_spacing_to_space_widgth(ui: &mut egui::Ui) {
    let space_width = ui
        .fonts()
        .glyph_width(&egui::TextStyle::Body.resolve(ui.style()), ' ');
    ui.spacing_mut().item_spacing.x = space_width;
}
pub(super) fn subtract_space(ui: &mut egui::Ui) {
    let space_width = ui
        .fonts()
        .glyph_width(&egui::TextStyle::Body.resolve(ui.style()), ' ');
    ui.add_space(-space_width);
}
