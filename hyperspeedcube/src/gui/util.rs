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
    let spacing = text_spacing(ui);
    ui.spacing_mut().item_spacing = spacing;
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.set_row_height(spacing.y);
}
/// Returns a vector containing the width and height of a space.
pub fn text_spacing(ui: &egui::Ui) -> egui::Vec2 {
    ui.fonts(|fonts| {
        let font_id = egui::TextStyle::Body.resolve(ui.style());
        let space_width = fonts.glyph_width(&font_id, ' ');
        let line_height = fonts.row_height(&font_id);
        egui::vec2(space_width, line_height)
    })
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

pub fn text_width(ui: &egui::Ui, text: &str) -> f32 {
    let wrap = None;
    let max_width = f32::INFINITY;
    let text_size = egui::WidgetText::from(text)
        .into_galley(ui, wrap, max_width, egui::TextStyle::Button)
        .size();
    text_size.x
}

pub fn bullet_list(ui: &mut egui::Ui, list_elements: &[&str]) {
    // TODO: proper markdown renderer
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.x /= 2.0;
        ui.spacing_mut().item_spacing.y = 3.0;
        for &elem in list_elements {
            // TODO: should this be wrapped?
            ui.horizontal_wrapped(|ui| {
                ui.label("•");
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::text::LayoutJob::single_section(
                        elem.to_string(),
                        body_text_format(ui),
                    ))
                });
            });
        }
    });
}

pub fn body_text_format(ui: &egui::Ui) -> egui::TextFormat {
    egui::TextFormat {
        color: ui.visuals().text_color(),
        font_id: egui::TextStyle::Body.resolve(ui.style()),
        ..Default::default()
    }
}
pub fn strong_text_format(ui: &egui::Ui) -> egui::TextFormat {
    egui::TextFormat {
        color: ui.visuals().strong_text_color(),
        font_id: egui::TextStyle::Body.resolve(ui.style()),
        ..Default::default()
    }
}

pub const BIG_ICON_BUTTON_SIZE: f32 = 22.0;
pub fn big_icon_button<'a>(icon: &str) -> egui::Button<'a> {
    egui::Button::new(icon).min_size(egui::Vec2::splat(BIG_ICON_BUTTON_SIZE))
}
