use crate::app::App;
use crate::puzzle::PuzzleControllerTrait;

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

pub(super) use key_combo_popup::key_combo_popup_handle_event;

const GENERAL_KEYBINDS_TITLE: &str = "Keybinds";
const PUZZLE_KEYBINDS_TITLE: &str = "Puzzle Keybinds";

pub fn build(ctx: &egui::Context, app: &mut App) {
    let window_state_id = egui::Id::new("hyperspeedcube::window_state");

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, app));

    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| status_bar::build(ui, app));

    let open = ctx
        .data()
        .get_persisted(window_state_id.with("side_panel"))
        .unwrap_or(true);
    if open {
        egui::SidePanel::left("side_panel").show(ctx, |ui| side_bar::build(ui, app));
    }

    let puzzle_type = app.puzzle.ty();

    let edit_key_combo_index = keybinds_table::build(
        ctx,
        GENERAL_KEYBINDS_TITLE,
        &mut app.prefs.general_keybinds,
        (),
        &mut app.prefs.needs_save,
    );
    if let Some(i) = edit_key_combo_index {
        key_combo_popup::open(
            ctx,
            app,
            move |app| &mut app.prefs.general_keybinds[i].key,
            true,
        );
    }

    let edit_key_combo_index = keybinds_table::build(
        ctx,
        PUZZLE_KEYBINDS_TITLE,
        &mut app.prefs.puzzle_keybinds[puzzle_type],
        app.puzzle.ty(),
        &mut app.prefs.needs_save,
    );
    if let Some(i) = edit_key_combo_index {
        key_combo_popup::open(
            ctx,
            app,
            move |app| &mut app.prefs.puzzle_keybinds[puzzle_type][i].key,
            false,
        );
    }

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

fn toggle_general_keybinds(ctx: &egui::Context) {
    let id = keybinds_table::window_id(GENERAL_KEYBINDS_TITLE);
    *ctx.data().get_persisted_mut_or_default::<bool>(id) ^= true;
}
fn toggle_puzzle_keybinds(ctx: &egui::Context) {
    let id = keybinds_table::window_id(PUZZLE_KEYBINDS_TITLE);
    *ctx.data().get_persisted_mut_or_default::<bool>(id) ^= true;
}
