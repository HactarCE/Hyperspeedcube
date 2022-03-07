use std::ops::RangeInclusive;

use crate::app::App;
use crate::preferences::Msaa;
use crate::puzzle::{PuzzleControllerTrait, PuzzleTypeTrait};

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    ui.heading("Preferences"); // TODO: disable wrap?
    ui.separator();
    egui::ScrollArea::new([false, true]).show(ui, |ui| {
        ui.collapsing("Colors", |ui| build_colors_section(ui, app));
        ui.collapsing("Graphics", |ui| build_graphics_section(ui, app));
        ui.collapsing("View", |ui| build_view_section(ui, app));
    });
}

fn build_colors_section(ui: &mut egui::Ui, app: &mut App) {
    let puzzle_type = app.puzzle.ty();
    let default_prefs = &*crate::preferences::DEFAULT_PREFS;

    let mut changed = false;

    // Puzzle opacity
    let r = ui.add(PercentDragValueWithReset {
        label: "Puzzle opacity",
        value: &mut app.prefs.colors.opacity,
        reset_value: default_prefs.colors.opacity,
    });
    changed |= r.changed();

    ui.separator();

    // Special colors
    let r = ui.add(WidgetWithReset {
        label: "Background",
        value: &mut app.prefs.colors.background,
        reset_value: default_prefs.colors.background,
        reset_value_str: &format!("{:?}", default_prefs.colors.background),
        make_widget: |value| |ui: &mut egui::Ui| ui.color_edit_button_rgb(value),
    });
    changed |= r.changed();
    let r = ui.add(WidgetWithReset {
        label: "Outline",
        value: &mut app.prefs.colors.outline,
        reset_value: default_prefs.colors.outline,
        reset_value_str: &format!("{:?}", default_prefs.colors.outline),
        make_widget: |value| |ui: &mut egui::Ui| ui.color_edit_button_rgb(value),
    });
    changed |= r.changed();

    ui.separator();

    // Sticker colors
    let sticker_colors = &mut app.prefs.colors.faces[puzzle_type].0;
    let default_sticker_colors = &default_prefs.colors.faces[puzzle_type].0;
    let face_names = puzzle_type.face_names().iter();
    for ((face_name, color), default_color) in
        face_names.zip(sticker_colors).zip(default_sticker_colors)
    {
        let r = ui.add(WidgetWithReset {
            label: face_name,
            value: color,
            reset_value: *default_color,
            reset_value_str: &format!("{:?}", default_color),
            make_widget: |value| |ui: &mut egui::Ui| ui.color_edit_button_rgb(value),
        });
        changed |= r.changed();
    }

    app.prefs.needs_save |= changed;
    app.wants_repaint |= changed;
}
fn build_graphics_section(ui: &mut egui::Ui, app: &mut App) {
    let default_prefs = &*crate::preferences::DEFAULT_PREFS;

    // FPS limit
    let r = ui.add(WidgetWithReset {
        label: "FPS limit",
        value: &mut app.prefs.gfx.fps,
        reset_value: default_prefs.gfx.fps,
        reset_value_str: &default_prefs.gfx.fps.to_string(),
        make_widget: |value| {
            egui::DragValue::new(value)
                .clamp_range(5..=255_u32)
                .speed(0.5)
        },
    });
    app.prefs.needs_save |= r.changed();

    // MSAA
    let r = ui.add(WidgetWithReset {
        label: "MSAA",
        value: &mut app.prefs.gfx.msaa,
        reset_value: default_prefs.gfx.msaa,
        reset_value_str: default_prefs.gfx.msaa.as_ref(),
        make_widget: |value| {
            move |ui: &mut egui::Ui| {
                let mut changed = false;
                let mut r = egui::ComboBox::from_id_source("msaa")
                    .selected_text(value.as_ref())
                    .show_ui(ui, |ui| {
                        changed |= [Msaa::Off, Msaa::_2, Msaa::_4, Msaa::_8]
                            .iter()
                            .map(|&option| ui.selectable_value(value, option, option.as_ref()))
                            .any(|r| r.changed());
                    })
                    .response;
                if changed {
                    r.mark_changed();
                }
                r
            }
        },
    });
    app.prefs.needs_save |= r.changed();
    app.wants_repaint |= r.changed();
}
fn build_view_section(ui: &mut egui::Ui, app: &mut App) {
    let view_prefs = &mut app.prefs.view[app.puzzle.ty()];
    let default_view_prefs = &crate::preferences::DEFAULT_PREFS.view[app.puzzle.ty()];

    let mut changed = false;

    ui.label("View angle:");
    // Pitch
    let r = ui.add(DegreesDragValueWithReset {
        label: "Pitch",
        value: &mut view_prefs.theta,
        reset_value: default_view_prefs.theta,
        clamp_range: -180.0..=180.0,
        speed: 1.0,
    });
    changed |= r.changed();
    // Yaw
    let r = ui.add(DegreesDragValueWithReset {
        label: "Yaw",
        value: &mut view_prefs.phi,
        reset_value: default_view_prefs.phi,
        clamp_range: -45.0..=45.0,
        speed: 1.0,
    });
    changed |= r.changed();

    ui.separator();
    ui.label("Projection:");
    // Scale
    let speed = view_prefs.scale / 100.0;
    let r = ui.add(WidgetWithReset {
        label: "Scale",
        value: &mut view_prefs.scale,
        reset_value: default_view_prefs.scale,
        reset_value_str: &default_view_prefs.scale.to_string(),
        make_widget: |value| {
            egui::DragValue::new(value)
                .fixed_decimals(2)
                .clamp_range(0.1..=5.0_f32)
                .speed(speed)
        },
    });
    changed |= r.changed();
    // 4D FOV
    let r = ui.add(DegreesDragValueWithReset {
        label: "4D FOV",
        value: &mut view_prefs.fov_4d,
        reset_value: default_view_prefs.fov_4d,
        clamp_range: 0.0..=120.0,
        speed: 0.5,
    });
    changed |= r.changed();
    // 3D FOV
    let r = ui.add(DegreesDragValueWithReset {
        label: "3D FOV",
        value: &mut view_prefs.fov_3d,
        reset_value: default_view_prefs.fov_3d,
        clamp_range: -120.0..=120.0,
        speed: 0.5,
    });
    changed |= r.changed();

    ui.separator();
    ui.label("Geometry:");
    // Face spacing
    let r = ui.add(WidgetWithReset {
        label: "Face spacing",
        value: &mut view_prefs.face_spacing,
        reset_value: default_view_prefs.face_spacing,
        reset_value_str: &default_view_prefs.face_spacing.to_string(),
        make_widget: |value| {
            egui::DragValue::new(value)
                .fixed_decimals(2)
                .clamp_range(0.0..=0.9_f32)
                .speed(0.005)
        },
    });
    changed |= r.changed();
    // Sticker spacing
    let r = ui.add(WidgetWithReset {
        label: "Sticker spacing",
        value: &mut view_prefs.sticker_spacing,
        reset_value: default_view_prefs.sticker_spacing,
        reset_value_str: &default_view_prefs.sticker_spacing.to_string(),
        make_widget: |value| {
            egui::DragValue::new(value)
                .fixed_decimals(2)
                .clamp_range(0.0..=0.9_f32)
                .speed(0.005)
        },
    });
    changed |= r.changed();
    // Enable outline
    let r = ui.add(CheckboxWithReset {
        label: "Enable outline",
        value: &mut view_prefs.enable_outline,
        reset_value: default_view_prefs.enable_outline,
    });
    changed |= r.changed();

    app.prefs.needs_save |= changed;
    app.wants_repaint |= changed;
}

