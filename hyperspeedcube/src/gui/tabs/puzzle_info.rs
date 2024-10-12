use itertools::Itertools;

use crate::{
    app::App,
    gui::{
        markdown::{md, md_escape},
        util::EguiTempFlag,
    },
    L,
};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    app.active_puzzle_view.with_opt(|p| {
        let Some(p) = p else {
            ui.label(L.no_active_puzzle);
            return;
        };

        let puz = p.puzzle();

        // TODO: rework this UI

        ui.label(format!("ID: {}", puz.id));
        ui.label(format!("Version: {}", puz.version));
        ui.label(format!("Name: {}", puz.name));
        ui.label(format!("Aliases: {:?}", puz.aliases));
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

        ui.add_space(10.0);
        ui.heading("Tags");

        let show_excluded_flag = EguiTempFlag::new(ui);
        let mut show_excluded = show_excluded_flag.get();
        ui.checkbox(&mut show_excluded, "Show excluded");
        match show_excluded {
            true => show_excluded_flag.set(),
            false => show_excluded_flag.reset(),
        };

        let show_inherited_flag = EguiTempFlag::new(ui);
        let mut show_inherited = show_inherited_flag.get();
        ui.checkbox(&mut show_inherited, "Show inherited");
        match show_inherited {
            true => show_inherited_flag.set(),
            false => show_inherited_flag.reset(),
        };

        let markdown_text = puz
            .tags
            .iter()
            .sorted_by_key(|(tag, _value)| *tag)
            .filter_map(|(tag, value)| match value {
                hyperpuzzle::TagValue::False => show_excluded.then(|| format!("!{tag}")),
                hyperpuzzle::TagValue::True => Some(tag.clone()),
                hyperpuzzle::TagValue::Inherited => show_inherited.then(|| format!("({tag})")),
                hyperpuzzle::TagValue::Int(i) => Some(format!("{tag} = {i}")),
                hyperpuzzle::TagValue::Str(s) => {
                    Some(format!("{tag} = {}", md_escape(&format!("{s:?}"))))
                }
                hyperpuzzle::TagValue::StrList(vec) => {
                    Some(format!("{tag} = {}", md_escape(&format!("{vec:?}"))))
                }
                hyperpuzzle::TagValue::Puzzle(puz) => Some(format!("{tag} = {puz}")),
            })
            .map(|s| format!("- {s}\n"))
            .join("");
        md(ui, markdown_text);
    });
}
