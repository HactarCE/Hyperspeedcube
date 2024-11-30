use std::{any::Any, marker::PhantomData};

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

pub fn text_width(ui: &egui::Ui, text: impl Into<egui::WidgetText>) -> f32 {
    text_size(ui, text).x
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
        let ctx = ui.ctx().clone();
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        Self {
            ctx,
            id,
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
