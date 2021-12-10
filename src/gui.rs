use glium::glutin::event_loop::ControlFlow;
use imgui::*;
use rfd::{FileDialog, MessageButtons, MessageDialog};
use std::fmt;
use std::path::Path;

use crate::config::Msaa;
use crate::puzzle::{PuzzleEnum, PuzzleType};

fn file_dialog() -> FileDialog {
    FileDialog::new()
        .add_filter("Magic Cube 4D Log Files", &["log"])
        .add_filter("All files", &["*"])
}
fn error_dialog(title: &str, e: impl fmt::Display) {
    MessageDialog::new()
        .set_title(title)
        .set_description(&e.to_string())
        .show();
}

fn try_save(puzzle: &mut PuzzleEnum, path: &Path) {
    match puzzle {
        PuzzleEnum::Rubiks4D(p) => match p.save_file(&path) {
            Ok(()) => (),
            Err(e) => error_dialog("Unable to save log file", e),
        },
        _ => error_dialog(
            "Unable to save log file",
            "Only 3x3x3x3 puzzle supports log files.",
        ),
    }
}

pub fn confirm_discard_changes(puzzle_needs_save: bool, action: &str) -> bool {
    !puzzle_needs_save
        || MessageDialog::new()
            .set_title("Unsaved changes")
            .set_description(&format!("Discard changes and {}?", action))
            .set_buttons(MessageButtons::YesNo)
            .show()
}

