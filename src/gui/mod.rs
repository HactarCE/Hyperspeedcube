macro_rules! unique_id {
    ($($args:tt)*) => {
        egui::Id::new((file!(), line!(), column!(), $($args)*))
    };
}

mod key_combo_popup;
mod keybinds_table;
mod menu_bar;
mod side_bar;
mod status_bar;
mod util;

use crate::app::App;
use crate::puzzle::PuzzleControllerTrait;
pub(super) use key_combo_popup::key_combo_popup_handle_event;

use self::keybinds_table::KeybindsTable;

const GENERAL_KEYBINDS_TITLE: &str = "Keybinds";
const PUZZLE_KEYBINDS_TITLE: &str = "Puzzle Keybinds";

pub fn build(ctx: &egui::Context, app: &mut App) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, app));

    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| status_bar::build(ui, app));

    if Window::SidePanel.is_open(ctx) {
        egui::SidePanel::left("side_panel").show(ctx, |ui| side_bar::build(ui, app));
    }

    let puzzle_type = app.puzzle.ty();

    let mut open = Window::GeneralKeybinds.is_open(ctx);
    egui::Window::new(GENERAL_KEYBINDS_TITLE)
        .open(&mut open)
        .show(ctx, |ui| {
            let r = ui.add(KeybindsTable::new(app, keybinds_table::GeneralKeybinds));
            app.prefs.needs_save |= r.changed();
        });
    Window::GeneralKeybinds.set_open(ctx, open);

    let mut open = Window::PuzzleKeybinds.is_open(ctx);
    egui::Window::new(PUZZLE_KEYBINDS_TITLE)
        .open(&mut open)
        .show(ctx, |ui| {
            let r = ui.add(KeybindsTable::new(
                app,
                keybinds_table::PuzzleKeybinds(puzzle_type),
            ));
            app.prefs.needs_save |= r.changed();
        });
    Window::PuzzleKeybinds.set_open(ctx, open);

    key_combo_popup::build(ctx, app);

    #[cfg(debug_assertions)]
    {
        let mut debug_info = crate::debug::FRAME_DEBUG_INFO.lock().unwrap();
        if !debug_info.is_empty() {
            egui::Window::new("Debug values").show(ctx, |ui| {
                ui.add(egui::TextEdit::multiline(&mut *debug_info).code_editor());
            });
            *debug_info = String::new();
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum Window {
    GeneralKeybinds,
    PuzzleKeybinds,
    SidePanel,
    About,
    Debug,
}
impl Window {
    fn id(self) -> egui::Id {
        egui::Id::new("hyperspeedcube::window_states").with(self)
    }
    fn open(self, ctx: &egui::Context) {
        self.set_open(ctx, true);
    }
    fn close(self, ctx: &egui::Context) {
        self.set_open(ctx, false);
    }
    fn toggle(self, ctx: &egui::Context) {
        *ctx.data().get_persisted_mut_or_default::<bool>(self.id()) ^= true;
    }
    fn is_open(self, ctx: &egui::Context) -> bool {
        ctx.data().get_persisted(self.id()).unwrap_or(false)
    }
    fn set_open(self, ctx: &egui::Context, open: bool) {
        ctx.data().insert_persisted(self.id(), open);
    }
}
