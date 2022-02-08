use glium::glutin::event_loop::ControlFlow;
use imgui::*;
use itertools::Itertools;
use std::path::Path;
use std::sync::Mutex;
use strum::IntoEnumIterator;

mod popups;
mod util;

use crate::commands::{Command, PuzzleCommand, SelectCategory, SelectHow, SelectThing};
use crate::preferences::{KeyCombo, Keybind, Msaa, Preferences};
use crate::puzzle::{
    traits::*, LayerMask, PieceType, Puzzle, PuzzleType, TwistDirection, TwistMetric,
};
pub use popups::keybind_popup_handle_event;

pub struct AppState<'a> {
    pub ui: &'a Ui<'a>,
    pub mouse_pos: [f32; 2],
    pub puzzle: &'a Puzzle,
    pub control_flow: &'a mut ControlFlow,
    pub command_queue: &'a mut Vec<Command>,
}

/// Builds the GUI.
pub fn build(app: &mut AppState) {
    let mut prefs_guard = crate::get_prefs();
    let prefs = &mut *prefs_guard;
    let ui = app.ui;

    // Build the menu bar.
    ui.main_menu_bar(|| {
        ui.menu("File", || {
            let can_save = app.puzzle.ty() == PuzzleType::Rubiks4D;

            if MenuItem::new("Open").build(ui) {
                app.command_queue.push(Command::Open);
            }
            ui.separator();
            if MenuItem::new("Save").enabled(can_save).build(ui) {
                app.command_queue.push(Command::Save);
            }
            if MenuItem::new("Save As...").enabled(can_save).build(ui) {
                app.command_queue.push(Command::SaveAs);
            }
            ui.separator();
            if MenuItem::new("Quit").build(ui) {
                app.command_queue.push(Command::Quit);
            }
        });

        ui.menu("Edit", || {
            if MenuItem::new("Undo")
                .enabled(app.puzzle.has_undo())
                .build(ui)
            {
                app.command_queue.push(Command::Undo);
            }
            if MenuItem::new("Redo")
                .enabled(app.puzzle.has_redo())
                .build(ui)
            {
                app.command_queue.push(Command::Redo);
            }
        });

        ui.menu("Puzzle", || {
            for &puzzle_type in PuzzleType::ALL {
                if MenuItem::new(puzzle_type.name()).build(ui) {
                    app.command_queue.push(Command::NewPuzzle(puzzle_type))
                }
            }
        });

        ui.menu("Settings", || {
            // TODO keep menu open, which requires internal API:
            // - PushItemFlag() / PopItemFlag()
            // - ImGuiMenuFlags_MenuItemDontCloseMenu

            let checkbox_menu_item = |name: &str, window_bool: &mut bool| -> bool {
                let ret = MenuItem::new(name).selected(*window_bool).build(ui);
                *window_bool ^= ret;
                ret
            };

            prefs.needs_save |= checkbox_menu_item("Graphics", &mut prefs.window_states.graphics);
            prefs.needs_save |= checkbox_menu_item("View", &mut prefs.window_states.view);
            prefs.needs_save |= checkbox_menu_item("Colors", &mut prefs.window_states.colors);
            prefs.needs_save |= checkbox_menu_item("Keybinds", &mut prefs.window_states.keybinds);

            #[cfg(debug_assertions)]
            {
                ui.separator();
                checkbox_menu_item("Imgui Demo", &mut prefs.window_states.demo);
            }
        });

        ui.menu("Help", || {
            prefs.window_states.about ^= MenuItem::new("About").build(ui);
        });
    });

    // Build the status bar.
    build_status_bar(
        ui,
        |tok| {
            // Display twist count.
            {
                let metric = prefs.info.metric;
                let twist_count = app.puzzle.twist_count(metric);
                let metric = tok.right_segment(ui, format!("{}: {}", metric, twist_count));
                if ui.is_item_clicked() || ui.is_item_clicked_with_button(MouseButton::Right) {
                    ui.open_popup("turn_metric_popup");
                }
                ui.popup("turn_metric_popup", || {
                    for metric in TwistMetric::iter() {
                        if Selectable::new(metric).build(ui) {
                            prefs.info.metric = metric;
                            prefs.needs_save = true;
                        }
                    }
                });
            }
        },
        |tok| tok.left_segment(ui, &*crate::status_msg()),
    );

    if prefs.window_states.graphics {
        Window::new("Graphics")
            .opened(&mut prefs.window_states.graphics)
            .resizable(false)
            .always_auto_resize(true)
            .build(ui, || {
                // FPS limit
                prefs.needs_save |= Slider::new("FPS limit", 5, 255)
                    .flags(SliderFlags::LOGARITHMIC)
                    .build(ui, &mut prefs.gfx.fps);

                // MSAA
                ComboBox::new("MSAA (requires restart)")
                    .preview_mode(ComboBoxPreviewMode::Full)
                    .preview_value(prefs.gfx.msaa.to_string())
                    .build(ui, || {
                        for option in [Msaa::Off, Msaa::_2, Msaa::_4, Msaa::_8] {
                            if Selectable::new(option.to_string())
                                .selected(prefs.gfx.msaa == option)
                                .build(ui)
                            {
                                prefs.gfx.msaa = option;
                                prefs.needs_save = true;
                            }
                        }
                    });

                ui.separator();

                // Font size
                prefs.needs_save |= Slider::new("Font size", 6.0, 36.0)
                    .flags(SliderFlags::LOGARITHMIC)
                    .display_format("%.0f")
                    .build(ui, &mut prefs.gfx.font_size);
                prefs.gfx.lock_font_size = ui.is_item_active();
            });
        // If the window closed, update preferences.
        prefs.needs_save |= !prefs.window_states.graphics;
    }

    if prefs.window_states.view {
        Window::new("View")
            .opened(&mut prefs.window_states.view)
            .resizable(false)
            .always_auto_resize(true)
            .build(ui, || {
                let view_prefs = &mut prefs.view[app.puzzle.ty()];

                // View angle settings
                prefs.needs_save |=
                    Slider::new("Theta", -180.0, 180.0).build(ui, &mut view_prefs.theta);
                prefs.needs_save |= Slider::new("Phi", -45.0, 45.0).build(ui, &mut view_prefs.phi);

                ui.separator();

                // Projection settings
                prefs.needs_save |= Slider::new("Scale", 0.1, 5.0)
                    .flags(SliderFlags::LOGARITHMIC)
                    .build(ui, &mut view_prefs.scale);
                prefs.needs_save |=
                    Slider::new("4D FOV", 0.0, 120.0).build(ui, &mut view_prefs.fov_4d);
                prefs.needs_save |=
                    Slider::new("3D FOV", -120.0, 120.0).build(ui, &mut view_prefs.fov_3d);

                ui.separator();

                // Geometry settings
                prefs.needs_save |=
                    Slider::new("Face spacing", 0.0, 0.9).build(ui, &mut view_prefs.face_spacing);
                prefs.needs_save |= Slider::new("Sticker spacing", 0.0, 0.9)
                    .build(ui, &mut view_prefs.sticker_spacing);

                // Outline settings
                prefs.needs_save |= ui.checkbox("Enable outline", &mut view_prefs.enable_outline);
            });
        // If the window closed, update preferences.
        prefs.needs_save |= !prefs.window_states.view;
    }

    if prefs.window_states.colors {
        Window::new("Colors")
            .opened(&mut prefs.window_states.colors)
            .resizable(false)
            .always_auto_resize(true)
            .build(ui, || {
                // Sticker opacity
                prefs.needs_save |=
                    Slider::new("Puzzle opacity", 0.0, 1.0).build(ui, &mut prefs.colors.opacity);

                ui.separator();

                // Special colors
                prefs.needs_save |=
                    ColorEdit::new("Background", &mut prefs.colors.background).build(ui);
                prefs.needs_save = ColorEdit::new("Outline", &mut prefs.colors.outline).build(ui);

                ui.separator();

                // Label colors
                prefs.needs_save |=
                    ColorEdit::new("Label fg", &mut prefs.colors.label_fg).build(ui);
                prefs.needs_save = ColorEdit::new("Label bg", &mut prefs.colors.label_bg).build(ui);

                ui.separator();

                // Sticker colors
                let puzzle_type = app.puzzle.ty();
                let sticker_colors = &mut prefs.colors.faces[puzzle_type].0;
                for (face_name, color) in puzzle_type.face_names().iter().zip(sticker_colors) {
                    prefs.needs_save |= ColorEdit::new(face_name, color).build(ui);
                }
            });
        // If the window closed, update preferences.
        prefs.needs_save |= !prefs.window_states.colors;
    }

    if prefs.window_states.keybinds {
        let keybinds_min_width: f32 = ui.frame_height() * 10.0;
        let keybinds_min_height: f32 = ui.frame_height() * 10.0;

        lazy_static! {
            static ref KEYBINDS_WINDOW_MIN_WIDTH: Mutex<f32> = Mutex::new(1.0);
        }

        let mut required_window_width = KEYBINDS_WINDOW_MIN_WIDTH.lock().unwrap();
        let mut open = prefs.window_states.keybinds;
        Window::new("Keybinds")
            .opened(&mut open)
            .size_constraints(
                [*required_window_width, keybinds_min_height],
                [f32::MAX, f32::MAX],
            )
            .build(ui, || {
                let puzzle_type = app.puzzle.ty();

                let current_window_width = ui.window_size()[0];
                let mut extra_width = current_window_width - keybinds_min_width;
                if ui.button("Add keybind") {
                    prefs.puzzle_keybinds[puzzle_type].push(Keybind::default());
                    prefs.needs_save = true;
                }
                prefs.needs_save |= build_keybind_table(
                    app,
                    &mut extra_width,
                    prefs,
                    move |prefs| &mut prefs.puzzle_keybinds[puzzle_type],
                    |i, command| build_puzzle_command_select_ui(ui, puzzle_type, i, command),
                );
                *required_window_width = current_window_width - extra_width;
            });
        prefs.window_states.keybinds = open;
        // If the window closed, update preferences.
        prefs.needs_save |= !prefs.window_states.keybinds;
    }

    if prefs.window_states.about {
        Window::new("About")
            .opened(&mut prefs.window_states.about)
            .resizable(false)
            .always_auto_resize(true)
            .build(ui, || {
                ui.text(format!("{} v{}", crate::TITLE, env!("CARGO_PKG_VERSION")));
                ui.text(env!("CARGO_PKG_DESCRIPTION"));
                ui.text("");
                ui.text(format!("License: {}", env!("CARGO_PKG_LICENSE")));
                ui.text(format!(
                    "Created by {}",
                    env!("CARGO_PKG_AUTHORS").split(':').join(", "),
                ));
            });
        // If the window closed, update preferences.
        prefs.needs_save |= !prefs.window_states.about;
    }

    #[cfg(debug_assertions)]
    if prefs.window_states.demo {
        ui.show_demo_window(&mut prefs.window_states.demo);
    }

    // Bulid the keybind popup.
    drop(prefs_guard);
    popups::build_keybind_popup(app);
    let mut prefs_guard = crate::get_prefs();
    let prefs = &mut prefs_guard;

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
    prefs.save();
}

