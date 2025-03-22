use hyperprefs::{ColorScheme, GlobalColorPalette};
use hyperpuzzle::ColorSystem;

use crate::L;
use crate::app::App;
use crate::gui::components::PresetsUi;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let id = unique_id!();

    let palette = &app.prefs.color_palette;

    app.active_puzzle.with_opt_view(|view| {
        if let Some(view) = view {
            let mut changed = false;

            let color_system = &view.puzzle().colors;
            let presets = &mut app.prefs.color_schemes.get_mut(color_system).schemes;
            let current = &mut view.colors;

            // Ensure that the active color scheme is valid.
            changed |= palette
                .ensure_color_scheme_is_valid_for_color_system(&mut current.value, color_system);

            let presets_ui = PresetsUi::new(id, presets, current, &mut changed);
            show_contents(ui, palette, color_system, presets_ui, &mut view.temp_colors);

            app.prefs.needs_save |= changed;
        } else {
            ui.disable();
            let color_system = ColorSystem::new_empty();
            show_contents(ui, palette, &color_system, dummy_presets_ui!(id), &mut None);
        }
    });
}

fn show_contents(
    ui: &mut egui::Ui,
    palette: &GlobalColorPalette,
    color_system: &ColorSystem,
    presets_ui: PresetsUi<'_, ColorScheme>,
    temp_colors_override: &mut Option<ColorScheme>,
) {
    presets_ui
        .with_text(&L.presets.color_schemes)
        .with_help_contents(&crate::gui::components::get_color_schemes_markdown(true))
        .show(ui, Some(&color_system.name), |mut prefs_ui| {
            let (prefs, ui) = prefs_ui.split();

            let mut colors_ui = crate::gui::components::ColorsUi::new(palette)
                .clickable(false)
                .drag_puzzle_colors(ui, true);

            let (changed, temp_scheme) =
                colors_ui.show_compact_palette(ui, Some((prefs.current, color_system)), None);
            *prefs.changed |= changed;
            if let Some(temp_scheme) = temp_scheme {
                *temp_colors_override = Some(temp_scheme);
            }
        });
}
