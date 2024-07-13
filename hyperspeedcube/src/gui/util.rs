use std::{any::Any, marker::PhantomData};

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

pub fn text_size(ui: &egui::Ui, text: &str) -> egui::Vec2 {
    let wrap = None;
    let max_width = f32::INFINITY;
    egui::WidgetText::from(text)
        .into_galley(ui, wrap, max_width, egui::TextStyle::Button)
        .size()
}

pub fn text_width(ui: &egui::Ui, text: &str) -> f32 {
    text_size(ui, text).x
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

/// Returns whether a widget of the given width will be put on the next line,
/// assuming a horizontal wrapping layout.
pub fn will_wrap_for_width(ui: &egui::Ui, width: f32) -> bool {
    ui.cursor().left() > ui.max_rect().left() && width >= ui.available_size_before_wrap().x
}
/// Wraps to the next line in a horizontal wrapping layout.
pub fn force_horizontal_wrap(ui: &mut egui::Ui) {
    // This is really hacky but I don't know anything else that works.
    let old_x_spacing = std::mem::take(&mut ui.spacing_mut().item_spacing.x);
    ui.add_space(ui.available_size_before_wrap().x);
    ui.allocate_exact_size(egui::vec2(1.0, 1.0), egui::Sense::hover());
    ui.add_space(-1.0);
    ui.spacing_mut().item_spacing.x = old_x_spacing;
}

pub fn wrap_if_needed_for_button(ui: &mut egui::Ui, label: &str) {
    let w = text_width(ui, label) + ui.spacing().button_padding.x * 2.0;
    if will_wrap_for_width(ui, w) {
        force_horizontal_wrap(ui);
    }
}

pub fn wrap_if_needed_for_color_button(ui: &mut egui::Ui) {
    if will_wrap_for_width(ui, ui.spacing().interact_size.x) {
        force_horizontal_wrap(ui);
    }
}

pub fn fake_popup<R>(
    ui: &mut egui::Ui,
    id: egui::Id,
    is_first_frame: bool,
    below_rect: egui::Rect,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<egui::InnerResponse<R>> {
    if ui.memory(|mem| mem.is_popup_open(id)) {
        let area_resp = egui::Area::new(unique_id!())
            .order(egui::Order::Middle)
            .fixed_pos(below_rect.left_bottom())
            .constrain_to(ui.ctx().available_rect())
            .sense(egui::Sense::hover())
            .show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    ui.set_height(BIG_ICON_BUTTON_SIZE);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        add_contents(ui)
                    })
                    .inner
                })
            });
        if area_resp.response.clicked_elsewhere() && !is_first_frame
            || ui.input(|input| input.key_pressed(egui::Key::Escape))
        {
            ui.memory_mut(|mem| mem.close_popup());
        }
        Some(area_resp.inner)
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct EguiTempFlag(EguiTempValue<()>);
impl EguiTempFlag {
    pub fn new(ui: &mut egui::Ui) -> Self {
        Self(EguiTempValue::new(ui))
    }
    pub fn get(&self) -> bool {
        self.0.get().is_some()
    }
    pub fn set(&self) {
        self.0.set(Some(()));
    }
    pub fn clear(&self) {
        self.0.set(None);
    }
}

#[derive(Debug, Clone)]
pub struct EguiTempValue<T> {
    pub ctx: egui::Context,
    pub id: egui::Id,
    _marker: PhantomData<T>,
}
impl<T: 'static + Any + Clone + Default + Send + Sync> EguiTempValue<T> {
    pub fn new(ui: &mut egui::Ui) -> Self {
        let ctx = ui.ctx().clone();
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        Self {
            ctx,
            id,
            _marker: PhantomData,
        }
    }
    pub fn get(&self) -> Option<T> {
        self.ctx.data(|data| data.get_temp::<T>(self.id))
    }
    pub fn set(&self, value: Option<T>) -> Option<T> {
        self.ctx.data_mut(|data| {
            let old_value = data.remove_temp::<T>(self.id);
            if let Some(v) = value {
                data.insert_temp::<T>(self.id, v);
            }
            old_value
        })
    }
    pub fn take(&self) -> Option<T> {
        self.set(None)
    }
}

/// Focuses a text edit and selects all its contents. Returns the ordinary
/// widget response.
pub fn focus_and_select_all(
    ui: &egui::Ui,
    mut r: egui::text_edit::TextEditOutput,
) -> egui::Response {
    r.response.request_focus();
    r.state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(r.galley.len()),
        )));
    r.state.store(ui.ctx(), r.response.id);
    r.response
}

/// Adds a label to the UI, centering it unless it needs multiple lines.
pub fn label_centered_unless_multiline(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    let widget_text = text.into();
    let galley = widget_text.clone().into_galley(
        ui,
        Some(true),
        ui.available_width(),
        egui::TextStyle::Body,
    );
    let is_multiline = galley.rows.len() > 1;
    ui.with_layout(
        egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(is_multiline),
        |ui| ui.label(widget_text),
    )
    .inner
}