struct RightStatusBarToken {
    sep_width: f32,
    start_x: f32,
    x: f32,
    y: f32,
    done: bool,
}
impl RightStatusBarToken {
    pub fn right_segment(&mut self, ui: &Ui<'_>, s: impl AsRef<str>) {
        let w = ui.calc_text_size(&s)[0] + self.sep_width;
        if self.x - w <= self.start_x {
            self.done = true;
        }
        if self.done {
            ui.text(""); // Reset "last item"
        } else {
            self.x -= w;
            ui.set_cursor_pos([self.x, self.y]);
            ui.separator();
            ui.text(s);
        }
    }
}
struct LeftStatusBarToken {
    sep_width: f32,
    end_x: f32,
    first: bool,
    done: bool,
}
impl LeftStatusBarToken {
    pub fn left_segment(&mut self, ui: &Ui<'_>, s: impl AsRef<str>) {
        if self.end_x < ui.cursor_pos()[0] + self.sep_width {
            self.done = true;
        }
        if self.done {
            ui.text(""); // Reset "last item"
        } else {
            if !self.first {
                self.first = false;
                ui.separator();
            }
            ui.push_text_wrap_pos_with_pos(self.end_x - ui.cursor_pos()[0]);
            ui.text_wrapped(s);
        }
    }
}
fn build_status_bar(
    ui: &Ui<'_>,
    build_right_segments: impl FnOnce(&mut RightStatusBarToken),
    build_left_segments: impl FnOnce(&mut LeftStatusBarToken),
) {
    let viewport_pos;
    let viewport_size;
    unsafe {
        let viewport = *imgui::sys::igGetMainViewport();
        viewport_pos = viewport.Pos;
        viewport_size = viewport.Size;
    }
    let w = viewport_size.x;
    let h = ui.frame_height();
    let x = viewport_pos.x;
    let y = viewport_pos.y + viewport_size.y - h;
    Window::new("Status bar")
        .position([x, y], Condition::Always)
        .size([w, h], Condition::Always)
        .flags(
            WindowFlags::NO_TITLE_BAR
                | WindowFlags::NO_RESIZE
                | WindowFlags::NO_MOVE
                | WindowFlags::NO_SCROLLBAR
                | WindowFlags::MENU_BAR,
        )
        .build(ui, || {
            ui.menu_bar(|| {
                let sep_width = ui.clone_style().item_spacing[0] * 2.0;

                // Display right segments, right-to-left.
                let [start_x, y] = ui.cursor_pos();
                let x = ui.content_region_max()[0];
                let mut tok = RightStatusBarToken {
                    sep_width,
                    start_x,
                    x,
                    y,
                    done: false,
                };
                build_right_segments(&mut tok);

                // Display left segments, left-to-right.
                ui.set_cursor_pos([start_x, y]);
                let mut tok = LeftStatusBarToken {
                    sep_width,
                    end_x: tok.x,
                    first: true,
                    done: false,
                };
                build_left_segments(&mut tok);
            });
        });
}

