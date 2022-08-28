mod appearance_settings;
mod interaction_settings;
mod keybinds_reference;
pub mod keybinds_table;
mod modifier_keys;
mod piece_filters;
mod puzzle_controls;
mod view_settings;

use crate::app::App;

pub const FLOATING_WINDOW_OPACITY: f32 = 0.98;
pub const PREFS_WINDOW_WIDTH: f32 = 240.0;
pub const ABOUT_WINDOW_WIDTH: f32 = 360.0;

pub const ALL: &'static [Window] = &[
    // Misc.
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
    GLOBAL_KEYBINDS,
    PUZZLE_KEYBINDS,
];

pub const ABOUT: Window = Window {
    name: "About",
    location: Location::Floating,
    fixed_width: Some(ABOUT_WINDOW_WIDTH),
    build: |ui, _app| {
        ui.vertical_centered(|ui| {
            ui.vertical_centered(|ui| {
                ui.strong(format!("{} v{}", crate::TITLE, env!("CARGO_PKG_VERSION")));
                ui.label(env!("CARGO_PKG_DESCRIPTION"));
                ui.hyperlink(env!("CARGO_PKG_REPOSITORY"));
                ui.label("");
                ui.label(format!("Created by {}", env!("CARGO_PKG_AUTHORS")));
                ui.label(format!("Licensed under {}", env!("CARGO_PKG_LICENSE")));
            });
        });
    },
    cleanup: |_| (),
};

#[cfg(debug_assertions)]
pub const DEBUG: Window = Window {
    name: "Debug values",
    location: Location::Floating,
    fixed_width: None,
    build: |ui, _app| {
        let mut debug_info = std::mem::take(&mut *crate::debug::FRAME_DEBUG_INFO.lock().unwrap());
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut debug_info).code_editor());
        });
    },
    cleanup: |_| *crate::debug::FRAME_DEBUG_INFO.lock().unwrap() = String::new(),
};

pub const KEYBINDS_REFERENCE: Window = Window {
    name: "Keybinds reference",
    location: Location::Floating,
    fixed_width: None,
    build: keybinds_reference::build,
    cleanup: |_| (),
};

pub const PUZZLE_CONTROLS: Window = Window {
    name: "Puzzle controls",
    location: Location::Floating,
    fixed_width: None,
    build: puzzle_controls::build,
    cleanup: puzzle_controls::cleanup,
};

pub const PIECE_FILTERS: Window = Window {
    name: "Piece filters",
    location: Location::Floating,
    fixed_width: None,
    build: piece_filters::build,
    cleanup: piece_filters::cleanup,
};

pub const MODIFIER_KEYS: Window = Window {
    name: "Modifier keys",
    location: Location::Floating,
    fixed_width: Some(0.0),
    build: modifier_keys::build,
    cleanup: |_| (),
};

pub const APPEARANCE_SETTINGS: Window = Window {
    name: "Appearance",
    location: Location::Floating,
    fixed_width: Some(PREFS_WINDOW_WIDTH),
    build: appearance_settings::build,
    cleanup: |_| (),
};

pub const INTERACTION_SETTINGS: Window = Window {
    name: "Interaction",
    location: Location::Floating,
    fixed_width: Some(PREFS_WINDOW_WIDTH),
    build: interaction_settings::build,
    cleanup: |_| (),
};

pub const VIEW_SETTINGS: Window = Window {
    name: "View",
    location: Location::Floating,
    fixed_width: Some(PREFS_WINDOW_WIDTH),
    build: view_settings::build,
    cleanup: |_| (),
};

pub const GLOBAL_KEYBINDS: Window = Window {
    name: "Global keybinds",
    location: Location::LeftSide,
    fixed_width: None,
    build: |ui, app| {
        let r = ui.add(keybinds_table::KeybindsTable::new(
            app,
            super::keybinds_set::GlobalKeybinds,
        ));
        app.prefs.needs_save |= r.changed();
    },
    cleanup: |_| (),
};

pub const PUZZLE_KEYBINDS: Window = Window {
    name: "Puzzle keybinds",
    location: Location::LeftSide,
    fixed_width: None,
    build: |ui, app| {
        let r = ui.add(keybinds_table::KeybindsTable::new(
            app,
            super::keybinds_set::PuzzleKeybinds(app.puzzle.ty()),
        ));
        app.prefs.needs_save |= r.changed();
    },
    cleanup: |_| (),
};

#[derive(Copy, Clone)]
pub struct Window {
    pub name: &'static str,
    pub location: Location,
    fixed_width: Option<f32>,
    build: fn(&mut egui::Ui, &mut App),
    cleanup: fn(&mut App),
}
impl Window {
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
            Location::Floating => {
                egui::Window::new(self.name)
                    .collapsible(true)
                    .open(&mut is_open)
                    .scroll2([false, true])
                    .frame(egui::Frame::popup(&ctx.style()).multiply_with_opacity(opacity))
                    .show(ctx, |ui| {
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
}
