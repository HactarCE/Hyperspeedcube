use itertools::Itertools;

use crate::L;
use crate::app::App;
use crate::gui::markdown::md;
use crate::gui::util::hyperlink;

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.set_width(400.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            // We can't just use display `about_text()` directly because the
            // Markdown renderer can't center things properly.

            let l = &L.about;

            let version = env!("CARGO_PKG_VERSION");
            md(ui, format!("# {} v{}", crate::TITLE, version));

            ui.add_space(ui.spacing().item_spacing.y * 2.0);

            ui.label(env!("CARGO_PKG_DESCRIPTION"));
            hyperlink(ui, env!("CARGO_PKG_REPOSITORY"));
            let license = env!("CARGO_PKG_LICENSE").replace('-', " ");
            md(ui, l.licensed_under.with(&license));

            ui.add_space(ui.spacing().item_spacing.y * 2.0);

            md(ui, l.created_by);
            hyperlink(ui, l.created_by_url);

            ui.add_space(ui.spacing().item_spacing.y * 2.0);

            md(ui, l.dedicated_to);
            hyperlink(ui, l.dedicated_to_url);

            ui.add_space(ui.spacing().item_spacing.y * 2.0);

            md(ui, l.kofi_request);
            hyperlink(ui, l.kofi_request_url);

            ui.add_space(ui.spacing().item_spacing.y * 8.0);

            md(ui, l.acknowledgements.with(&markdown_puzzle_authors_list()));
        });
    });
}

/// Returns program info and credits in Markdown.
pub fn about_text() -> String {
    let mut ret = String::new();

    let l = &L.about;

    let version = env!("CARGO_PKG_VERSION");
    ret += &format!("# {} v{}", crate::TITLE, version);
    ret += "\n\n";

    ret += env!("CARGO_PKG_DESCRIPTION");
    ret += "  \n";
    ret += &format!("<{}>", env!("CARGO_PKG_REPOSITORY"));
    ret += "\n\n";

    let license = env!("CARGO_PKG_LICENSE").replace('-', " ");
    ret += &l.licensed_under.with(&license);
    ret += "\n\n";

    ret += l.created_by;
    ret += "  \n";
    ret += &format!("<{}>", l.created_by_url);
    ret += "\n\n";

    ret += l.dedicated_to;
    ret += "  \n";
    ret += &format!("<{}>", l.dedicated_to_url);
    ret += "\n\n";

    ret += l.kofi_request;
    ret += "  \n";
    ret += &format!("<{}>", l.kofi_request_url);
    ret += "\n\n";

    ret += &l.acknowledgements.with(&markdown_puzzle_authors_list());

    ret
}

fn markdown_puzzle_authors_list() -> String {
    hyperpuzzle::catalog()
        .authors()
        .into_iter()
        .map(|s| format!("- {s}"))
        .join("\n")
}
