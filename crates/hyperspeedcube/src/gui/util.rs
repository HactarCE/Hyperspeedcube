use std::any::Any;
use std::hash::Hash;
use std::marker::PhantomData;

pub struct Access<T, U> {
    pub get_ref: Box<dyn Fn(&T) -> &U>,
    pub get_mut: Box<dyn Fn(&mut T) -> &mut U>,
}
impl<T, U> Access<T, U> {
    pub fn get<'a>(&self, t: &'a T) -> &'a U {
        (self.get_ref)(t)
    }
    pub fn get_mut<'a>(&self, t: &'a mut T) -> &'a mut U {
        (self.get_mut)(t)
    }
}
macro_rules! access {
    ($($suffix_tok:tt)*) => {
        crate::gui::util::Access {
            get_ref: Box::new(move |t| &t $($suffix_tok)*),
            get_mut: Box::new(move |t| &mut t $($suffix_tok)*),
        }
    }
}
macro_rules! access_option {
    ($default:expr, $($suffix_tok:tt)*) => {
        crate::gui::util::Access {
            get_ref: Box::new(move |t| t $($suffix_tok)* .as_ref().unwrap_or(&$default)),
            get_mut: Box::new(move |t| t $($suffix_tok)* .get_or_insert($default)),
        }
    };
}

// TODO: is this necessary and/or good?
macro_rules! dummy_presets_ui {
    ($id:expr) => {
        $crate::gui::components::PresetsUi::new(
            $id,
            &mut hyperprefs::PresetsList::default(),
            &mut hyperprefs::ModifiedPreset::default(),
            &mut false,
        )
    };
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

pub fn text_size(ui: &egui::Ui, text: impl Into<egui::WidgetText>) -> egui::Vec2 {
    let wrap = None;
    let max_width = f32::INFINITY;
    text.into()
        .into_galley(ui, wrap, max_width, egui::TextStyle::Button)
        .size()
}
pub fn text_size_ctx(ctx: &egui::Context, text: impl Into<egui::WidgetText>) -> egui::Vec2 {
    text.into()
        .into_galley_impl(
            ctx,
            &ctx.style(),
            egui::text::TextWrapping::no_max_width(),
            egui::TextStyle::Button.into(),
            egui::Align::TOP,
        )
        .size()
}

pub fn text_width(ui: &egui::Ui, text: impl Into<egui::WidgetText>) -> f32 {
    text_size(ui, text).x
}
pub fn text_width_ctx(ctx: &egui::Context, text: impl Into<egui::WidgetText>) -> f32 {
    text_size_ctx(ctx, text).x
}

/// Reduces the font size of `text` as much as necessary to make it fit the given width.
///
/// Returns `true` if the text was shrunken.
pub fn autosize_font(ui: &egui::Ui, text: &str, available_width: f32) -> (egui::RichText, bool) {
    let w = text_width(ui, text);
    if w < available_width {
        (text.into(), false)
    } else {
        let mut font_id = egui::TextStyle::Button.resolve(ui.style());
        // Adjust font size to fit
        let font_size = (font_id.size) * available_width / w;
        // Floor font size to 0.5
        let font_size = (font_size * 2.0).floor() / 2.0;
        (egui::RichText::new(text).size(font_size), true)
    }
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

/// Boolean flag stored in egui temporary memory.
#[derive(Debug, Clone)]
pub struct EguiTempFlag(EguiTempValue<()>);
impl EguiTempFlag {
    /// Returns a temporary value based on the current position in the UI.
    pub fn new(ui: &mut egui::Ui) -> Self {
        Self(EguiTempValue::new(ui))
    }
    /// Returns the current value of the flag.
    pub fn get(&self) -> bool {
        self.0.get().is_some()
    }
    /// Sets the flag to `true`, returning the old value.
    pub fn set(&self) -> bool {
        self.0.set(Some(())).is_some()
    }
    /// Resets the flag to `false`, returning the old value.
    pub fn reset(&self) -> bool {
        self.0.set(None).is_some()
    }
}

/// Arbitrary value stored in egui temporary memory.
#[derive(Debug, Clone)]
pub struct EguiTempValue<T> {
    pub ctx: egui::Context,
    pub id: egui::Id,
    _marker: PhantomData<T>,
}
impl<T: 'static + Any + Clone + Default + Send + Sync> EguiTempValue<T> {
    /// Returns a temporary value based on the current position in the UI.
    pub fn new(ui: &mut egui::Ui) -> Self {
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        Self::from_ctx_and_id(ui.ctx(), id)
    }
    /// Returns a temporary value based on `id_source`.
    pub fn from_ctx_and_id(ctx: &egui::Context, id_source: impl Hash) -> Self {
        Self {
            ctx: ctx.clone(),
            id: egui::Id::new(id_source),
            _marker: PhantomData,
        }
    }
    /// Returns the currently stored value.
    pub fn get(&self) -> Option<T> {
        self.ctx.data(|data| data.get_temp::<T>(self.id))
    }
    /// Sets the value, returning the old value.
    pub fn set(&self, value: Option<T>) -> Option<T> {
        self.ctx.data_mut(|data| {
            let old_value = data.remove_temp::<T>(self.id);
            if let Some(v) = value {
                data.insert_temp::<T>(self.id, v);
            }
            old_value
        })
    }
    /// Deletes the value, returning the old value.
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
        Some(egui::TextWrapMode::Wrap),
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

/// Same as [`egui::Response::clicked_elsewhere()`], but considers all
/// pointer-down events.
pub fn clicked_elsewhere(ui: &egui::Ui, r: &egui::Response) -> bool {
    ui.input(|input| {
        input.pointer.any_pressed()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|pos| !r.rect.contains(pos))
    })
}

pub fn centered_popup_area<R>(
    ctx: &egui::Context,
    rect: egui::Rect,
    id: egui::Id,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Area::new(id)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .constrain_to(rect)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, contents).inner
        })
}

