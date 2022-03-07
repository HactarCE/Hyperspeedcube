//! A keyboard-controlled speedcube simulator.

#![allow(dead_code)]
// #![warn(missing_docs)]
#![allow(
    clippy::collapsible_match,
    clippy::match_like_matches_macro,
    clippy::single_match
)]

#[macro_use]
extern crate delegate;
#[macro_use]
extern crate enum_dispatch;
#[macro_use]
extern crate glium;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate strum;

use glium::glutin::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
    ContextBuilder,
};
use imgui::FontSource;
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use send_wrapper::SendWrapper;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

#[macro_use]
mod debug;

mod colors;
mod commands;
mod gui;
mod input;
mod preferences;
pub mod puzzle;
mod render;
mod serde_impl;

use commands::Command;
use preferences::{get_prefs, Preferences};
use puzzle::{Puzzle, PuzzleControllerTrait, PuzzleType};

/// The title of the window.
const TITLE: &str = "Hyperspeedcube";

fn main() {
    // Initialize runtime data.
    let mut puzzle = Puzzle::default();
    let mut input_state = input::State::default();
    let mut events_buffer = VecDeque::new();

    // Initialize imgui.
    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);
    let mut platform = WinitPlatform::init(&mut imgui);
    let gl_window = DISPLAY.gl_window();
    let window = gl_window.window();
    platform.attach_window(imgui.io_mut(), window, HiDpiMode::Locked(1.0));
    let mut renderer = None; // We'll initialize it on the first frame.
    let mut old_scale_factor = 1.0;
    let mut old_font_size = 1.0;

    // Load last open file.
    {
        let mut prefs = get_prefs();
        if let Some(path) = prefs.log_file.take() {
            try_load(&mut puzzle, &mut prefs, path.to_owned());
        }
    }

    // Main loop
    let mut last_frame_time = Instant::now();
    let mut next_frame_time = Instant::now();
    EVENTS_LOOP
        .borrow_mut()
        .take()
        .unwrap()
        .run(move |event, _ev_loop, control_flow| {
            let mut now = Instant::now();
            let mut do_frame = false;

            // Handle events.
            match event.to_static() {
                Some(Event::NewEvents(cause)) => match cause {
                    StartCause::ResumeTimeReached {
                        start: _,
                        requested_resume,
                    } => {
                        now = requested_resume;
                        do_frame = true;
                    }
                    StartCause::Init => {
                        next_frame_time = now;
                        do_frame = true;
                    }
                    _ => (),
                },

                // The program is about to exit.
                Some(Event::LoopDestroyed) => (),

                // Queue the event to be handled next time we render
                // everything.
                Some(ev) => events_buffer.push_back(ev),

                // Ignore this event.
                None => (),
            };

            // Update and draw.
            if do_frame && next_frame_time <= now {
                let frame_duration = get_prefs().gfx.frame_duration();
                next_frame_time = now + frame_duration;
                if next_frame_time < Instant::now() {
                    // Skip a frame (or several).
                    next_frame_time = Instant::now() + frame_duration;
                }
                *control_flow = ControlFlow::WaitUntil(next_frame_time);

                // Initialize imgui renderer before the first frame, or whenever
                // the scale factor or font size is updated.
                let renderer = {
                    // Has the scale factor changed?
                    let new_scale_factor = gl_window.window().scale_factor() as f32;
                    if old_scale_factor != new_scale_factor {
                        // If so, invalidate the renderer.
                        renderer = None;
                    }

                    // Has the font size changed?
                    let new_font_size = get_prefs().gfx.font_size;
                    if old_font_size != new_font_size && !get_prefs().gfx.lock_font_size {
                        // If so, invalidate the renderer.
                        renderer = None;
                    }

                    renderer.get_or_insert_with(|| {
                        imgui
                            .style_mut()
                            .scale_all_sizes(new_scale_factor / old_scale_factor);

                        imgui.fonts().clear();
                        imgui.fonts().add_font(&[FontSource::TtfData {
                            data: include_bytes!("../resources/font/NotoSans-Regular.ttf"),
                            size_pixels: (new_font_size * new_scale_factor).floor(),
                            config: None,
                        }]);

                        old_scale_factor = new_scale_factor;
                        old_font_size = new_font_size;

                        Renderer::init(&mut imgui, &**DISPLAY)
                            .expect("failed to initialize renderer")
                    })
                };

                // Prep imgui for event handling.
                let imgui_io = imgui.io_mut();
                platform
                    .prepare_frame(imgui_io, gl_window.window())
                    .expect("failed to start frame");

                if let Some(delta) = now.checked_duration_since(last_frame_time) {
                    imgui_io.update_delta_time(delta);
                    puzzle.advance(delta);
                }
                last_frame_time = now;

                let mut command_queue = vec![];

                // Prep the puzzle for event handling.
                let mut input_frame = input_state.frame(&puzzle, imgui_io, &mut command_queue);

                for ev in events_buffer.drain(..) {
                    // Let the keybind popup handle events.
                    if gui::keybind_popup_handle_event(&ev) {
                        continue;
                    }
                    // Handle events for imgui.
                    platform.handle_event(imgui_io, gl_window.window(), &ev);
                    // Handle events for the application.
                    input_frame.handle_event(&ev);
                }

                // Finish handling events for the application.
                input_frame.finish();

                // Prep imgui for rendering.
                let mouse_pos = imgui_io.mouse_pos;
                let ui = imgui.frame();
                let mut app = gui::AppState {
                    ui: &ui,
                    mouse_pos,
                    puzzle: &puzzle,
                    control_flow,
                    command_queue: &mut command_queue,
                };
                gui::build(&mut app);

                // Execute commands.
                puzzle.set_highlight(input_state.total_selection());
                for command in command_queue {
                    *crate::status_msg() = String::new();
                    let mut prefs = get_prefs();

                    match command {
                        Command::Open => {
                            if confirm_discard_changes(puzzle.is_unsaved(), "open another file") {
                                if let Some(path) = file_dialog().pick_file() {
                                    try_load(&mut puzzle, &mut prefs, path.to_owned());
                                }
                            }
                        }
                        Command::Save => match &prefs.log_file {
                            Some(path) => try_save(&mut puzzle, path),
                            None => try_save_as(&mut puzzle, &mut prefs),
                        },
                        Command::SaveAs => try_save_as(&mut puzzle, &mut prefs),
                        Command::Exit => {
                            if confirm_discard_changes(puzzle.is_unsaved(), "exit") {
                                *control_flow = ControlFlow::Exit;
                            }
                        }

                        Command::Undo => {
                            if !puzzle.undo() {
                                status_error("Nothing to undo");
                            }
                        }
                        Command::Redo => {
                            if !puzzle.redo() {
                                status_error("Nothing to redo");
                            }
                        }
                        Command::Reset => {
                            if confirm_discard_changes(puzzle.is_unsaved(), "reset puzzle") {
                                puzzle = Puzzle::new(puzzle.ty());
                            }
                        }

                        Command::NewPuzzle(puzzle_type) => {
                            if confirm_discard_changes(puzzle.is_unsaved(), "load new puzzle") {
                                puzzle = Puzzle::new(puzzle_type);
                                prefs.log_file = None;
                                *crate::status_msg() = format!("Loaded {}", puzzle_type);
                            }
                        }

                        Command::Twist {
                            face,
                            direction,
                            layer_mask,
                        } => {
                            if let Err(e) = puzzle.do_twist_command(face, direction, layer_mask) {
                                status_error(e);
                            }
                        }
                        Command::Recenter(face) => {
                            if let Err(e) = puzzle.do_recenter_command(face) {
                                status_error(e);
                            }
                        }

                        Command::ErrorMsg(e) => status_error(e),

                        Command::None => (),
                    }
                }

                let mut target = DISPLAY.draw();

                // Render the puzzle.
                render::draw_puzzle(&mut target, &puzzle);

                // Render imgui.
                platform.prepare_render(&ui, gl_window.window());
                let draw_data = ui.render();
                renderer
                    .render(&mut target, draw_data)
                    .expect("error while rendering imgui");

                // Put it all on the screen.
                target.finish().expect("failed to swap buffers");
            }
        });
}

