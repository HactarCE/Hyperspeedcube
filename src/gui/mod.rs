use glium::glutin::event_loop::ControlFlow;
use imgui::*;
use itertools::Itertools;
use std::path::Path;
use std::sync::Mutex;

mod popups;
mod util;

use crate::config::{Keybind, Msaa};
use crate::puzzle::{traits::*, Command, LayerMask, PieceTypeId, Puzzle, PuzzleType};
pub use popups::keybind_popup_handle_event;

pub struct AppState<'a> {
    pub ui: &'a Ui<'a>,
    pub mouse_pos: [f32; 2],
    pub puzzle: &'a mut Puzzle,
    pub control_flow: &'a mut ControlFlow,
}

fn try_save(puzzle: &mut Puzzle, path: &Path) {
    match puzzle {
        Puzzle::Rubiks4D(p) => match p.save_file(&path) {
            Ok(()) => (),
            Err(e) => popups::error_dialog("Unable to save log file", e),
        },
        _ => popups::error_dialog(
            "Unable to save log file",
            "Only 3x3x3x3 puzzle supports log files.",
        ),
    }
}

pub fn confirm_discard_changes(is_unsaved: bool, action: &str) -> bool {
    !is_unsaved || popups::confirm_discard_changes_dialog(action).show()
}

