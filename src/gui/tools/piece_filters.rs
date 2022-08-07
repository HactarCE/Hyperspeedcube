use crate::app::App;
use crate::puzzle::{traits::*, PieceType};

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();

    ui.collapsing("Types", |ui| {
        for (i, piece_type) in puzzle_type.piece_types().iter().enumerate() {
            ui.horizontal(|ui| {
                let mut s = format!("{}s", piece_type.name);
                s[0..1].make_ascii_uppercase();
                ui.label(&s);
                ui.with_layout(egui::Layout::right_to_left(), |ui| {
                    if ui
                        .button("👁")
                        .on_hover_text(format!("Show {}s", piece_type.name))
                        .clicked()
                    {
                        todo!("hide pice type")
                        // app.puzzle.set_piece_type_hidden(PieceType(i as _), false);
                    }
                    if ui
                        .button("ｘ")
                        .on_hover_text(format!("Hide {}s", piece_type.name))
                        .clicked()
                    {
                        todo!("hide piece type")
                        // app.puzzle.set_piece_type_hidden(PieceType(i as _), true);
                    }
                })
            });
        }
    });
}
