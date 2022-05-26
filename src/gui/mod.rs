macro_rules! unique_id {
    ($($args:tt)*) => {
        egui::Id::new((file!(), line!(), column!(), $($args)*))
    };
}

mod key_combo_popup;
mod keybinds_reference;
mod keybinds_table;
mod menu_bar;
mod prefs;
mod puzzle_controls;
mod status_bar;
mod util;

use crate::app::App;
pub(super) use key_combo_popup::{key_combo_popup_captures_event, key_combo_popup_handle_event};

use self::keybinds_table::KeybindsTable;

const GENERAL_KEYBINDS_TITLE: &str = "Keybinds";
const PUZZLE_KEYBINDS_TITLE: &str = "Puzzle Keybinds";

pub fn build(ctx: &egui::Context, app: &mut App) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, app));

    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| status_bar::build(ui, app));

    if Window::PrefsPanel.is_open(ctx) {
        egui::SidePanel::left("prefs_panel").show(ctx, |ui| prefs::build(ui, app));
    }

    if Window::PuzzleControlsPanel.is_open(ctx) {
        egui::SidePanel::right("puzzle_controls_panel")
            .show(ctx, |ui| puzzle_controls::build(ui, app));
    }

    if Window::KeybindsReference.is_open(ctx) {
        let alpha = app.prefs.gui.keybinds_reference.opacity;
        let frame = egui::Frame::window(&ctx.style());
        egui::Window::new("Keybinds Reference")
            .title_bar(false)
            .frame(frame.fill(frame.fill.linear_multiply(alpha)))
            .show(ctx, |ui| keybinds_reference::build(ui, app));
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

    let mut open = Window::About.is_open(ctx);
    egui::Window::new("About").open(&mut open).show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.strong(format!("{} v{}", crate::TITLE, env!("CARGO_PKG_VERSION")));
            ui.label(env!("CARGO_PKG_DESCRIPTION"));
            ui.hyperlink(env!("CARGO_PKG_REPOSITORY"));
            ui.label("");
            ui.label(format!("Created by {}", env!("CARGO_PKG_AUTHORS")));
            ui.label(format!("Licensed under {}", env!("CARGO_PKG_LICENSE")));
        });
    });
    Window::About.set_open(ctx, open);

    #[cfg(debug_assertions)]
    {
        let mut open = Window::Debug.is_open(ctx);
        let mut debug_info = crate::debug::FRAME_DEBUG_INFO.lock().unwrap();
        egui::Window::new("Debug values")
            .open(&mut open)
            .show(ctx, |ui| {
                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut *debug_info).code_editor());
                });
            });
        *debug_info = String::new();
        Window::Debug.set_open(ctx, open);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum Window {
    GeneralKeybinds,
    PuzzleKeybinds,
    PrefsPanel,
    PuzzleControlsPanel,
    KeybindsReference,
    About,
    Debug,
}
impl Window {
    fn id(self) -> egui::Id {
        egui::Id::new("hyperspeedcube::window_states").with(self)
    }
    fn toggle(self, ctx: &egui::Context) {
        *ctx.data()
            .get_persisted_mut_or_insert_with(self.id(), || self.default_is_open()) ^= true;
    }
    fn is_open(self, ctx: &egui::Context) -> bool {
        ctx.data()
            .get_persisted(self.id())
            .unwrap_or_else(|| self.default_is_open())
    }
    fn set_open(self, ctx: &egui::Context, open: bool) {
        ctx.data().insert_persisted(self.id(), open);
    }

    fn default_is_open(self) -> bool {
        match self {
            Window::PuzzleControlsPanel => true,
            _ => false,
        }
    }
}
