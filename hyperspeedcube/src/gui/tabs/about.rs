use itertools::Itertools;

use crate::app::App;
use crate::gui::markdown::md;
use crate::L;

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    ui.set_width(400.0);
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.set_width(400.0);

        // We can't just use display `about_text()` directly because the
        // Markdown renderer can't center things properly.

        let version = env!("CARGO_PKG_VERSION");
        md(ui, format!("# {} v{}", crate::TITLE, version));

        let license = env!("CARGO_PKG_LICENSE").replace('-', " ");
        md(ui, L.licensed_under.with(&license));

        ui.label(env!("CARGO_PKG_DESCRIPTION"));
        ui.hyperlink(env!("CARGO_PKG_REPOSITORY"));

        ui.add_space(ui.spacing().item_spacing.y);

        md(ui, L.created_by);
        ui.hyperlink(L.created_by_url);

        ui.add_space(ui.spacing().item_spacing.y);

        md(ui, L.about.with(&markdown_puzzle_authors_list()));
    });
}

/// Returns program info and credits in Markdown.
pub fn about_text() -> String {
    let mut ret = String::new();

    let version = env!("CARGO_PKG_VERSION");
    ret += &format!("# {} v{}", crate::TITLE, version);
    ret += "\n\n";

    let license = env!("CARGO_PKG_LICENSE").replace('-', " ");
    ret += &L.licensed_under.with(&license);
    ret += "\n\n";

    ret += env!("CARGO_PKG_DESCRIPTION");
    ret += "  \n";
    ret += &format!("<{}>", env!("CARGO_PKG_REPOSITORY"));
    ret += "\n\n";

    ret += L.created_by;
    ret += "  \n";
    ret += &format!("<{}>", L.created_by_url);
    ret += "\n\n";

    ret += &L.about.with(&markdown_puzzle_authors_list());

    ret
}

fn markdown_puzzle_authors_list() -> String {
    hyperpuzzle::catalog()
        .authors()
        .into_iter()
        .map(|s| format!("- {s}"))
        .join("\n")
}
