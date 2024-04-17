use std::borrow::Cow;
use std::hash::Hash;

use itertools::Itertools;

use crate::gui::ext::*;

const NONE_TEXT: &str = "-";
const NONE_TOOLTIP: &str = "Use the current grip";

pub struct FancyComboBox<'a, T> {
    pub combo_box: egui::ComboBox,
    pub selected: &'a mut T,
    pub options: Vec<(T, Cow<'a, str>)>,
}
impl<'a> FancyComboBox<'a, String> {
    pub fn new<O: 'a + AsRef<str>>(
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
    pub fn new_optional<O: 'a + AsRef<str>>(
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
