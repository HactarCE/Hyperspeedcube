use egui::NumExt;

use super::util::{self, ResponseExt};
use crate::app::App;
use crate::preferences::DEFAULT_PREFS;
use crate::puzzle::{traits::*, Face};
use crate::serde_impl::hex_color;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    ui.spacing_mut().interact_size.x *= 1.5;
    ui.style_mut().wrap = Some(false);

    ui.heading("Preferences");
    ui.separator();
    egui::ScrollArea::new([false, true]).show(ui, |ui| {
        ui.collapsing("Graphics", |ui| build_graphics_section(ui, app));
        ui.collapsing("View", |ui| build_view_section(ui, app));
        ui.collapsing("Outlines", |ui| build_outlines_section(ui, app));
        ui.collapsing("Colors", |ui| build_colors_section(ui, app));
        ui.collapsing("Interaction", |ui| {
            build_interaction_section(ui, app);

            ui.separator();

            ui.strong("Keybinds");
            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::Center),
                |ui| {
                    if ui.button("Edit general keybinds").clicked() {
                        super::Window::GeneralKeybinds.toggle(ui.ctx());
                    }
                    if ui.button("Edit puzzle keybinds").clicked() {
                        super::Window::PuzzleKeybinds.toggle(ui.ctx());
                    }
                },
            )
        });
    });
}

macro_rules! resettable {
    (
        $label:expr,
        ($prefs:ident $($prefs_tok:tt)*),
        $make_widget:expr $(,)?
    ) => {
        resettable!($label, "{}", ($prefs $($prefs_tok)*), $make_widget)
    };
    (
        $label:expr,
        $format_str:tt,
        ($prefs:ident $($prefs_tok:tt)*),
        $make_widget:expr $(,)?
    ) => {
        resettable!($label, |x| format!($format_str, x), ($prefs $($prefs_tok)*), $make_widget)
    };
    (
        $label:expr,
        $format_fn:expr,
        ($prefs:ident $($prefs_tok:tt)*),
        $make_widget:expr $(,)?
    ) => {{
        let value = &mut $prefs $($prefs_tok)*;
        let reset_value = &DEFAULT_PREFS $($prefs_tok)*;
        #[allow(clippy::redundant_closure_call)]
        let reset_value_str = ($format_fn)(reset_value);
        crate::gui::util::WidgetWithReset {
            label: $label,
            value,
            reset_value: reset_value.clone(),
            reset_value_str,
            make_widget: $make_widget,
        }
    }};
}