/// Builds the GUI.
pub fn build(app: &mut AppState) {
    let mut config_guard = crate::get_config();
    let config = &mut *config_guard;
    let ui = app.ui;

    // Build the menu bar.
    ui.main_menu_bar(|| {
        ui.menu("File", || {
            let can_save = app.puzzle.ty() == PuzzleType::Rubiks4D;

            if MenuItem::new("Open").build(ui) {
                if let Some(path) = popups::file_dialog().pick_file() {
                    match crate::puzzle::PuzzleController::load_file(&path) {
                        Ok(p) => *app.puzzle = Puzzle::Rubiks4D(p),
                        Err(e) => popups::error_dialog("Unable to open log file", e),
                    }
                }
            }
            ui.separator();
            if MenuItem::new("Save").enabled(can_save).build(ui) {
                try_save(app.puzzle, &config.log_file);
            }
            if MenuItem::new("Save As...").enabled(can_save).build(ui) {
                if let Some(path) = popups::file_dialog().save_file() {
                    config.needs_save = true;
                    config.log_file = path;
                    try_save(app.puzzle, &config.log_file);
                }
            }
            ui.separator();
            if MenuItem::new("Quit").build(ui)
                && confirm_discard_changes(app.puzzle.is_unsaved(), "quit")
            {
                *app.control_flow = ControlFlow::Exit;
            }
        });

        ui.menu("Edit", || {
            if MenuItem::new("Undo")
                .enabled(app.puzzle.has_undo())
                .build(ui)
            {
                app.puzzle.undo();
            }
            if MenuItem::new("Redo")
                .enabled(app.puzzle.has_redo())
                .build(ui)
            {
                app.puzzle.redo();
            }
        });

        ui.menu("Puzzle", || {
            for &puz_type in PuzzleType::ALL {
                if MenuItem::new(puz_type.name()).build(ui)
                    && confirm_discard_changes(app.puzzle.is_unsaved(), "load new puzzle")
                {
                    *app.puzzle = Puzzle::new(puz_type);
                }
            }
        });

        ui.menu("Settings", || {
            // TODO keep menu open, which requires internal API:
            // - PushItemFlag() / PopItemFlag()
            // - ImGuiMenuFlags_MenuItemDontCloseMenu

            let checkbox_menu_item = |name: &str, window_bool: &mut bool| {
                *window_bool ^= MenuItem::new(name).selected(*window_bool).build(ui)
            };

            checkbox_menu_item("Graphics", &mut config.window_states.graphics);
            checkbox_menu_item("View", &mut config.window_states.view);
            checkbox_menu_item("Colors", &mut config.window_states.colors);
            checkbox_menu_item("Keybinds", &mut config.window_states.keybinds);

            #[cfg(debug_assertions)]
            {
                ui.separator();
                checkbox_menu_item("Imgui Demo", &mut config.window_states.demo);
            }
        });

        ui.menu("Help", || {
            config.window_states.about ^= MenuItem::new("About").build(ui);
        });
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

                // Font size
                config.needs_save |= Slider::new("Font size", 6.0, 36.0)
                    .flags(SliderFlags::LOGARITHMIC)
                    .display_format("%.0f")
                    .build(ui, &mut config.gfx.font_size);
                config.gfx.lock_font_size = ui.is_item_active();
            });
    }

    if config.window_states.view {
        Window::new("View")
            .opened(&mut config.window_states.view)
            .resizable(false)
            .always_auto_resize(true)
            .build(ui, || {
                let view_config = &mut config.view[app.puzzle.ty()];

                // View angle settings
                config.needs_save |= AngleSlider::new("Theta")
                    .range_degrees(-180.0, 180.0)
                    .build(ui, &mut view_config.theta);
                config.needs_save |= AngleSlider::new("Phi")
                    .range_degrees(-45.0, 45.0)
                    .build(ui, &mut view_config.phi);

                ui.separator();

                // Projection settings
                config.needs_save |= Slider::new("Scale", 0.1, 5.0)
                    .flags(SliderFlags::LOGARITHMIC)
                    .build(ui, &mut view_config.scale);
                config.needs_save |= AngleSlider::new("4D FOV")
                    .range_degrees(0.0, 120.0)
                    .build(ui, &mut view_config.fov_4d);
                config.needs_save |= AngleSlider::new("3D FOV")
                    .range_degrees(-120.0, 120.0)
                    .build(ui, &mut view_config.fov_3d);

                ui.separator();

                // Geometry settings
                config.needs_save |=
                    Slider::new("Face spacing", 0.0, 0.9).build(ui, &mut view_config.face_spacing);
                config.needs_save |= Slider::new("Sticker spacing", 0.0, 0.9)
                    .build(ui, &mut view_config.sticker_spacing);

                // Outline settings
                config.needs_save |= ui.checkbox("Enable outline", &mut view_config.enable_outline);
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
                config.needs_save = ColorEdit::new("Outline", &mut config.colors.outline).build(ui);

                ui.separator();

                // Label colors
                config.needs_save |=
                    ColorEdit::new("Label fg", &mut config.colors.label_fg).build(ui);
                config.needs_save =
                    ColorEdit::new("Label bg", &mut config.colors.label_bg).build(ui);

                ui.separator();

                // Sticker colors
                let puzzle_type = app.puzzle.ty();
                let sticker_colors = &mut config.colors.stickers[puzzle_type].0;
                for (face_name, color) in puzzle_type.face_names().iter().zip(sticker_colors) {
                    config.needs_save |= ColorEdit::new(face_name, color).build(ui);
                }
            });
    }

    if config.window_states.keybinds {
        const MIN_WIDTH: f32 = 200.0; // TODO use a better value
        const MIN_HEIGHT: f32 = 100.0;

        lazy_static! {
            static ref KEYBINDS_WINDOW_MIN_WIDTH: Mutex<f32> = Mutex::new(MIN_WIDTH);
        }

        let mut min_window_width = KEYBINDS_WINDOW_MIN_WIDTH.lock().unwrap();
        Window::new("Keybinds")
            .opened(&mut config.window_states.keybinds)
            .size_constraints([*min_window_width, 200.0], [f32::MAX, f32::MAX])
            .build(ui, || {
                let current_window_width = ui.window_size()[0];
                let mut extra_width = current_window_width - MIN_WIDTH;
                if ui.button("Add keybind") {
                    config.keybinds[app.puzzle.ty()].push(Keybind::default());
                    config.needs_save = true;
                }
                build_keybind_table(
                    app,
                    &mut config.keybinds[app.puzzle.ty()],
                    &mut config.needs_save,
                    &mut extra_width,
                );
                *min_window_width = current_window_width - extra_width;
            });
    }

    if config.window_states.about {
        Window::new("About")
            .opened(&mut config.window_states.about)
            .resizable(false)
            .always_auto_resize(true)
            .build(ui, || {
                ui.text(format!("{} v{}", crate::TITLE, env!("CARGO_PKG_VERSION")));
                ui.text(format!("{}", env!("CARGO_PKG_DESCRIPTION")));
                ui.text("");
                ui.text(format!("License: {}", env!("CARGO_PKG_LICENSE")));
                ui.text(format!(
                    "Created by {}",
                    env!("CARGO_PKG_AUTHORS").split(':').join(", "),
                ));
            });
    }

    #[cfg(debug_assertions)]
    if config.window_states.demo {
        ui.show_demo_window(&mut config.window_states.demo);
    }

    // Bulid the keybind popup.
    drop(config_guard);
    popups::build_keybind_popup(app);
    let mut config_guard = crate::get_config();
    let config = &mut config_guard;

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

    // Save any configuration changes.
    config.save();
}

