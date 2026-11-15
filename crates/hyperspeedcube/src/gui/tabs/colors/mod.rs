use crate::L;
use crate::gui::{App, EguiValue};

mod color_scheme;
mod global_color_palette;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
enum ColorsTab {
    #[default]
    Schemes,
    GlobalPalette,
}

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut tab = EguiValue::<ColorsTab>::load_or_default(ui, unique_id!());
    ui.group(|ui| {
        egui::ScrollArea::horizontal()
            .id_salt("colors_tab_select")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut *tab, ColorsTab::Schemes, L.colors.color_schemes);
                    ui.selectable_value(
                        &mut *tab,
                        ColorsTab::GlobalPalette,
                        L.colors.global_palette,
                    );
                });
            });
    });
    ui.add_space(ui.spacing().item_spacing.x - ui.spacing().item_spacing.y);
    match *tab {
        ColorsTab::Schemes => color_scheme::show(ui, app),
        ColorsTab::GlobalPalette => global_color_palette::show(ui, unique_id!(), app),
    }
}