fn build_colors_section(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();
    let prefs = &mut app.prefs;

    let mut changed = false;

    // Opacity
    ui.strong("Opacity");
    let r = ui.add(resettable!(
        "Default",
        |x| format!("{:.0}%", x * 100.0),
        (prefs.colors.default_opacity),
        util::make_percent_drag_value,
    ));
    changed |= r.changed();
    let r = ui
        .add(resettable!(
            "Hidden",
            |x| format!("{:.0}%", x * 100.0),
            (prefs.colors.hidden_opacity),
            util::make_percent_drag_value,
        ))
        .on_hover_explanation(
            "",
            "Opacity of hidden stickers (multiplied \
             by default sticker opacity)",
        );
    changed |= r.changed();
    let r = ui.add(resettable!(
        "Hovered",
        |x| format!("{:.0}%", x * 100.0),
        (prefs.colors.hovered_opacity),
        util::make_percent_drag_value,
    ));
    changed |= r.changed();

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

    ui.separator();

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

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}
fn build_graphics_section(ui: &mut egui::Ui, app: &mut App) {
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
fn build_view_section(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();
    let prefs = &mut app.prefs;

    let mut changed = false;

    ui.strong("View angle");
    // Pitch
    let r = ui.add(resettable!(
        "Pitch",
        "{}°",
        (prefs.view[puzzle_type].pitch),
        |value| util::make_degrees_drag_value(value).clamp_range(-90.0..=90.0),
    ));
    changed |= r.changed();
    // Yaw
    let r = ui.add(resettable!(
        "Yaw",
        "{}°",
        (prefs.view[puzzle_type].yaw),
        |value| util::make_degrees_drag_value(value).clamp_range(-45.0..=45.0),
    ));
    changed |= r.changed();

    ui.separator();
    ui.strong("Projection");
    // Scale
    let r = ui.add(resettable!(
        "Scale",
        (prefs.view[puzzle_type].scale),
        |value| {
            let speed = *value / 100.0; // logarithmic speed
            egui::DragValue::new(value)
                .fixed_decimals(2)
                .clamp_range(0.1..=5.0_f32)
                .speed(speed)
        },
    ));
    changed |= r.changed();
    // 4D FOV
    let r = ui.add(resettable!(
        "4D FOV",
        "{}°",
        (prefs.view[puzzle_type].fov_4d),
        |value| {
            util::make_degrees_drag_value(value)
                .clamp_range(1.0..=120.0)
                .speed(0.5)
        },
    ));
    changed |= r.changed();
    // 3D FOV
    let r = ui.add(resettable!(
        "3D FOV",
        "{}°",
        (prefs.view[puzzle_type].fov_3d),
        |value| {
            util::make_degrees_drag_value(value)
                .clamp_range(-120.0..=120.0)
                .speed(0.5)
        },
    ));
    changed |= r.changed();

    ui.separator();

    ui.strong("Geometry");
    // Face spacing
    let r = ui.add(resettable!(
        "Face spacing",
        (prefs.view[puzzle_type].face_spacing),
        |value| {
            egui::DragValue::new(value)
                .fixed_decimals(2)
                .clamp_range(0.0..=0.9_f32)
                .speed(0.005)
        },
    ));
    changed |= r.changed();
    // Sticker spacing
    let r = ui.add(resettable!(
        "Sticker spacing",
        (prefs.view[puzzle_type].sticker_spacing),
        |value| {
            egui::DragValue::new(value)
                .fixed_decimals(2)
                .clamp_range(0.0..=0.9_f32)
                .speed(0.005)
        },
    ));
    changed |= r.changed();

    ui.separator();

    ui.strong("Lighting");
    // Pitch
    let r = ui.add(resettable!(
        "Pitch",
        "{}°",
        (prefs.view[puzzle_type].light_pitch),
        |value| util::make_degrees_drag_value(value).clamp_range(-90.0..=90.0),
    ));
    changed |= r.changed();
    // Yaw
    let r = ui.add(resettable!(
        "Yaw",
        "{}°",
        (prefs.view[puzzle_type].light_yaw),
        |value| util::make_degrees_drag_value(value).clamp_range(-180.0..=180.0),
    ));
    changed |= r.changed();
    // Intensity
    let r = ui.add(resettable!(
        "Intensity",
        |x| format!("{:.0}%", x * 100.0),
        (prefs.view[puzzle_type].light_intensity),
        util::make_percent_drag_value,
    ));
    changed |= r.changed();

    prefs.needs_save |= changed;
}
fn build_interaction_section(ui: &mut egui::Ui, app: &mut App) {
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

    let r = ui
        .add(util::CheckboxWithReset {
            label: "Highlight whole piece",
            value: &mut prefs.interaction.highlight_piece_on_hover,
            reset_value: DEFAULT_PREFS.interaction.highlight_piece_on_hover,
        })
        .on_hover_explanation(
            "",
            "When enabled, hovering over a sticker \
             highlights all stickers on the same piece.",
        );
    changed |= r.changed();

    ui.separator();

    ui.strong("Animations");
    let r = ui
        .add(resettable!(
            "Selection fade duration",
            (prefs.interaction.selection_fade_duration),
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
            "Number of seconds for a sticker to fade from \
             fully opaque to fully transparent, or vice versa",
        );
    changed |= r.changed();
    let r = ui
        .add(resettable!(
            "Hover fade duration",
            (prefs.interaction.hover_fade_duration),
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
            "Number of seconds for the sticker \
             hover indicator to fade away.",
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

    prefs.needs_save |= changed;
}
fn build_outlines_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    let mut changed = false;

    ui.strong("Outline size");
    let outline_size_drag_value_fn = |value| {
        egui::DragValue::new(value)
            .fixed_decimals(1)
            .clamp_range(0.0..=5.0_f32)
            .speed(0.01)
    };
    let r = ui.add(resettable!(
        "Default",
        (prefs.outlines.default_size),
        outline_size_drag_value_fn,
    ));
    changed |= r.changed();
    let r = ui.add(resettable!(
        "Hidden",
        (prefs.outlines.hidden_size),
        outline_size_drag_value_fn,
    ));
    changed |= r.changed();
    let r = ui.add(resettable!(
        "Hovered",
        (prefs.outlines.hovered_size),
        outline_size_drag_value_fn,
    ));
    changed |= r.changed();

    ui.separator();

    ui.strong("Outline color");
    let r = ui.add(resettable!(
        "Default",
        hex_color::to_str,
        (prefs.outlines.default_color),
        |value| |ui: &mut egui::Ui| ui.color_edit_button_srgba(value),
    ));
    changed |= r.changed();
    let r = ui.add(resettable!(
        "Hidden",
        hex_color::to_str,
        (prefs.outlines.hidden_color),
        |value| |ui: &mut egui::Ui| ui.color_edit_button_srgba(value),
    ));
    changed |= r.changed();
    let r = ui.add(resettable!(
        "Hovered",
        hex_color::to_str,
        (prefs.outlines.hovered_color),
        |value| |ui: &mut egui::Ui| ui.color_edit_button_srgba(value),
    ));
    changed |= r.changed();

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}