fn build_keybind_table(
    app: &mut AppState,
    keybinds: &mut Vec<Keybind>,
    needs_save: &mut bool,
    extra_width: &mut f32,
) {
    let ui = app.ui;
    let puzzle_type = app.puzzle.ty();

    let flags = TableFlags::BORDERS | TableFlags::SIZING_FIXED_FIT | TableFlags::SCROLL_Y;
    let table_token = match ui.begin_table_with_flags("keybinds", 3, flags) {
        Some(tok) => tok,
        None => return,
    };

    ui.table_setup_column("##reorder_column");
    ui.table_setup_column("Keybind##column");
    ui.table_setup_column_with(TableColumnSetup {
        name: "Command##column",
        flags: TableColumnFlags::WIDTH_STRETCH,
        ..Default::default()
    });

    ui.table_setup_scroll_freeze(0, 1);
    ui.table_headers_row();

    lazy_static! {
        static ref DRAG: Mutex<Option<(usize, usize)>> = Mutex::new(None);
    }
    let mut drag = DRAG.lock().unwrap();
    if !ui.is_mouse_dragging(MouseButton::Left) {
        *drag = None;
    }

    // Table contents
    let mut drag_to = None;
    let mut delete_idx = None;
    let w = ui.calc_text_size("Ctrl + Shift + Alt")[0] * 3.0;

    for (i, keybind) in keybinds.iter_mut().enumerate() {
        ui.table_next_row();

        ui.table_next_column();
        let label_prefix = " = ##reorder";
        let reorder_selectable_label = match *drag {
            Some((start, end)) if i == end => format!("{}{}", label_prefix, start),
            Some((start, _)) if i == start => format!("{}_tmp", label_prefix),
            _ => format!("{}{}", label_prefix, i),
        };
        Selectable::new(&reorder_selectable_label)
            .size([0.0, ui.frame_height()])
            .build(ui);
        ui.align_text_to_frame_padding(); // after Selectable so that Selectable uses the full height
        if ui.is_item_hovered() {
            ui.set_mouse_cursor(Some(MouseCursor::ResizeAll));
            // ui.tooltip_text("Drag to reorder"); // TODO maybe show after tooltip delay
        }
        if ui.is_item_active() {
            ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
            if drag.is_none() {
                *drag = Some((i, i));
            }
        }
        if let Some((_, drag_end)) = *drag {
            let mouse_y = app.mouse_pos[1];
            if (drag_end == i + 1 && mouse_y < ui.item_rect_max()[1])
                || (drag_end + 1 == i && mouse_y > ui.item_rect_min()[1])
            {
                drag_to = Some(i);
            }
        }

        ui.table_next_column();
        if ui.button(&format!("X##delete_keybind{}", i)) {
            delete_idx = Some(i);
        }
        ui.same_line();
        if ui.button_with_size(format!("{}##change_keybind{}", keybind, i), [w, 0.0]) {
            popups::open_keybind_popup(keybind.clone(), move |new_keybind| {
                let mut config = crate::get_config();
                config.keybinds[puzzle_type][i] = new_keybind;
                config.needs_save = true;
            });
        }

        ui.table_next_column();
        build_command_select_ui(ui, puzzle_type, i, &mut keybind.command, needs_save);

        ui.same_line();
        let extra_width_in_col = ui.content_region_avail()[0];
        if *extra_width > extra_width_in_col {
            *extra_width = extra_width_in_col
        }
    }

    if let Some(((_start, ref mut from), to)) = drag.as_mut().zip(drag_to) {
        keybinds.swap(*from, to);
        *from = to;
        *needs_save = true;
    }
    if let Some(i) = delete_idx {
        keybinds.remove(i);
        *needs_save = true;
    }

    drop(table_token);
}

