use egui::NumExt;

use crate::app::App;
use crate::gui::util::{self, ResponseExt};
use crate::preferences::DEFAULT_PREFS;
use crate::puzzle::{traits::*, Face};
use crate::serde_impl::hex_color;

pub(super) fn build_colors_section(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();
    let prefs = &mut app.prefs;

    let mut changed = false;

    // Face colors
    ui.strong("Faces");
    for (i, &face) in puzzle_type.faces().iter().enumerate() {
        let r = ui.add(resettable!(
            face.name,
            hex_color::to_str,
            (prefs.colors[(puzzle_type, Face(i as _))]),
            |value| |ui: &mut egui::Ui| ui.color_edit_button_srgba(value),
        ));
        changed |= r.changed();
    }

    ui.separator();

    // Special colors
    ui.strong("Special");
    let r = ui.add(resettable!(
        "Background",
        hex_color::to_str,
        (prefs.colors.background),
        |value| |ui: &mut egui::Ui| ui.color_edit_button_srgba(value),
    ));
    changed |= r.changed();
    let r = ui.add(resettable!(
        "Blindfolded stickers",
        hex_color::to_str,
        (prefs.colors.blind_face),
        |value| |ui: &mut egui::Ui| ui.color_edit_button_srgba(value),
    ));
    changed |= r.changed();
    let r = ui.add(util::CheckboxWithReset {
        label: "Blindfold mode",
        value: &mut prefs.colors.blindfold,
        reset_value: DEFAULT_PREFS.colors.blindfold,
    });
    changed |= r.changed();

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}
pub(super) fn build_graphics_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    // MSAA
    let r = ui
        .add(util::CheckboxWithReset {
            label: "MSAA",
            value: &mut prefs.gfx.msaa,
            reset_value: DEFAULT_PREFS.gfx.msaa,
        })
        .on_hover_explanation(
            "Multisample Anti-Aliasing",
            "Makes edges less jagged, \
             but may worsen performance.",
        );

    prefs.needs_save |= r.changed();
    if r.changed() {
        app.request_redraw_puzzle();
    }
}
pub(super) fn build_interaction_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    let mut changed = false;

    ui.add(util::CheckboxWithReset {
        label: "Confirm discard only when scrambled",
        value: &mut prefs.interaction.confirm_discard_only_when_scrambled,
        reset_value: DEFAULT_PREFS
            .interaction
            .confirm_discard_only_when_scrambled,
    })
    .on_hover_explanation(
        "",
        "When enabled, a confirmation dialog before \
         destructive actions (like resetting the puzzle) \
         is only shown when the puzzle has been fully \
         scrambled.",
    );

    ui.separator();

    let r = ui.add(resettable!(
        "Drag sensitivity",
        (prefs.interaction.drag_sensitivity),
        |value| {
            egui::DragValue::new(value)
                .fixed_decimals(2)
                .clamp_range(0.0..=3.0_f32)
                .speed(0.01)
        },
    ));
    changed |= r.changed();

    ui.separator();

    ui.collapsing("Animations", |ui| {
        let r = ui
            .add(util::CheckboxWithReset {
                label: "Dynamic twist speed",
                value: &mut prefs.interaction.dynamic_twist_speed,
                reset_value: DEFAULT_PREFS.interaction.dynamic_twist_speed,
            })
            .on_hover_explanation(
                "",
                "When enabled, the puzzle twists faster when \
             many moves are queued up. When all queued \
             moves are complete, the twist speed resets.",
            );
        changed |= r.changed();
        let r = ui.add(resettable!(
            "Twist duration",
            (prefs.interaction.twist_duration),
            |value| {
                let speed = value.at_least(0.1) / 100.0; // logarithmic speed
                egui::DragValue::new(value)
                    .fixed_decimals(2)
                    .clamp_range(0.0..=5.0_f32)
                    .speed(speed)
            },
        ));
        changed |= r.changed();
        let r = ui
            .add(resettable!(
                "Animation duration",
                (prefs.interaction.other_anim_duration),
                |value| {
                    let speed = value.at_least(0.01) / 100.0; // logarithmic speed
                    egui::DragValue::new(value)
                        .fixed_decimals(2)
                        .clamp_range(0.0..=1.0_f32)
                        .speed(speed)
                },
            ))
            .on_hover_explanation(
                "",
                "Number of seconds for other animations, \
             such as hiding a piece.",
            );
        changed |= r.changed();
    });

    prefs.needs_save |= changed;
}
pub(super) fn build_outlines_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    let mut changed = false;
    ui.strong("Colors");
    macro_rules! resettable_outline_color_edit {
        ($ui:ident, $name:ident, $label:expr) => {
            $ui.add(resettable!(
                $label,
                hex_color::to_str,
                (prefs.outlines.$name),
                |value| |ui: &mut egui::Ui| ui.color_edit_button_srgba(value),
            ))
        };
    }
    changed |= resettable_outline_color_edit!(ui, default_color, "Default").changed();
    changed |= resettable_outline_color_edit!(ui, hidden_color, "Hidden").changed();
    changed |= resettable_outline_color_edit!(ui, hovered_color, "Hovered").changed();
    // changed |= resettable_outline_color_edit!(ui, marked_color, "Marked").changed(); // TODO: enable or delete this
    changed |= resettable_outline_color_edit!(ui, selected_sticker_color, "Sel. sticker").changed();
    changed |= resettable_outline_color_edit!(ui, selected_piece_color, "Sel. piece").changed();

    ui.separator();

    ui.strong("Sizes");
    macro_rules! resettable_outline_size_dragvalue {
        ($ui:ident, $name:ident, $label:expr) => {
            $ui.add(resettable!($label, (prefs.outlines.$name), |value| {
                egui::DragValue::new(value)
                    .fixed_decimals(1)
                    .clamp_range(0.0..=5.0_f32)
                    .speed(0.01)
            }))
        };
    }
    changed |= resettable_outline_size_dragvalue!(ui, default_size, "Default").changed();
    changed |= resettable_outline_size_dragvalue!(ui, hidden_size, "Hidden").changed();
    changed |= resettable_outline_size_dragvalue!(ui, hovered_size, "Hovered").changed();
    changed |= resettable_outline_size_dragvalue!(ui, marked_size, "Marked").changed();
    changed |= resettable_outline_size_dragvalue!(ui, selected_size, "Selected").changed();

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}
pub(super) fn build_opacity_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    let mut changed = false;

    changed |= resettable_opacity_dragvalue!(ui, prefs.opacity.base, "Base").changed();
    changed |= resettable_opacity_dragvalue!(ui, prefs.opacity.ungripped, "Ungripped").changed();
    changed |= resettable_opacity_dragvalue!(ui, prefs.opacity.hidden, "Hidden").changed();
    changed |= resettable_opacity_dragvalue!(ui, prefs.opacity.selected, "Selected").changed();

    let r = ui
        .add(util::CheckboxWithReset {
            label: "Unhide grip",
            value: &mut prefs.opacity.unhide_grip,
            reset_value: DEFAULT_PREFS.opacity.unhide_grip,
        })
        .on_hover_explanation(
            "",
            "When enabled, gripping a face will temporarily \
             disable piece filters.",
        );
    changed |= r.changed();

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}
