use egui::NumExt;

use super::util::{self, ResponseExt};
use crate::app::App;
use crate::preferences::DEFAULT_PREFS;
use crate::puzzle::{traits::*, Face, ProjectionType};
use crate::serde_impl::hex_color;

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
        let reset_value = &crate::preferences::DEFAULT_PREFS $($prefs_tok)*;
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

macro_rules! resettable_opacity_dragvalue {
    ($ui:ident, $prefs:ident.opacity.$name:ident, $label:expr) => {
        $ui.add(resettable!(
            $label,
            |x| format!("{:.0}%", x * 100.0),
            ($prefs.opacity.$name),
            crate::gui::util::make_percent_drag_value,
        ))
    };
}

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    ui.spacing_mut().interact_size.x *= 1.5;
    ui.style_mut().wrap = Some(false);

    ui.heading("Preferences");
    ui.separator();
    egui::ScrollArea::new([false, true]).show(ui, |ui| {
        ui.collapsing("Graphics", |ui| build_graphics_section(ui, app));
        ui.collapsing("View", |ui| build_view_section(ui, app));
        ui.collapsing("Outlines", |ui| build_outlines_section(ui, app));
        ui.collapsing("Opacity", |ui| build_opacity_section(ui, app));
        ui.collapsing("Colors", |ui| build_colors_section(ui, app));
        ui.collapsing("Interaction", |ui| {
            build_interaction_section(ui, app);

            ui.separator();

            ui.strong("Keybinds");
            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::Center),
                |ui| {
                    if ui.button("Edit global keybinds").clicked() {
                        super::Window::GlobalKeybinds.toggle(ui.ctx());
                    }
                    if ui.button("Edit puzzle keybinds").clicked() {
                        super::Window::PuzzleKeybinds.toggle(ui.ctx());
                    }
                },
            )
        });
    });
}

fn build_colors_section(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();
    let prefs = &mut app.prefs;

    let mut changed = false;

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
    let proj_ty = app.puzzle.ty().projection_type();
    let prefs = &mut app.prefs;

    let mut changed = false;

    ui.strong("View angle");
    // Pitch
    let r = ui.add(resettable!(
        "Pitch",
        "{}°",
        (prefs[proj_ty].pitch),
        |value| util::make_degrees_drag_value(value).clamp_range(-90.0..=90.0),
    ));
    changed |= r.changed();
    // Yaw
    let r = ui.add(resettable!("Yaw", "{}°", (prefs[proj_ty].yaw), |value| {
        util::make_degrees_drag_value(value).clamp_range(-45.0..=45.0)
    }));
    changed |= r.changed();

    ui.separator();
    ui.strong("Projection");
    // Scale
    let r = ui.add(resettable!("Scale", (prefs[proj_ty].scale), |value| {
        let speed = *value / 100.0; // logarithmic speed
        egui::DragValue::new(value)
            .fixed_decimals(2)
            .clamp_range(0.1..=5.0_f32)
            .speed(speed)
    }));
    changed |= r.changed();
    if proj_ty == ProjectionType::_4D {
        // 4D FOV
        let r = ui.add(resettable!(
            "4D FOV",
            "{}°",
            (prefs.view_4d.fov_4d),
            |value| {
                util::make_degrees_drag_value(value)
                    .clamp_range(1.0..=120.0)
                    .speed(0.5)
            },
        ));
        changed |= r.changed();
    }
    // 3D FOV
    let r = ui.add(resettable!(
        "3D FOV",
        "{}°",
        (prefs[proj_ty].fov_3d),
        |value| {
            util::make_degrees_drag_value(value)
                .clamp_range(-120.0..=120.0)
                .speed(0.5)
        },
    ));
    changed |= r.changed();

    ui.separator();

    ui.strong("Geometry");
    if proj_ty == ProjectionType::_3D {
        // Show front faces
        ui.add(util::CheckboxWithReset {
            label: "Show frontfaces",
            value: &mut prefs.view_3d.show_frontfaces,
            reset_value: DEFAULT_PREFS.view_3d.show_frontfaces,
        });
        // Show back faces
        changed |= r.changed();
        ui.add(util::CheckboxWithReset {
            label: "Show backfaces",
            value: &mut prefs.view_3d.show_backfaces,
            reset_value: DEFAULT_PREFS.view_3d.show_backfaces,
        });
        changed |= r.changed();
    }
    // Face spacing
    let r = ui.add(resettable!(
        "Face spacing",
        (prefs[proj_ty].face_spacing),
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
        (prefs[proj_ty].sticker_spacing),
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
        (prefs[proj_ty].light_pitch),
        |value| util::make_degrees_drag_value(value).clamp_range(-90.0..=90.0),
    ));
    changed |= r.changed();
    // Yaw
    let r = ui.add(resettable!(
        "Yaw",
        "{}°",
        (prefs[proj_ty].light_yaw),
        |value| util::make_degrees_drag_value(value).clamp_range(-180.0..=180.0),
    ));
    changed |= r.changed();
    // Directional
    let r = ui.add(resettable!(
        "Directional",
        |x| format!("{:.0}%", x * 100.0),
        (prefs[proj_ty].light_directional),
        util::make_percent_drag_value,
    ));
    changed |= r.changed();
    // Ambient
    let r = ui.add(resettable!(
        "Ambient",
        |x| format!("{:.0}%", x * 100.0),
        (prefs[proj_ty].light_ambient),
        util::make_percent_drag_value,
    ));
    changed |= r.changed();

    prefs.needs_save |= changed;
}
fn build_interaction_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    let mut changed = false;

    ui.add(util::CheckboxWithReset {
        label: "Unhide grip",
        value: &mut prefs.interaction.unhide_grip,
        reset_value: DEFAULT_PREFS.interaction.unhide_grip,
    })
    .on_hover_explanation(
        "",
        "When enabled, gripping a face will temporarily \
         disable piece filters.",
    );

    ui.separator();

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

    ui.strong("Animations");
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

    ui.separator();
    ui.strong("View angle drag");
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

    prefs.needs_save |= changed;
}
fn build_outlines_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    let mut changed = false;

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

    ui.separator();
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
    changed |= resettable_outline_color_edit!(ui, marked_color, "Marked").changed();
    changed |= resettable_outline_color_edit!(ui, selected_sticker_color, "Sel. sticker").changed();
    changed |= resettable_outline_color_edit!(ui, selected_piece_color, "Sel. piece").changed();

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}
fn build_opacity_section(ui: &mut egui::Ui, app: &mut App) {
    let prefs = &mut app.prefs;

    let mut changed = false;

    changed |= resettable_opacity_dragvalue!(ui, prefs.opacity.base, "Base").changed();
    changed |= resettable_opacity_dragvalue!(ui, prefs.opacity.ungripped, "Ungripped").changed();
    changed |= resettable_opacity_dragvalue!(ui, prefs.opacity.hidden, "Hidden").changed();
    changed |= resettable_opacity_dragvalue!(ui, prefs.opacity.selected, "Selected").changed();

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }
}