lazy_static! {
    static ref EVENTS_LOOP: SendWrapper<RefCell<Option<EventLoop<()>>>> =
        SendWrapper::new(RefCell::new(Some(EventLoop::new())));
    static ref DISPLAY: SendWrapper<glium::Display> = SendWrapper::new({
        let wb = WindowBuilder::new()
            .with_title(TITLE.to_owned())
            .with_window_icon(load_application_icon());
        let cb = ContextBuilder::new()
            .with_vsync(false)
            .with_multisampling(get_prefs().gfx.msaa as u16);
        glium::Display::new(wb, cb, EVENTS_LOOP.borrow().as_ref().unwrap())
            .expect("failed to initialize display")
    });
    static ref STATUS_MSG: Mutex<String> = Mutex::new(String::new());
}

fn status_msg<'a>() -> MutexGuard<'a, String> {
    STATUS_MSG.lock().unwrap()
}
fn status_error(msg: impl fmt::Display) {
    *status_msg() = format!("Error: {}", msg);
}

fn load_application_icon() -> Option<Icon> {
    let icon_png_data = include_bytes!("../resources/icon/hyperspeedcube_32x32.png");
    let png_decoder = png::Decoder::new(&icon_png_data[..]);
    match png_decoder.read_info() {
        Ok(mut reader) => match reader.output_color_type() {
            (png::ColorType::Rgba, png::BitDepth::Eight) => {
                let mut img_data = vec![0_u8; reader.output_buffer_size()];
                if let Err(err) = reader.next_frame(&mut img_data) {
                    eprintln!("Failed to read icon data: {:?}", err);
                    return None;
                };
                let info = reader.info();
                match Icon::from_rgba(img_data, info.width, info.height) {
                    Ok(icon) => Some(icon),
                    Err(err) => {
                        eprintln!("Failed to construct icon: {:?}", err);
                        None
                    }
                }
            }
            other => {
                eprintln!(
                    "Failed to load icon data due to unknown color format: {:?}",
                    other,
                );
                None
            }
        },
        Err(err) => {
            eprintln!("Failed to load icon data: {:?}", err);
            None
        }
    }
}

