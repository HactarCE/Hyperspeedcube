use std::any::Any;
use std::hash::Hash;
use std::marker::PhantomData;

/// Returns a globally-unique but layout-unstable ID.
///
/// It is safe to use this ID for widgets that store ephemeral state that does
/// not need to survive layout changes (e.g., drag interactions), but not
/// longer-lived state (e.g., text edit contents).
#[expect(clippy::disallowed_methods)]
pub fn consume_auto_id(ui: &mut egui::Ui) -> egui::Id {
    let id = ui.next_auto_id();
    ui.skip_ahead_auto_ids(1);
    id
}

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
            &mut hsc_prefs::PresetsList::default(),
            &mut hsc_prefs::ModifiedPreset::default(),
            &mut false,
        )
    };
}

/// Rounds an egui rectangle to the nearest pixel boundary and returns it along
/// with its width & height in pixels.
pub fn rounded_pixel_rect(
    ui: &egui::Ui,
    rect: egui::Rect,
    downscale_rate: u32,
) -> (egui::Rect, [u32; 2]) {
    let dpi = ui.pixels_per_point();

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

/// Reduces the font size of `text` as much as necessary to make it fit the
/// given width.
///
/// Returns `true` if the text was shrunken.
pub fn autosize_font(ui: &egui::Ui, text: &str, available_width: f32) -> (egui::RichText, bool) {
    let w = text_width(ui, text);
    if w < available_width {
        (text.into(), false)
    } else {
        let font_id = egui::TextStyle::Button.resolve(ui.style());
        // Adjust font size to fit
        let font_size = (font_id.size) * available_width / w;
        // Floor font size to 0.5
        let font_size = (font_size * 2.0).floor() / 2.0;
        (egui::RichText::new(text).size(font_size), true)
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
    r.state.store(ui, r.response.id);
    r.response.response
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

/// [`egui::Ui::hyperlink()`] but the link opens in a new tab.
///
/// This is equivalent to the following:
///
/// ```ignore
/// ui.add(egui::Hyperlink::new(url).open_in_new_tab(true))
/// ```
pub fn hyperlink(ui: &mut egui::Ui, url: impl ToString) -> egui::Response {
    ui.add(egui::Hyperlink::new(url).open_in_new_tab(true))
}

/// [`egui::Ui::hyperlink()`] but the link opens in a new tab.
///
/// This is equivalent to the following:
///
/// ```ignore
/// ui.add(egui::Hyperlink::from_label_and_url(text, url).open_in_new_tab(true))
/// ```
pub fn hyperlink_to(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
    url: impl ToString,
) -> egui::Response {
    ui.add(egui::Hyperlink::from_label_and_url(text, url).open_in_new_tab(true))
}

/// Paints a close button for a window and processes clicks on it.
///
/// This is stolen shamelessly from `egui::containers::window`.
pub fn close_button(ui: &mut egui::Ui, rect: egui::Rect) -> egui::Response {
    let close_id = consume_auto_id(ui); // only used for clicks; ok to depend on layout
    let response = ui.interact(rect, close_id, egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), "Close window")
    });

    ui.expand_to_include_rect(response.rect);

    let visuals = ui.style().interact(&response);
    let rect = rect.shrink(2.0).expand(visuals.expansion);
    let stroke = visuals.fg_stroke;
    ui.painter() // paints \
        .line_segment([rect.left_top(), rect.right_bottom()], stroke);
    ui.painter() // paints /
        .line_segment([rect.right_top(), rect.left_bottom()], stroke);
    response
}

pub fn menu_button_that_stays_open<'a, R>(
    ui: &mut egui::Ui,
    atoms: impl egui::IntoAtoms<'a>,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Response, Option<egui::InnerResponse<R>>) {
    let config = egui::containers::menu::MenuConfig::default()
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
    if egui::containers::menu::is_in_menu(ui) {
        egui::containers::menu::SubMenuButton::new(atoms)
            .config(config)
            .ui(ui, content)
    } else {
        egui::containers::menu::MenuButton::new(atoms)
            .config(config)
            .ui(ui, content)
    }
}

pub const MDI_STYLE_ICON_ROTATE_SQUARE: (&str, &str) = (
    "ROTATE_SQUARE.svg",
    "M 12 3 L 11 3.0605469 L 11 5.0800781 L 12 5 C 15.31 5 18 7.69 18 11 L 15 11 L 19 15 L 23 11 L 20 11 C 20 6.58 16.42 3 12 3 z M 5 9 C 3.9 9 3 9.9 3 11 L 3 18 C 3 19.11 3.9 20 5 20 L 12 20 C 13.11 20 14 19.11 14 18 L 14 11 C 14 9.8900001 13.11 9 12 9 L 5 9 z M 5 11 L 12 11 L 12 18 L 5 18 L 5 11 z",
);
