use crate::app::App;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    super::ui_with_active_puzzle_view(ui, app, |ui, _app, view| {
        let puzzle = view.puzzle();

        ui.label(format!("ID: {}", puzzle.id));
        ui.label(format!("Name: {}", puzzle.name));
        ui.label(format!("Piece count: {}", puzzle.pieces.len()));
        ui.label(format!("Sticker count: {}", puzzle.stickers.len()));
        ui.label(format!("Color count: {}", puzzle.colors.len()));

        ui.add_space(10.0);
        ui.heading("Piece types");
        for piece_type in puzzle.piece_types.iter_values() {
            ui.label(format!("• {}", &piece_type.name));
        }

        ui.add_space(10.0);
        ui.heading("Colors");
        for color in puzzle.colors.iter_values() {
            let name = &color.name;
            let default_color_string = match &color.default_color {
                Some(default) => format!(" (default={default})"),
                None => String::new(),
            };
            ui.label(format!("• {name}{default_color_string}"));
        }
    });
}