fn file_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new()
        .add_filter("Magic Cube 4D Log Files", &["log"])
        .add_filter("All files", &["*"])
}
fn error_dialog(title: &str, e: impl fmt::Display) {
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(&e.to_string())
        .show();
}
fn confirm_discard_changes(is_unsaved: bool, action: &str) -> bool {
    !is_unsaved
        || rfd::MessageDialog::new()
            .set_title("Unsaved changes")
            .set_description(&format!("Discard puzzle state and {}?", action))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
}

fn try_load(puzzle: &mut Puzzle, prefs: &mut Preferences, path: PathBuf) {
    match crate::puzzle::PuzzleController::load_file(&path) {
        Ok(p) => {
            *puzzle = Puzzle::Rubiks4D(p);
            *crate::status_msg() = format!("Loaded log file from {}", path.display());
            prefs.log_file = Some(path);
            prefs.needs_save = true;
        }
        Err(e) => error_dialog("Unable to load log file", e),
    }
}

fn try_save_as(puzzle: &mut Puzzle, prefs: &mut Preferences) {
    if let Some(path) = file_dialog().save_file() {
        try_save(puzzle, &path);
        prefs.log_file = Some(path);
        prefs.needs_save = true;
    }
}
fn try_save(puzzle: &mut Puzzle, path: &Path) {
    match puzzle {
        Puzzle::Rubiks4D(p) => match p.save_file(path) {
            Ok(()) => {
                *crate::status_msg() = format!("Saved log file to {}", path.display());
            }
            Err(e) => error_dialog("Unable to save log file", e),
        },
        _ => error_dialog(
            "Unable to save log file",
            format!("Log files are only supported for {}.", PuzzleType::Rubiks4D),
        ),
    }
}