#[must_use]
fn build_keybind_table<C>(
    app: &mut AppState,
    extra_width: &mut f32,
    prefs: &mut Preferences,
    get_keybinds: impl 'static + Send + Clone + Fn(&mut Preferences) -> &mut Vec<Keybind<C>>,
    mut build_command_select_ui: impl FnMut(usize, &mut C) -> bool,
) -> bool {
    let mut needs_save = false;

    let ui = app.ui;

    let flags = TableFlags::BORDERS | TableFlags::SIZING_FIXED_FIT | TableFlags::SCROLL_Y;
    let table_token = match ui.begin_table_with_flags("keybinds", 3, flags) {
        Some(tok) => tok,
        None => return needs_save,
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

    let keybind_list = get_keybinds(prefs);

    for (i, keybind) in keybind_list.iter_mut().enumerate() {
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
        if ui.button_with_size(format!("{}##change_keybind{}", keybind.key, i), [w, 0.0]) {
            let get_keybinds = get_keybinds.clone();
            popups::open_keybind_popup(keybind.key, move |new_key| {
                let mut prefs = crate::get_prefs();
                get_keybinds(&mut *prefs)[i].key = new_key;
                prefs.needs_save = true;
            });
        }

        ui.table_next_column();
        needs_save |= build_command_select_ui(i, &mut keybind.command);

        ui.same_line();
        let extra_width_in_col = ui.content_region_avail()[0];
        if *extra_width > extra_width_in_col {
            *extra_width = extra_width_in_col
        }
    }

    if let Some(((_start, ref mut from), to)) = drag.as_mut().zip(drag_to) {
        keybind_list.swap(*from, to);
        *from = to;
        needs_save = true;
    }
    if let Some(i) = delete_idx {
        keybind_list.remove(i);
        needs_save = true;
    }

    drop(table_token);

    needs_save
}

#[must_use]
fn build_puzzle_command_select_ui(
    ui: &Ui<'_>,
    puzzle_type: PuzzleType,
    i: usize,
    command: &mut PuzzleCommand,
) -> bool {
    use PuzzleCommand as Cmd;

    let mut needs_save = false;

    #[derive(Display, EnumIter, Copy, Clone, PartialEq, Eq)]
    enum CmdType {
        None,
        Twist,
        Recenter,
        #[strum(serialize = "Select")]
        HoldSelect,
        #[strum(serialize = "Toggle select")]
        ToggleSelect,
        #[strum(serialize = "Clear selected")]
        ClearToggleSelect,
    }

    let mut cmd_type = match command {
        Cmd::Twist { .. } => CmdType::Twist,
        Cmd::Recenter { .. } => CmdType::Recenter,

        Cmd::HoldSelect(_) => CmdType::HoldSelect,
        Cmd::ToggleSelect(_) => CmdType::ToggleSelect,
        Cmd::ClearToggleSelect(_) => CmdType::ClearToggleSelect,

        Cmd::None => CmdType::None,
    };
    let old_cmd_type = cmd_type;

    let default_thing = SelectThing::Face(puzzle_type.faces()[0]);
    let default_direction = TwistDirection::default(puzzle_type);

    if build_select_combo_iter(
        ui,
        &format!("##command{}", i),
        &mut cmd_type,
        CmdType::iter(),
    ) && cmd_type != old_cmd_type
    {
        needs_save = true;
        *command = match cmd_type {
            CmdType::None => Cmd::None,

            CmdType::Twist => Cmd::Twist {
                face: None,
                direction: default_direction,
                layer_mask: LayerMask(1),
            },
            CmdType::Recenter => Cmd::Recenter { face: None },

            CmdType::HoldSelect => Cmd::HoldSelect(command.get_select_thing(puzzle_type)),
            CmdType::ToggleSelect => Cmd::ToggleSelect(command.get_select_thing(puzzle_type)),
            CmdType::ClearToggleSelect => Cmd::ClearToggleSelect(command.get_select_category()),
        }
    }

    if let Some(select_how) = command.get_select_how() {
        ui.same_line();

        let mut category = command.get_select_category();
        if build_select_combo_iter(
            ui,
            &format!("##select_category{}", i),
            &mut category,
            SelectCategory::iter(),
        ) {
            needs_save = true;
            *command = match select_how {
                SelectHow::Hold => Cmd::HoldSelect(SelectThing::default(category, puzzle_type)),
                SelectHow::Toggle => Cmd::ToggleSelect(SelectThing::default(category, puzzle_type)),
                SelectHow::Clear => Cmd::ClearToggleSelect(category),
            };
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
            direction,
            layer_mask,
        } => {
            combo_label(ui, "Face");
            needs_save |=
                build_optional_select_combo(ui, &format!("##face{}", i), face, puzzle_type.faces());

            combo_label(ui, "Layers");
            needs_save |= build_layer_mask_select_checkboxes(
                ui,
                &format!("##layer_mask{}", i),
                layer_mask,
                puzzle_type.layer_count(),
            );

            combo_label(ui, "Direction");
            needs_save |= build_select_combo_iter(
                ui,
                &format!("##direction{}", i),
                direction,
                TwistDirection::iter(puzzle_type),
            );
        }
        Cmd::Recenter { face } => {
            combo_label(ui, "Face");
            needs_save |=
                build_optional_select_combo(ui, &format!("##face{}", i), face, puzzle_type.faces());
        }

        Cmd::HoldSelect(thing) | Cmd::ToggleSelect(thing) => {
            ui.same_line();
            match thing {
                SelectThing::Face(face) => {
                    needs_save |=
                        build_select_combo(ui, &format!("##face{}", i), face, puzzle_type.faces())
                }
                SelectThing::Layers(layer_mask) => {
                    needs_save |= build_layer_mask_select_checkboxes(
                        ui,
                        &format!("##layer_mask{}", i),
                        layer_mask,
                        puzzle_type.layer_count(),
                    )
                }
                SelectThing::PieceType(piece_type) => {
                    needs_save |= build_select_combo_iter(
                        ui,
                        &format!("##piece_type{}", i),
                        piece_type,
                        PieceType::iter(puzzle_type),
                    )
                }
            }
        }
        Cmd::ClearToggleSelect(_) => (),
    }

    needs_save
}

#[must_use]
fn build_layer_mask_select_checkboxes(
    ui: &Ui<'_>,
    label: &str,
    layer_mask: &mut LayerMask,
    layer_count: usize,
) -> bool {
    let mut needs_save = false;

    let checkbox_padding = ui.clone_style().frame_padding[0];
    for l in 0..layer_count {
        needs_save |= ui.checkbox_flags(&format!("{}##{}", label, l), &mut layer_mask.0, 1 << l);
        ui.same_line_with_spacing(0.0, checkbox_padding);
        if ui.is_item_hovered() {
            ui.tooltip_text(format!("Layer {}", l + 1));
        }
    }

    needs_save
}

#[must_use]
fn build_optional_select_combo<T: AsRef<str> + Clone + PartialEq>(
    ui: &Ui<'_>,
    label: &str,
    selected: &mut Option<T>,
    items: &[T],
) -> bool {
    let mut choices = vec![None];
    choices.extend(items.iter().cloned().map(Some));

    fn to_string<'a, T: AsRef<str>>(item: &'a Option<T>) -> &'a str {
        match item {
            Some(x) => x.as_ref(),
            None => "(selected)",
        }
    }

    build_autosize_combo(ui, label, selected, &choices, to_string::<T>)
}

