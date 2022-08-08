mod keybinds_reference;
mod piece_filters;
mod puzzle_controls;
mod view_settings;

use crate::app::App;

pub const ALL_FLOATING: &'static [Window] = &[
    KEYBINDS_REFERENCE,
    PUZZLE_CONTROLS,
    PIECE_FILTERS,
    VIEW_SETTINGS,
];

pub const KEYBINDS_REFERENCE: Window = Window {
    name: "Keybinds reference",
    build: keybinds_reference::build,
    cleanup: |_| (),
};

pub const PIECE_FILTERS: Window = Window {
    name: "Piece filters",
    build: piece_filters::build,
    cleanup: piece_filters::cleanup,
};

pub const PUZZLE_CONTROLS: Window = Window {
    name: "Puzzle controls",
    build: puzzle_controls::build,
    cleanup: puzzle_controls::cleanup,
};

pub const VIEW_SETTINGS: Window = Window {
    name: "View settings",
    build: view_settings::build,
    cleanup: |_| (),
};

#[derive(Copy, Clone)]
pub struct Window {
    name: &'static str,
    build: fn(&mut egui::Ui, &mut App),
    cleanup: fn(&mut App),
}
impl Window {
    fn id(self) -> egui::Id {
        unique_id!(self.name())
    }

    pub fn name(self) -> &'static str {
        self.name
    }

    pub fn is_open(self, ctx: &egui::Context) -> bool {
        ctx.data().get_persisted(self.id()).unwrap_or(false)
    }
    pub fn set_open(self, ctx: &egui::Context, is_open: bool) {
        ctx.data().insert_persisted(self.id(), is_open);
    }

    pub fn show(self, ui: &mut egui::Ui, app: &mut App) {
        let opacity = if self.id() == KEYBINDS_REFERENCE.id() {
            app.prefs.info.keybinds_reference.opacity
        } else {
            0.95
        };

        let mut is_open = self.is_open(ui.ctx());

        egui::Window::new(self.name())
            .collapsible(true)
            .open(&mut is_open)
            .frame(egui::Frame::popup(ui.style()).multiply_with_opacity(opacity))
            .show(ui.ctx(), |ui| (self.build)(ui, app));
        self.set_open(ui.ctx(), is_open);
        if !is_open {
            (self.cleanup)(app);
        }
    }

    pub fn menu_button_toggle(self, ui: &mut egui::Ui) {
        let mut is_open = self.is_open(ui.ctx());
        if ui.checkbox(&mut is_open, self.name()).changed() {
            self.set_open(ui.ctx(), is_open);
        }
    }
}
