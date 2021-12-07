use imgui::*;

use crate::config::Msaa;
use crate::puzzle::{PuzzleEnum, PuzzleType};

/// Builds the window.
pub fn build(ui: &imgui::Ui<'_>, puzzle: &mut PuzzleEnum) {
    Window::new(&ImString::new(crate::TITLE)).build(ui, || {
        let mut config = crate::get_config();

        ui.text(format!("{} v{}", crate::TITLE, env!("CARGO_PKG_VERSION")));
        ui.text("");

        ui.text("Puzzle");
        ui.set_next_item_width(ui.window_content_region_width());
        let current_puz_type = puzzle.puzzle_type();
        ComboBox::new("##puzzle")
            .preview_mode(ComboBoxPreviewMode::Full)
            .preview_value(current_puz_type.to_string())
            .build(ui, || {
                for puz_type in [PuzzleType::Rubiks3D, PuzzleType::Rubiks4D] {
                    if Selectable::new(puz_type.to_string())
                        .selected(puz_type == current_puz_type)
                        .build(ui)
                    {
                        *puzzle = puz_type.new();
                    }
                }
            });

        ui.text("");

        if ui.button("Load from puzzle.log") {
            match crate::puzzle::PuzzleController::load_file("puzzle.log") {
                Ok(p) => *puzzle = PuzzleEnum::Rubiks4D(p),
                Err(e) => eprintln!("error: {}", e),
            }
        }
        if ui.button("Save to puzzle.log") {
            match puzzle {
                PuzzleEnum::Rubiks3D(_) => eprintln!("error: can't save 3D cube"),
                PuzzleEnum::Rubiks4D(cube) => {
                    if let Err(e) = cube.save_file("puzzle.log") {
                        eprintln!("error: {}", e);
                    }
                }
            }
        }

        ui.text("");

        // FPS limit
        ui.text("FPS limit");
        ui.set_next_item_width(ui.window_content_region_width());
        Slider::new("##fps_slider", 5, 255)
            .flags(SliderFlags::LOGARITHMIC)
            .build(ui, &mut config.gfx.fps);

        ui.text("");

        // MSAA
        ui.text("MSAA (requires restart)");
        ui.set_next_item_width(ui.window_content_region_width());
        ComboBox::new("##msaa")
            .preview_mode(ComboBoxPreviewMode::Full)
            .preview_value(config.gfx.msaa.to_string())
            .build(ui, || {
                for option in [Msaa::Off, Msaa::_2, Msaa::_4, Msaa::_8] {
                    if Selectable::new(option.to_string())
                        .selected(config.gfx.msaa == option)
                        .build(ui)
                    {
                        config.gfx.msaa = option;
                    }
                }
            });

        ui.text("");

        // Theta
        ui.text("Theta");
        ui.set_next_item_width(ui.window_content_region_width());
        AngleSlider::new("##theta_slider")
            .range_degrees(-180.0, 180.0)
            .build(ui, &mut config.gfx.theta);

        // Phi
        ui.text("Phi");
        ui.set_next_item_width(ui.window_content_region_width());
        AngleSlider::new("##phi_slider")
            .range_degrees(-180.0, 180.0)
            .build(ui, &mut config.gfx.phi);

        ui.text("");

        // 4D FOV
        ui.text("4D FOV");
        ui.set_next_item_width(ui.window_content_region_width());
        AngleSlider::new("##4d_fov_slider")
            .range_degrees(0.0, 120.0)
            .build(ui, &mut config.gfx.fov_4d);

        // 3D FOV
        ui.text("3D FOV");
        ui.set_next_item_width(ui.window_content_region_width());
        AngleSlider::new("##3d_fov_slider")
            .range_degrees(-120.0, 120.0)
            .build(ui, &mut config.gfx.fov_3d);

        ui.text("");

        // Scale
        ui.text("Scale");
        ui.set_next_item_width(ui.window_content_region_width());
        Slider::new("##scale_slider", 0.1, 5.0)
            .flags(SliderFlags::LOGARITHMIC)
            .build(ui, &mut config.gfx.scale);

        // Face spacing
        ui.text("Face spacing");
        ui.set_next_item_width(ui.window_content_region_width());
        Slider::new("##face_spacing_slider", 0.0, 0.9).build(ui, &mut config.gfx.face_spacing);

        // Sticker spacing
        ui.text("Sticker spacing");
        ui.set_next_item_width(ui.window_content_region_width());
        Slider::new("##sticker_spacing_slider", 0.0, 0.9)
            .build(ui, &mut config.gfx.sticker_spacing);

        ui.text("");

        // Opacity
        ui.text("Opacity");
        ui.set_next_item_width(ui.window_content_region_width());
        Slider::new("##opacity_slider", 0.0, 1.0).build(ui, &mut config.gfx.opacity);
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