struct PercentDragValueWithReset<'a> {
    label: &'a str,
    value: &'a mut f32,
    reset_value: f32,
}
impl egui::Widget for PercentDragValueWithReset<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(WidgetWithReset {
            label: self.label,
            value: self.value,
            reset_value: self.reset_value,
            reset_value_str: &format!("{}%", self.reset_value * 100.0),
            make_widget: |value| {
                egui::DragValue::from_get_set(|new_value| {
                    if let Some(x) = new_value {
                        *value = x as f32 / 100.0;
                    }
                    *value as f64 * 100.0
                })
                .suffix("%")
                .fixed_decimals(0)
                .clamp_range(0.0..=100.0_f32)
                .speed(0.5)
            },
        })
    }
}

struct DegreesDragValueWithReset<'a> {
    label: &'a str,
    value: &'a mut f32,
    reset_value: f32,
    clamp_range: RangeInclusive<f64>,
    speed: f64,
}
impl egui::Widget for DegreesDragValueWithReset<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(WidgetWithReset {
            label: self.label,
            value: self.value,
            reset_value: self.reset_value,
            reset_value_str: &format!("{}°", self.reset_value),
            make_widget: |value| {
                egui::DragValue::new(value)
                    .suffix("°")
                    .fixed_decimals(0)
                    .clamp_range(self.clamp_range)
                    .speed(self.speed)
            },
        })
    }
}

#[must_use]
struct WidgetWithReset<'a, V, W: 'a, F: FnOnce(&'a mut V) -> W> {
    label: &'a str,
    value: &'a mut V,
    reset_value: V,
    reset_value_str: &'a str,
    make_widget: F,
}
impl<'a, V, W, F> egui::Widget for WidgetWithReset<'a, V, W, F>
where
    V: PartialEq,
    W: 'a + egui::Widget,
    F: FnOnce(&'a mut V) -> W,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        with_reset_button(
            ui,
            self.value,
            self.reset_value,
            self.reset_value_str,
            |ui, value| {
                let r = ui
                    .allocate_ui_with_layout(
                        ui.spacing().interact_size * egui::vec2(1.5, 1.0),
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| ui.add((self.make_widget)(value)),
                    )
                    .inner;
                ui.label(self.label);
                r
            },
        )
    }
}

#[must_use]
struct CheckboxWithReset<'a> {
    label: &'a str,
    value: &'a mut bool,
    reset_value: bool,
}
impl egui::Widget for CheckboxWithReset<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        with_reset_button(ui, self.value, self.reset_value, "", |ui, value| {
            ui.checkbox(value, self.label)
        })
    }
}

fn with_reset_button<'a, T: PartialEq>(
    ui: &mut egui::Ui,
    value: &'a mut T,
    reset_value: T,
    reset_value_str: &str,
    widget: impl FnOnce(&mut egui::Ui, &'a mut T) -> egui::Response,
) -> egui::Response {
    ui.horizontal(|ui| {
        let hover_text = match reset_value_str {
            "" => "Reset".to_owned(),
            s => format!("Reset to {}", s),
        };
        let reset_resp = ui
            .add_enabled(*value != reset_value, egui::Button::new("⟲"))
            .on_hover_text(&hover_text);
        if reset_resp.clicked() {
            *value = reset_value;
        }
        let mut r = widget(ui, value);
        if reset_resp.clicked() {
            r.mark_changed();
        }
        r
    })
    .inner
}
