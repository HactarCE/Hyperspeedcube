use crate::{app::App, L};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    app.active_puzzle_view.with_opt(|p| {
        let Some(p) = p else {
            ui.label(L.no_active_puzzle);
            return;
        };

        let puz = p.puzzle();

        // TODO: rework this UI

        ui.label(format!("ID: {}", puz.id));
        ui.label(format!("Name: {}", puz.name));
        ui.label(format!("Piece count: {}", puz.pieces.len()));
        ui.label(format!("Sticker count: {}", puz.stickers.len()));
        ui.label(format!("Color count: {}", puz.colors.list.len()));

        ui.add_space(10.0);
        ui.heading("Piece types");
        for piece_type in puz.piece_types.iter_values() {
            ui.label(format!("• {}", &piece_type.name));
        }

        ui.add_space(10.0);
        ui.heading("Colors");
        for color in puz.colors.list.iter_values() {
            let name = &color.name;
            let display = &color.display;
            ui.label(format!("• {name} = {display}"));
        }
    });
}
