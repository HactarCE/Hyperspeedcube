mod about;
mod keybind_sets;
mod keybinds_reference;
mod keybinds_table;
mod modifier_keys;
mod mousebinds_table;
mod piece_filters;
mod puzzle_controls;
mod settings;
mod welcome;

use crate::app::App;
pub(crate) use about::*;
pub(crate) use keybind_sets::*;
pub(crate) use keybinds_reference::*;
pub(crate) use keybinds_table::*;
pub(crate) use modifier_keys::*;
pub(crate) use mousebinds_table::*;
pub(crate) use piece_filters::*;
pub(crate) use puzzle_controls::*;
pub(crate) use settings::*;
pub(crate) use welcome::*;

pub const FLOATING_WINDOW_OPACITY: f32 = 0.98;
pub const PREFS_WINDOW_WIDTH: f32 = 240.0;
pub const ABOUT_WINDOW_WIDTH: f32 = 360.0;
pub const WELCOME_WINDOW_WIDTH: f32 = 540.0;

pub const ALL: &[Window] = &[
    // Misc.
    WELCOME,
    ABOUT,
    #[cfg(debug_assertions)]
    DEBUG,
    // Tools
    KEYBINDS_REFERENCE,
    PUZZLE_CONTROLS,
    PIECE_FILTERS,
    MODIFIER_KEYS,
    // Settings
    APPEARANCE_SETTINGS,
    INTERACTION_SETTINGS,
    VIEW_SETTINGS,
    // Keybinds
    KEYBIND_SETS,
    GLOBAL_KEYBINDS,
    PUZZLE_KEYBINDS,
    MOUSEBINDS,
];

#[cfg(debug_assertions)]
pub const DEBUG: Window = Window {
    name: "Debug values",
    location: Location::Floating,
    fixed_width: None,
    vscroll: true,
    build: |ui, _app| {
        let mut debug_info = std::mem::take(&mut *crate::debug::FRAME_DEBUG_INFO.lock().unwrap());
        ui.add(egui::TextEdit::multiline(&mut debug_info).code_editor());
    },
    cleanup: |_| *crate::debug::FRAME_DEBUG_INFO.lock().unwrap() = String::new(),
};

#[derive(Copy, Clone)]
pub struct Window {
    pub name: &'static str,
    pub location: Location,
    fixed_width: Option<f32>,
    vscroll: bool,
    build: fn(&mut egui::Ui, &mut App),
    cleanup: fn(&mut App),
}
impl Window {
    const DEFAULT: Self = Self {
        name: "<unnamed>",
        location: Location::Floating,
        fixed_width: None,
        vscroll: false,
        build: |_, _| (),
        cleanup: |_| (),
    };

    fn id(self) -> egui::Id {
        unique_id!(self.name)
    }

    pub fn is_open(self, ctx: &egui::Context) -> bool {
        ctx.data().get_persisted(self.id()).unwrap_or(false)
    }
    pub fn set_open(self, ctx: &egui::Context, is_open: bool) {
        if is_open && self.location == Location::LeftSide {
            // Close other windows in the same location.
            for window in ALL {
                if window.location == self.location {
                    window.set_open(ctx, false);
                }
            }
        }

        ctx.data().insert_persisted(self.id(), is_open);
    }

    pub fn show(self, ctx: &egui::Context, app: &mut App) {
        let opacity = if self.id() == KEYBINDS_REFERENCE.id() {
            app.prefs.info.keybinds_reference.opacity
        } else {
            FLOATING_WINDOW_OPACITY
        };

        let mut is_open = self.is_open(ctx);

        match self.location {
            Location::Floating | Location::Centered => {
                let mut w = egui::Window::new(self.name).open(&mut is_open);
                if self.location == Location::Centered {
                    w = w
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .collapsible(false)
                        .resizable(false);
                } else {
                    w = w
                        .collapsible(true)
                        .scroll2([false, self.vscroll])
                        .resizable(self.fixed_width.is_none() || self.vscroll)
                        .frame(egui::Frame::popup(&ctx.style()).multiply_with_opacity(opacity));
                }
                w.show(ctx, |ui| {
                    if let Some(w) = self.fixed_width {
                        ui.set_min_width(w);
                        ui.set_max_width(w);
                    }
                    (self.build)(ui, app);
                });
            }
            Location::LeftSide => {
                super::side_bar::build(ctx, self.name, &mut is_open, |ui| (self.build)(ui, app));
            }
        }

        self.set_open(ctx, is_open);
        if !is_open {
            (self.cleanup)(app);
        }
    }

    pub fn menu_button_toggle(self, ui: &mut egui::Ui) {
        let mut is_open = self.is_open(ui.ctx());
        if ui.checkbox(&mut is_open, self.name).changed() {
            self.set_open(ui.ctx(), is_open);
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Location {
    Floating,
    LeftSide,
    Centered,
}