fn build_command_select_ui(
    ui: &Ui<'_>,
    puzzle_type: PuzzleType,
    i: usize,
    command: &mut Command,
    needs_save: &mut bool,
) {
    use Command as Cmd;

    let mut command_idx = match command {
        Cmd::None => 0,

        Cmd::Twist { .. } => 1,
        Cmd::Recenter { .. } => 2,

        Cmd::HoldSelectFace(_) | Cmd::HoldSelectLayers(_) | Cmd::HoldSelectPieceType(_) => 3,
        Cmd::ToggleSelectFace(_) | Cmd::ToggleSelectLayers(_) | Cmd::ToggleSelectPieceType(_) => 4,
        Cmd::ClearToggleSelectFaces
        | Cmd::ClearToggleSelectLayers
        | Cmd::ClearToggleSelectPieceType => 5,
    };
    let old_command_idx = command_idx;

    let default_direction = puzzle_type.twist_direction_names()[0].to_owned();
    let default_face = puzzle_type.face_names()[0].to_owned();

    if build_autosize_combo(
        ui,
        &format!("##command{}", i),
        &mut command_idx,
        &[
            "None",
            "Twist",
            "Recenter",
            "Select",
            "Toggle select",
            "Clear selected",
        ],
    ) && command_idx != old_command_idx
    {
        *needs_save = true;
        match command_idx {
            0 => *command = Cmd::None,

            1 => {
                *command = Cmd::Twist {
                    face: None,
                    layers: LayerMask(1),
                    direction: default_direction,
                }
            }
            2 => *command = Cmd::Recenter { face: None },

            3 => {
                *command = match command {
                    Cmd::ToggleSelectFace(f) => Cmd::HoldSelectFace(f.clone()),
                    Cmd::ToggleSelectLayers(l) => Cmd::HoldSelectLayers(*l),
                    Cmd::ToggleSelectPieceType(p) => Cmd::HoldSelectPieceType(*p),
                    Cmd::ClearToggleSelectFaces => Cmd::HoldSelectFace(default_face.clone()),
                    Cmd::ClearToggleSelectLayers => Cmd::HoldSelectLayers(LayerMask(1)),
                    Cmd::ClearToggleSelectPieceType => Cmd::HoldSelectPieceType(PieceTypeId(0)),
                    _ => Cmd::HoldSelectFace(default_face.clone()),
                }
            }
            4 => {
                *command = match command {
                    Cmd::HoldSelectFace(f) => Cmd::ToggleSelectFace(f.clone()),
                    Cmd::HoldSelectLayers(l) => Cmd::ToggleSelectLayers(*l),
                    Cmd::HoldSelectPieceType(p) => Cmd::ToggleSelectPieceType(*p),
                    Cmd::ClearToggleSelectFaces => Cmd::ToggleSelectFace(default_face.clone()),
                    Cmd::ClearToggleSelectLayers => Cmd::ToggleSelectLayers(LayerMask(1)),
                    Cmd::ClearToggleSelectPieceType => Cmd::ToggleSelectPieceType(PieceTypeId(0)),
                    _ => Cmd::ToggleSelectFace(default_face.clone()),
                }
            }
            5 => {
                *command = match command {
                    Cmd::HoldSelectFace(_) | Cmd::ToggleSelectFace(_) => {
                        Cmd::ClearToggleSelectFaces
                    }
                    Cmd::HoldSelectLayers(_) | Cmd::ToggleSelectLayers(_) => {
                        Cmd::ClearToggleSelectLayers
                    }
                    Cmd::HoldSelectPieceType(_) | Cmd::ToggleSelectPieceType(_) => {
                        Cmd::ClearToggleSelectPieceType
                    }
                    _ => Cmd::ClearToggleSelectFaces,
                }
            }
            _ => *command = Cmd::None, // should be unreachable
        }
    }

    let is_hold_select_command = matches!(
        command,
        Cmd::HoldSelectFace(_) | Cmd::HoldSelectLayers(_) | Cmd::HoldSelectPieceType(_)
    );
    let is_toggle_select_command = matches!(
        command,
        Cmd::ToggleSelectFace(_) | Cmd::ToggleSelectLayers(_) | Cmd::ToggleSelectPieceType(_)
    );
    let is_clear_toggle_select_command = matches!(
        command,
        Cmd::ClearToggleSelectFaces
            | Cmd::ClearToggleSelectLayers
            | Cmd::ClearToggleSelectPieceType
    );
    if is_hold_select_command || is_toggle_select_command || is_clear_toggle_select_command {
        let mut current_item = match command {
            Cmd::HoldSelectFace(_) | Cmd::ToggleSelectFace(_) | Cmd::ClearToggleSelectFaces => 0,
            Cmd::HoldSelectLayers(_)
            | Cmd::ToggleSelectLayers(_)
            | Cmd::ClearToggleSelectLayers => 1,
            Cmd::HoldSelectPieceType(_)
            | Cmd::ToggleSelectPieceType(_)
            | Cmd::ClearToggleSelectPieceType => 2,
            _ => unreachable!(),
        };
        ui.same_line();
        if build_autosize_combo(
            ui,
            &format!("##select_what{}", i),
            &mut current_item,
            &["Face", "Layers", "Piece type"],
        ) {
            *needs_save = true;
            if is_hold_select_command {
                match current_item {
                    0 => *command = Cmd::HoldSelectFace(default_face),
                    1 => *command = Cmd::HoldSelectLayers(LayerMask(1)),
                    2 => *command = Cmd::HoldSelectPieceType(PieceTypeId(0)),
                    _ => (), // should be unreachable
                }
            } else if is_toggle_select_command {
                match current_item {
                    0 => *command = Cmd::ToggleSelectFace(default_face),
                    1 => *command = Cmd::ToggleSelectLayers(LayerMask(1)),
                    2 => *command = Cmd::ToggleSelectPieceType(PieceTypeId(0)),
                    _ => (), // should be unreachable
                }
            } else if is_clear_toggle_select_command {
                match current_item {
                    0 => *command = Cmd::ClearToggleSelectFaces,
                    1 => *command = Cmd::ClearToggleSelectLayers,
                    2 => *command = Cmd::ClearToggleSelectPieceType,
                    _ => (), // should be unreachable
                }
            }
        }
    }

    fn combo_label(ui: &Ui<'_>, s: &str) {
        ui.same_line();
        ui.text(&format!("{}:", s));
        ui.same_line();
    }

    match command {
        Cmd::None => (),

        Cmd::Twist {
            face,
            layers,
            direction,
        } => {
            combo_label(ui, "Face");
            *needs_save |= build_optional_string_select_combo(
                ui,
                &format!("##face{}", i),
                face,
                puzzle_type.face_names(),
            );

            combo_label(ui, "Layers");
            *needs_save |= build_layer_mask_select_checkboxes(
                ui,
                &format!("##layers{}", i),
                layers,
                puzzle_type.layer_count(),
            );

            combo_label(ui, "Direction");
            *needs_save |= build_string_select_combo(
                ui,
                &format!("##direction{}", i),
                direction,
                puzzle_type.twist_direction_names(),
            );
        }
        Cmd::Recenter { face } => {
            combo_label(ui, "Face");
            *needs_save |= build_optional_string_select_combo(
                ui,
                &format!("##face{}", i),
                face,
                puzzle_type.face_names(),
            );
        }

        Cmd::HoldSelectFace(face) | Cmd::ToggleSelectFace(face) => {
            ui.same_line();
            *needs_save |= build_string_select_combo(
                ui,
                &format!("##face{}", i),
                face,
                puzzle_type.face_names(),
            );
        }
        Cmd::HoldSelectLayers(layers) | Cmd::ToggleSelectLayers(layers) => {
            ui.same_line();
            *needs_save |= build_layer_mask_select_checkboxes(
                ui,
                &format!("##layers{}", i),
                layers,
                puzzle_type.layer_count(),
            );
        }
        Cmd::HoldSelectPieceType(piece_type) | Cmd::ToggleSelectPieceType(piece_type) => {
            let mut piece_type_id = piece_type.0 as usize;
            ui.same_line();
            *needs_save |= build_autosize_combo(
                ui,
                &format!("##piece_type{}", i),
                &mut piece_type_id,
                puzzle_type.piece_type_names(),
            );
            piece_type.0 = piece_type_id as u32;
        }
        Cmd::ClearToggleSelectFaces
        | Cmd::ClearToggleSelectLayers
        | Cmd::ClearToggleSelectPieceType => (),
    }
}

