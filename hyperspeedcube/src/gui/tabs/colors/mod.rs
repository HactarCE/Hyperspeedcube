use crate::gui::{util::EguiTempValue, App};

mod color_scheme;
mod global_color_palette;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
enum ColorsTab {
    #[default]
    Schemes,
    GlobalPalette,
}

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let tab_state = EguiTempValue::<ColorsTab>::new(ui);
    let mut tab = tab_state.get().unwrap_or_default();
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.selectable_value(&mut tab, ColorsTab::Schemes, "Color schemes");
            ui.selectable_value(&mut tab, ColorsTab::GlobalPalette, "Global color palette");
        });
    });
    tab_state.set(Some(tab));
    ui.add_space(ui.spacing().item_spacing.x - ui.spacing().item_spacing.y);
    match tab {
        ColorsTab::Schemes => color_scheme::show(ui, app),
        ColorsTab::GlobalPalette => global_color_palette::show(ui, app),
    }
}
