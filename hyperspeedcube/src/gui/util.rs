pub struct Access<T, U> {
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

pub fn set_widget_spacing_to_space_width(ui: &mut egui::Ui) {
    let space_width =
        ui.fonts(|fonts| fonts.glyph_width(&egui::TextStyle::Body.resolve(ui.style()), ' '));
    ui.spacing_mut().item_spacing.x = space_width;
}
pub fn subtract_space(ui: &mut egui::Ui) {
    let space_width =
        ui.fonts(|fonts| fonts.glyph_width(&egui::TextStyle::Body.resolve(ui.style()), ' '));
    ui.add_space(-space_width);
}
