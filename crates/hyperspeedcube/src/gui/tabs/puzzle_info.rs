use hyperpuzzle::prelude::*;
use itertools::Itertools;

use crate::L;
use crate::app::App;
use crate::gui::markdown::{md, md_escape};
use crate::gui::util::EguiTempFlag;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    app.active_puzzle.with_opt_view(|view| {
        let Some(view) = view else {
            ui.label(L.no_active_puzzle);
            return;
        };

        let puz = view.puzzle();

        // TODO: rework this UI

        ui.label(format!("ID: {}", puz.meta.id));
        ui.label(format!("Version: {}", puz.meta.version));
        ui.label(format!("Name: {}", puz.meta.name));
        ui.label(format!("Aliases: {:?}", puz.meta.aliases));
        ui.label(format!("Piece count: {}", puz.pieces.len()));
        ui.label(format!("Sticker count: {}", puz.stickers.len()));
        ui.label(format!("Color count: {}", puz.colors.names.len()));

        ui.add_space(10.0);
        ui.heading("Piece types");
        for piece_type in puz.piece_types.iter_values() {
            ui.label(format!("• {}", &piece_type.name));
        }

        ui.add_space(10.0);
        ui.heading("Colors");
        for (name, display) in std::iter::zip(
            puz.colors.names.iter_values(),
            puz.colors.display_names.iter_values(),
        ) {
            let NameSpec {
                preferred,
                spec,
                canonical,
            } = name;
            ui.label(format!(
                "• spec={spec:?}, \
                   preferred={preferred:?}, \
                   canonical={canonical:?}, \
                   display={display:?}"
            ));
        }

        ui.add_space(10.0);
        ui.heading("Axes");
        for name in puz.axes().names.iter_values() {
            let NameSpec {
                preferred,
                spec,
                canonical,
            } = name;
            ui.label(format!(
                "• spec={spec:?}, \
                   preferred={preferred:?}, \
                   canonical={canonical:?}"
            ));
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
            .meta
            .tags
            .iter()
            .sorted_by_key(|(tag, _value)| *tag)
            .filter_map(|(tag, value)| match value {
                TagValue::False => show_excluded.then(|| format!("!{tag}")),
                TagValue::True => Some(tag.to_string()),
                TagValue::Inherited => show_inherited.then(|| format!("({tag})")),
                TagValue::Int(i) => Some(format!("{tag} = {i}")),
                TagValue::Str(s) => Some(format!("{tag} = {}", md_escape(&format!("{s:?}")))),
                TagValue::StrList(vec) => {
                    Some(format!("{tag} = {}", md_escape(&format!("{vec:?}"))))
                }
                TagValue::Puzzle(puz) => Some(format!("{tag} = {puz}")),
            })
            .map(|s| format!("- {s}\n"))
            .join("");
        md(ui, markdown_text);
    });
}