#[must_use]
fn build_optional_string_select_combo<'a>(
    ui: &Ui<'_>,
    label: &str,
    current_item: &mut Option<String>,
    items: impl Into<Vec<&'a str>>,
) -> bool {
    let mut items = items.into();
    items.insert(0, "(selected)");

    // Find the index of the currently selected item.
    let mut i = current_item
        .as_ref()
        .and_then(|item_name| items.iter().position(|&x| x == item_name))
        .unwrap_or(0);

    if build_autosize_combo(ui, label, &mut i, &items) {
        *current_item = if i == 0 {
            None
        } else {
            Some(items[i].to_owned())
        };
        true
    } else {
        false
    }
}

#[must_use]
fn build_layer_mask_select_checkboxes(
    ui: &Ui<'_>,
    label: &str,
    layers: &mut LayerMask,
    layer_count: usize,
) -> bool {
    let mut needs_save = false;

    let checkbox_padding = ui.clone_style().frame_padding[0];
    for l in 0..layer_count {
        needs_save |= ui.checkbox_flags(&format!("{}##{}", label, l), &mut layers.0, 1 << l);
        ui.same_line_with_spacing(0.0, checkbox_padding);
        if ui.is_item_hovered() {
            ui.tooltip_text(format!("Layer {}", l + 1));
        }
    }

    needs_save
}

#[must_use]
fn build_string_select_combo(
    ui: &Ui<'_>,
    label: &str,
    selected: &mut String,
    items: &[&str],
) -> bool {
    // Find the index of the currently selected item.
    let mut i = items.iter().position(|&x| x == selected).unwrap_or(0);

    if build_autosize_combo(ui, label, &mut i, &items) {
        *selected = items[i].to_owned();
        true
    } else {
        false
    }
}

#[must_use]
fn build_autosize_combo(
    ui: &Ui<'_>,
    label: &str,
    current_item: &mut usize,
    items: &[&str],
) -> bool {
    let w = items
        .iter()
        .map(|s| ui.calc_text_size(s)[0])
        .fold(0.0, |a, b| if b > a { b } else { a })
        + ui.text_line_height_with_spacing()
        + ui.clone_style().frame_padding[0] * 3.0;
    ui.set_next_item_width(w);
    ui.combo(label, current_item, items, |&s| s.into())
}