/// Builds the GUI.
pub fn build(ui: &imgui::Ui<'_>, puzzle: &mut PuzzleEnum, control_flow: &mut ControlFlow) {
    let mut config = crate::get_config();
    let config = &mut *config;

    // Build the menu bar.
    ui.main_menu_bar(|| {
        ui.menu("File", || {
            let can_save = puzzle.puzzle_type() == PuzzleType::Rubiks4D;

            if MenuItem::new("Open").build(ui) {
                if let Some(path) = file_dialog().pick_file() {
                    match crate::puzzle::PuzzleController::load_file(&path) {
                        Ok(p) => *puzzle = PuzzleEnum::Rubiks4D(p),
                        Err(e) => error_dialog("Unable to open log file", e),
                    }
                }
            }
            ui.separator();
            if MenuItem::new("Save").enabled(can_save).build(ui) {
                try_save(puzzle, &config.log_file);
            }
            if MenuItem::new("Save As...").enabled(can_save).build(ui) {
                if let Some(path) = file_dialog().save_file() {
                    config.needs_save = true;
                    config.log_file = path;
                    try_save(puzzle, &config.log_file);
                }
            }
            ui.separator();
            if MenuItem::new("Quit").build(ui)
                && confirm_discard_changes(puzzle.needs_save(), "quit")
            {
                *control_flow = ControlFlow::Exit;
            }
        });

        ui.menu("Edit", || {
            if MenuItem::new("Undo").enabled(puzzle.has_undo()).build(ui) {
                puzzle.undo();
            }
            if MenuItem::new("Redo").enabled(puzzle.has_redo()).build(ui) {
                puzzle.redo();
            }
        });

        ui.menu("Puzzle", || {
            for puz_type in PuzzleType::ALL {
                if MenuItem::new(&puz_type.to_string()).build(ui)
                    && confirm_discard_changes(puzzle.needs_save(), "load new puzzle")
                {
                    *puzzle = puz_type.new();
                }
            }
        });

        ui.menu("Settings", || {
            config.window_states.graphics ^= MenuItem::new("Graphics...").build(ui);
            config.window_states.view ^= MenuItem::new("View...").build(ui);
            config.window_states.colors ^= MenuItem::new("Colors...").build(ui);
            config.window_states.keybinds ^= MenuItem::new("Keybinds...").build(ui);
        })
    });

    if config.window_states.graphics {
        Window::new("Graphics")
            .opened(&mut config.window_states.graphics)
            .resizable(false)
            .always_auto_resize(true)
            .build(ui, || {
                // FPS limit
                config.needs_save ^= Slider::new("FPS limit", 5, 255)
                    .flags(SliderFlags::LOGARITHMIC)
                    .build(ui, &mut config.gfx.fps);

                // MSAA
                ComboBox::new("MSAA (requires restart)")
                    .preview_mode(ComboBoxPreviewMode::Full)
                    .preview_value(config.gfx.msaa.to_string())
                    .build(ui, || {
                        for option in [Msaa::Off, Msaa::_2, Msaa::_4, Msaa::_8] {
                            if Selectable::new(option.to_string())
                                .selected(config.gfx.msaa == option)
                                .build(ui)
                            {
                                config.needs_save = true;
                                config.gfx.msaa = option;
                            }
                        }
                    });

                ui.separator();

                // Scaling settings
                config.needs_save |=
                    ui.checkbox("Auto DPI (requires restart)", &mut config.gfx.auto_dpi);
                ui.disabled(config.gfx.auto_dpi, || {
                    config.needs_save |= Slider::new("Font scaling (reqiures restart)", 0.5, 4.0)
                        .flags(SliderFlags::LOGARITHMIC)
                        .display_format("%.1f")
                        .build(ui, &mut config.gfx.font_scaling);
                });
            });
    }

    if config.window_states.view {
        Window::new("View")
            .opened(&mut config.window_states.view)
            .resizable(false)
            .always_auto_resize(true)
            .build(ui, || {
                // View angle settings
                config.needs_save |= AngleSlider::new("Theta")
                    .range_degrees(-180.0, 180.0)
                    .build(ui, &mut config.view.theta);
                config.needs_save |= AngleSlider::new("Phi")
                    .range_degrees(-45.0, 45.0)
                    .build(ui, &mut config.view.phi);

                ui.separator();

                // Projection settings
                config.needs_save |= Slider::new("Scale", 0.1, 5.0)
                    .flags(SliderFlags::LOGARITHMIC)
                    .build(ui, &mut config.view.scale);
                config.needs_save |= AngleSlider::new("4D FOV")
                    .range_degrees(0.0, 120.0)
                    .build(ui, &mut config.view.fov_4d);
                config.needs_save |= AngleSlider::new("3D FOV")
                    .range_degrees(-120.0, 120.0)
                    .build(ui, &mut config.view.fov_3d);

                ui.separator();

                // Geometry settings
                config.needs_save |=
                    Slider::new("Face spacing", 0.0, 0.9).build(ui, &mut config.view.face_spacing);
                config.needs_save |= Slider::new("Sticker spacing", 0.0, 0.9)
                    .build(ui, &mut config.view.sticker_spacing);

                // Wireframe settings
                config.needs_save |=
                    ui.checkbox("Enable wireframe", &mut config.view.enable_wireframe);
            });
    }

    if config.window_states.colors {
        Window::new("Colors")
            .opened(&mut config.window_states.colors)
            .resizable(false)
            .always_auto_resize(true)
            .build(ui, || {
                // Sticker opacity
                config.needs_save |=
                    Slider::new("Puzzle opacity", 0.0, 1.0).build(ui, &mut config.colors.opacity);

                ui.separator();

                // Special colors
                config.needs_save |=
                    ColorEdit::new("Background", &mut config.colors.background).build(ui);
                config.needs_save =
                    ColorEdit::new("Wireframe", &mut config.colors.wireframe).build(ui);

                ui.separator();

                // Label colors
                config.needs_save |=
                    ColorEdit::new("Label fg", &mut config.colors.label_fg).build(ui);
                config.needs_save =
                    ColorEdit::new("Label bg", &mut config.colors.label_bg).build(ui);

                ui.separator();

                // Sticker colors
                let puz_type = puzzle.puzzle_type();
                let sticker_colors = config
                    .colors
                    .stickers
                    .get_mut(&puz_type)
                    .expect("missing sticker colors");
                for (face_name, color) in puz_type.face_names().iter().zip(sticker_colors) {
                    config.needs_save |= ColorEdit::new(face_name, color).build(ui);
                }
            });
    }

    if config.window_states.keybinds {
        Window::new("Keybinds")
            .opened(&mut config.window_states.keybinds)
            .resizable(false)
            .always_auto_resize(true)
            .build(ui, || {});
    }

    Window::new(&ImString::new(crate::TITLE)).build(ui, || {
        ui.text(format!("{} v{}", crate::TITLE, env!("CARGO_PKG_VERSION")));

        // Opacity
        ui.text("Opacity");
        ui.set_next_item_width(ui.window_content_region_width());

        config.save();
    });

    // Debug window.
    #[cfg(debug_assertions)]
    {
        let mut debug_info = crate::debug::FRAME_DEBUG_INFO.lock().unwrap();
        if !debug_info.is_empty() {
            Window::new(&ImString::new("Debug values"))
                .size([400.0, 300.0], Condition::FirstUseEver)
                .build(ui, || {
                    ui.text(&*debug_info);
                    *debug_info = String::new();
                });
        }
    }
}
