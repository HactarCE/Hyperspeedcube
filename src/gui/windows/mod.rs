mod appearance_settings;
mod interaction_settings;
mod keybinds_reference;
pub mod keybinds_table;
mod piece_filters;
mod puzzle_controls;
mod view_settings;

use crate::app::App;

pub const FLOATING_WINDOW_OPACITY: f32 = 0.98;

pub const ALL: &'static [Window] = &[
    // Misc.
    ABOUT,
    #[cfg(debug_assertions)]
    DEBUG,
    // Tools
    KEYBINDS_REFERENCE,
    PUZZLE_CONTROLS,
    PIECE_FILTERS,
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
    build: keybinds_reference::build,
    cleanup: |_| (),
};

pub const PIECE_FILTERS: Window = Window {
    name: "Piece filters",
    location: Location::Floating,
    build: piece_filters::build,
    cleanup: piece_filters::cleanup,
};

pub const PUZZLE_CONTROLS: Window = Window {
    name: "Puzzle controls",
    location: Location::Floating,
    build: puzzle_controls::build,
    cleanup: puzzle_controls::cleanup,
};

pub const APPEARANCE_SETTINGS: Window = Window {
    name: "Appearance settings",
    location: Location::Floating,
    build: appearance_settings::build,
    cleanup: |_| (),
};

pub const INTERACTION_SETTINGS: Window = Window {
    name: "Interaction settings",
    location: Location::Floating,
    build: interaction_settings::build,
    cleanup: |_| (),
};

pub const VIEW_SETTINGS: Window = Window {
    name: "View settings",
    location: Location::Floating,
    build: view_settings::build,
    cleanup: |_| (),
};

pub const GLOBAL_KEYBINDS: Window = Window {
    name: "Global keybinds",
    location: Location::LeftSide,
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
                    .frame(egui::Frame::popup(&ctx.style()).multiply_with_opacity(opacity))
                    .show(ctx, |ui| (self.build)(ui, app));
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