/// Sets styling to be similar to a menu.
///
/// Stolen from
/// [`egui/src/menu.rs`](https://github.com/emilk/egui/blob/0.31.1/crates/egui/src/menu.rs#L77).
pub fn set_menu_style(style: &mut egui::Style) {
    style.spacing.button_padding = egui::vec2(2.0, 0.0);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
}

pub trait GuiRoundingExt {
    fn floor_to_pixels_ui(self, ctx: &egui::Context) -> Self;
    fn ceil_to_pixels_ui(self, ctx: &egui::Context) -> Self;
}

impl GuiRoundingExt for f32 {
    fn floor_to_pixels_ui(self, ctx: &egui::Context) -> Self {
        use egui::emath::GUI_ROUNDING;

        let pixels_per_point = ctx.pixels_per_point();
        let rounded_to_pixel = ((self * pixels_per_point).ceil() / pixels_per_point);
        (rounded_to_pixel / GUI_ROUNDING).ceil() * GUI_ROUNDING
    }

    fn ceil_to_pixels_ui(self, ctx: &egui::Context) -> Self {
        use egui::emath::GUI_ROUNDING;

        let pixels_per_point = ctx.pixels_per_point();
        let rounded_to_pixel = ((self * pixels_per_point).ceil() / pixels_per_point);
        (rounded_to_pixel / GUI_ROUNDING).ceil() * GUI_ROUNDING
    }
}

impl GuiRoundingExt for egui::Vec2 {
    fn floor_to_pixels_ui(self, ctx: &egui::Context) -> Self {
        let Self { x, y } = self;
        egui::vec2(x.floor_to_pixels_ui(ctx), y.floor_to_pixels_ui(ctx))
    }

    fn ceil_to_pixels_ui(self, ctx: &egui::Context) -> Self {
        let Self { x, y } = self;
        egui::vec2(x.ceil_to_pixels_ui(ctx), y.ceil_to_pixels_ui(ctx))
    }
}

impl GuiRoundingExt for egui::Pos2 {
    fn floor_to_pixels_ui(self, ctx: &egui::Context) -> Self {
        let Self { x, y } = self;
        egui::pos2(x.floor_to_pixels_ui(ctx), y.floor_to_pixels_ui(ctx))
    }

    fn ceil_to_pixels_ui(self, ctx: &egui::Context) -> Self {
        let Self { x, y } = self;
        egui::pos2(x.ceil_to_pixels_ui(ctx), y.ceil_to_pixels_ui(ctx))
    }
}

pub trait GuiRoundingExtRect {
    fn round_to_pixels_ui_inward(self, ctx: &egui::Context) -> Self;
    fn round_to_pixels_ui_outward(self, ctx: &egui::Context) -> Self;
}

impl GuiRoundingExtRect for egui::Rect {
    fn round_to_pixels_ui_inward(self, ctx: &egui::Context) -> Self {
        let Self { min, max } = self;
        egui::Rect::from_min_max(min.ceil_to_pixels_ui(ctx), max.floor_to_pixels_ui(ctx))
    }

    fn round_to_pixels_ui_outward(self, ctx: &egui::Context) -> Self {
        let Self { min, max } = self;
        egui::Rect::from_min_max(min.floor_to_pixels_ui(ctx), max.ceil_to_pixels_ui(ctx))
    }
}

macro_rules! mdi {
    ($name:ident) => {{
        const PATH_DATA: &[u8] = ::material_design_icons::$name.as_bytes();
        ::egui::Image::from_bytes(
            concat!("MDI_", stringify!($name), ".svg"),
            &const {
                $crate::gui::util::const_concat_3::<
                    { PATH_DATA.len() + $crate::gui::util::SVG_EXTRA_LEN },
                >(
                    $crate::gui::util::SVG_PRE,
                    PATH_DATA,
                    $crate::gui::util::SVG_POST,
                )
            },
        )
        .fit_to_original_size(0.5)
    }};
}

#[doc(hidden)]
pub const SVG_PRE: &[u8] =
    br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="white" d=""#;
#[doc(hidden)]
pub const SVG_POST: &[u8] = br#"" /></svg>"#;
#[doc(hidden)]
pub const SVG_EXTRA_LEN: usize = SVG_PRE.len() + SVG_POST.len();

#[doc(hidden)]
pub const fn const_copy_slice_from_to(src: &[u8], dst: &mut [u8], start: &mut usize) {
    let mut i = 0;
    while i < src.len() {
        dst[*start + i] = src[i];
        i += 1;
    }
    *start += i;
}

#[doc(hidden)]
pub const fn const_concat_3<const OUT: usize>(a: &[u8], b: &[u8], c: &[u8]) -> [u8; OUT] {
    let mut ret = [0; OUT];
    let mut i = 0;
    const_copy_slice_from_to(a, &mut ret, &mut i);
    const_copy_slice_from_to(b, &mut ret, &mut i);
    const_copy_slice_from_to(c, &mut ret, &mut i);
    ret
}
