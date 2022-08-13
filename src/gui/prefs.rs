use egui::NumExt;

use crate::app::App;
use crate::gui::util::{self, ResponseExt};
use crate::preferences::{OpacityPreferences, DEFAULT_PREFS};
use crate::puzzle::{traits::*, Face};

use super::util::PrefsUi;

pub(super) fn build_colors_section(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();
    let prefs = &mut app.prefs;

    let mut changed = false;
    let mut prefs_ui = util::PrefsUi {
        ui,
        current: &mut prefs.colors,
        defaults: &DEFAULT_PREFS.colors,
        changed: &mut changed,
    };

    prefs_ui.ui.strong("Faces");
    for (i, &face) in puzzle_type.faces().iter().enumerate() {
        prefs_ui.color(face.name, access!([(puzzle_type, Face(i as _))]));
    }

    prefs_ui.ui.separator();

    prefs_ui.ui.strong("Special");
    prefs_ui.color("Background", access!(.background));
    prefs_ui.color("Blindfolded stickers", access!(.blind_face));
    prefs_ui.checkbox("Blindfold mode", access!(.blindfold));

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}
pub(super) fn build_graphics_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    let mut changed = false;
    let mut prefs_ui = util::PrefsUi {
        ui,
        current: &mut prefs.gfx,
        defaults: &DEFAULT_PREFS.gfx,
        changed: &mut changed,
    };

    prefs_ui
        .checkbox("MSAA", access!(.msaa))
        .on_hover_explanation(
            "Multisample Anti-Aliasing",
            "Makes edges less jagged, \
             but may worsen performance.",
        );

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}
pub(super) fn build_interaction_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    let mut changed = false;
    let mut prefs_ui = util::PrefsUi {
        ui,
        current: &mut prefs.interaction,
        defaults: &DEFAULT_PREFS.interaction,
        changed: &mut changed,
    };

    prefs_ui
        .checkbox(
            "Confirm discard only when scrambled",
            access!(.confirm_discard_only_when_scrambled),
        )
        .on_hover_explanation(
            "",
            "When enabled, a confirmation dialog before \
             destructive actions (like resetting the puzzle) \
             is only shown when the puzzle has been fully \
             scrambled.",
        );

    prefs_ui.ui.separator();

    prefs_ui.float("Drag sensitivity", access!(.drag_sensitivity), |dv| {
        dv.fixed_decimals(2).clamp_range(0.0..=3.0_f32).speed(0.01)
    });

    prefs_ui.ui.separator();

    prefs_ui.collapsing("Animations", |mut prefs_ui| {
        prefs_ui
            .checkbox("Dynamic twist speed", access!(.dynamic_twist_speed))
            .on_hover_explanation(
                "",
                "When enabled, the puzzle twists faster when \
                 many moves are queued up. When all queued \
                 moves are complete, the twist speed resets.",
            );

        let speed = prefs_ui.current.twist_duration.at_least(0.1) / 100.0; // logarithmic speed
        prefs_ui.float("Twist duration", access!(.twist_duration), |dv| {
            dv.fixed_decimals(2).clamp_range(0.0..=5.0_f32).speed(speed)
        });

        let speed = prefs_ui.current.other_anim_duration.at_least(0.1) / 100.0; // logarithmic speed
        prefs_ui
            .float("Other animations", access!(.other_anim_duration), |dv| {
                dv.fixed_decimals(2).clamp_range(0.0..=1.0_f32).speed(speed)
            })
            .on_hover_explanation(
                "",
                "Number of seconds for other animations, \
                 such as hiding a piece.",
            );
    });

    prefs.needs_save |= changed;
}
pub(super) fn build_outlines_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    let mut changed = false;
    let mut prefs_ui = util::PrefsUi {
        ui,
        current: &mut prefs.outlines,
        defaults: &DEFAULT_PREFS.outlines,
        changed: &mut changed,
    };

    prefs_ui.ui.strong("Colors");
    prefs_ui.color("Default", access!(.default_color));
    prefs_ui.color("Hidden", access!(.hidden_color));
    prefs_ui.color("Hovered", access!(.hovered_color));
    prefs_ui.color("Sel. sticker", access!(.selected_sticker_color));
    prefs_ui.color("Sel. piece", access!(.selected_piece_color));

    prefs_ui.ui.separator();

    prefs_ui.ui.strong("Sizes");

    fn outline_size_dv<'a>(drag_value: egui::DragValue<'a>) -> egui::DragValue<'a> {
        drag_value
            .fixed_decimals(1)
            .clamp_range(0.0..=5.0_f32)
            .speed(0.01)
    }
    prefs_ui.float("Default", access!(.default_size), outline_size_dv);
    prefs_ui.float("Hidden", access!(.hidden_size), outline_size_dv);
    prefs_ui.float("Hovered", access!(.hovered_size), outline_size_dv);
    prefs_ui.float("Selected", access!(.selected_size), outline_size_dv);

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}
pub(super) fn build_opacity_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    let mut changed = false;
    let mut prefs_ui = util::PrefsUi {
        ui,
        current: &mut prefs.opacity,
        defaults: &DEFAULT_PREFS.opacity,
        changed: &mut changed,
    };

    prefs_ui.percent("Base", access!(.base));
    prefs_ui.percent("Ungripped", access!(.ungripped));
    prefs_ui.percent("Hidden", access!(.hidden));
    prefs_ui.percent("Selected", access!(.selected));
    build_unhide_grip_checkbox(&mut prefs_ui);

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}

pub(super) fn build_unhide_grip_checkbox(prefs_ui: &mut PrefsUi<OpacityPreferences>) {
    prefs_ui
        .checkbox("Unhide grip", access!(.unhide_grip))
        .on_hover_explanation(
            "",
            "When enabled, gripping a face will temporarily \
             disable piece filters.",
        );
}