#[must_use]
fn build_select_combo_iter<T: ToString + Clone + PartialEq>(
    ui: &Ui<'_>,
    label: &str,
    selected: &mut T,
    items: impl IntoIterator<Item = T>,
) -> bool {
    build_select_combo(ui, label, selected, &items.into_iter().collect_vec())
}

#[must_use]
fn build_select_combo<T: ToString + Clone + PartialEq>(
    ui: &Ui<'_>,
    label: &str,
    selected: &mut T,
    items: &[T],
) -> bool {
    build_autosize_combo(ui, label, selected, items, |x| x.to_string())
}

#[must_use]
fn build_autosize_combo<'s, 't, T: Clone + PartialEq, S: 's + AsRef<str>>(
    ui: &Ui<'_>,
    label: &str,
    current_item: &mut T,
    items: &'t [T],
    to_string: fn(&'t T) -> S,
) -> bool {
    let strings = items.iter().map(to_string).collect_vec();
    let w = strings
        .iter()
        .map(|s| ui.calc_text_size(s)[0])
        .fold(0.0, |a, b| if b > a { b } else { a })
        + ui.text_line_height_with_spacing()
        + ui.clone_style().frame_padding[0] * 3.0;
    ui.set_next_item_width(w);
    let mut i = items
        .iter()
        .position(|item| item == current_item)
        .unwrap_or(0);
    if ui.combo(label, &mut i, &strings, |s| s.as_ref().into()) {
        *current_item = items[i].clone();
        true
    } else {
        false
    }
}
