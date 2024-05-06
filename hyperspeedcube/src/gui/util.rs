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

/// Rounds an egui rectangle to the nearest pixel boundary and returns the
/// rounded egui rectangle, along with its width & height in pixels.
pub fn rounded_pixel_rect(
    ui: &egui::Ui,
    rect: egui::Rect,
    downscale_rate: u32,
) -> (egui::Rect, [u32; 2]) {
    let dpi = ui.ctx().pixels_per_point();

    // Round rectangle to pixel boundary for crisp image.
    let mut pixels_rect = rect;
    pixels_rect.set_left((dpi * pixels_rect.left()).ceil());
    pixels_rect.set_bottom((dpi * pixels_rect.bottom()).floor());
    pixels_rect.set_right((dpi * pixels_rect.right()).floor());
    pixels_rect.set_top((dpi * pixels_rect.top()).ceil());

    // Convert back from pixel coordinates to egui coordinates.
    let mut egui_rect = pixels_rect;
    *egui_rect.left_mut() /= dpi;
    *egui_rect.bottom_mut() /= dpi;
    *egui_rect.right_mut() /= dpi;
    *egui_rect.top_mut() /= dpi;

    let pixel_size = [
        pixels_rect.width() as u32 / downscale_rate,
        pixels_rect.height() as u32 / downscale_rate,
    ];
    (egui_rect, pixel_size)
}
