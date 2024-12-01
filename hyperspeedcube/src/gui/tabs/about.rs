use itertools::Itertools;

use crate::app::App;
use crate::gui::markdown::md;
use crate::L;

pub fn show(ui: &mut egui::Ui, _app: &mut App) {
    ui.set_width(400.0);
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.set_width(400.0);

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

        let author_list = crate::LIBRARY
            .with(|lib| lib.authors())
            .into_iter()
            .map(|s| format!("- {s}"))
            .join("\n");
        md(ui, L.about.with(&author_list));
    });
}
