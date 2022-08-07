mod piece_filters;
mod puzzle_controls;

use crate::app::App;

#[derive(Copy, Clone)]
pub struct ToolWindow {
    name: &'static str,
    build: fn(&mut egui::Ui, &mut App),
    cleanup: fn(&mut App),
}
impl ToolWindow {
    pub const PUZZLE_CONTROLS: Self = ToolWindow {
        name: "Puzzle controls",
        build: puzzle_controls::build,
        cleanup: puzzle_controls::cleanup,
    };
    pub const PIECE_FILTERS: Self = ToolWindow {
        name: "Piece filters",
        build: piece_filters::build,
        cleanup: piece_filters::cleanup,
    };

    pub const ALL: &'static [Self] = &[Self::PUZZLE_CONTROLS, Self::PIECE_FILTERS];

    fn id(self) -> egui::Id {
        unique_id!(self.name())
    }

    pub fn name(self) -> &'static str {
        self.name
    }

    pub fn toggle(self, ctx: &egui::Context) {
        *ctx.data()
            .get_persisted_mut_or_insert_with(self.id(), || false) ^= true;
    }
    pub fn is_open(self, ctx: &egui::Context) -> bool {
        ctx.data().get_persisted(self.id()).unwrap_or(false)
    }

    pub fn show(self, ui: &mut egui::Ui, app: &mut App) {
        if self.is_open(ui.ctx()) {
            let mut is_open = true;
            egui::Window::new(self.name())
                .collapsible(true)
                .open(&mut is_open)
                .frame(egui::Frame::popup(ui.style()).multiply_with_opacity(0.9))
                .show(ui.ctx(), |ui| (self.build)(ui, app));
            if !is_open {
                self.toggle(ui.ctx());
                (self.cleanup)(app);
            }
        }
    }
}
