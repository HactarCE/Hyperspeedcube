use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};

use crate::commands::Command;
use crate::preferences::Preferences;
use crate::puzzle::{
    Face, LayerMask, Puzzle, PuzzleController, PuzzleControllerTrait, PuzzleType, TwistDirection,
};
use crate::render::PuzzleRenderCache;

pub struct App {
    pub(crate) prefs: Preferences,

    events: EventLoopProxy<AppEvent>,

    pub(crate) puzzle: Puzzle,
    pub(crate) render_cache: PuzzleRenderCache,
    pub(crate) puzzle_texture_size: (u32, u32),
    pub(crate) wants_repaint: bool,

    status_msg: String,
}
impl App {
    pub(crate) fn new(prefs: Preferences, event_loop: &EventLoop<AppEvent>) -> Self {
        let mut this = Self {
            prefs,

            events: event_loop.create_proxy(),

            puzzle: Puzzle::default(),
            render_cache: PuzzleRenderCache::default(),
            puzzle_texture_size: (1, 1),
            wants_repaint: true,

            status_msg: String::new(),
        };

        // Always save preferences after opening.
        this.prefs.needs_save = true;

        // Load last open file.
        if let Some(path) = this.prefs.log_file.take() {
            this.try_load_puzzle(path.to_owned());
        }

        this
    }

    pub(crate) fn event(&self, event: impl Into<AppEvent>) {
        self.events
            .send_event(event.into())
            .expect("tried to send event but event loop doesn't exist")
    }

    pub(crate) fn handle_app_event(&mut self, event: AppEvent, control_flow: &mut ControlFlow) {
        match event {
            AppEvent::Command(c) => match c {
                Command::Open => {
                    if self.confirm_discard_changes("open another file") {
                        if let Some(path) = file_dialog().pick_file() {
                            self.try_load_puzzle(path.to_owned());
                        }
                    }
                }
                Command::Save => match self.prefs.log_file.clone() {
                    Some(path) => self.try_save_puzzle(&path),
                    None => self.try_save_puzzle_as(),
                },
                Command::SaveAs => self.try_save_puzzle_as(),
                Command::Exit => {
                    if self.confirm_discard_changes("exit") {
                        *control_flow = ControlFlow::Exit;
                    }
                }

                Command::Undo => {
                    if !self.puzzle.undo() {
                        self.set_status_err("Nothing to undo");
                    }
                }
                Command::Redo => {
                    if !self.puzzle.redo() {
                        self.set_status_err("Nothing to redo");
                    }
                }
                Command::Reset => {
                    if self.confirm_discard_changes("reset puzzle") {
                        self.puzzle = Puzzle::new(self.puzzle.ty());
                        self.wants_repaint = true;
                    }
                }

                Command::NewPuzzle(puzzle_type) => {
                    if self.confirm_discard_changes("reset puzzle") {
                        self.puzzle = Puzzle::new(puzzle_type);
                        self.wants_repaint = true;
                        self.prefs.log_file = None;
                        self.set_status_ok(format!("Loaded {}", puzzle_type));
                    }
                }

                Command::None => (),
            },

            AppEvent::Twist {
                face,
                direction,
                layer_mask,
            } => {
                if let Err(e) = self.puzzle.do_twist_command(face, direction, layer_mask) {
                    self.set_status_err(e);
                }
            }
            AppEvent::Recenter(face) => {
                if let Err(e) = self.puzzle.do_recenter_command(face) {
                    self.set_status_err(e);
                }
            }
        }
    }
    pub(crate) fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.event(Command::Exit),
            WindowEvent::DroppedFile(_) => println!("dropped file!"),
            WindowEvent::HoveredFile(_) => println!("hovered file!"),
            WindowEvent::HoveredFileCancelled => println!("nvm hovered file"),
            WindowEvent::ReceivedCharacter(_) => (/* todo */),
            WindowEvent::Focused(_) => (/* todo */),
            WindowEvent::KeyboardInput {
                device_id,
                input,
                is_synthetic,
            } => (/* todo */),
            WindowEvent::ModifiersChanged(_) => (/* todo */),
            WindowEvent::CursorMoved {
                device_id,
                position,
                modifiers,
            } => (/* todo */),
            WindowEvent::CursorEntered { device_id } => (/* todo */),
            WindowEvent::CursorLeft { device_id } => (/* todo */),
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
                modifiers,
            } => (/* todo */),
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
                modifiers,
            } => (/* todo */),
            WindowEvent::TouchpadPressure {
                device_id,
                pressure,
                stage,
            } => (/* todo */),
            WindowEvent::AxisMotion {
                device_id,
                axis,
                value,
            } => (/* todo */),
            WindowEvent::Touch(_) => (/* todo */),
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => (/* todo */),
            // WindowEvent::ThemeChanged(theme) => match theme {
            // },
            _ => (),
        }
    }

    pub(crate) fn frame(&mut self, delta: Duration) {
        self.wants_repaint |= self.puzzle.advance(delta);
    }

    fn confirm_discard_changes(&self, action: &str) -> bool {
        !self.puzzle.is_unsaved()
            || rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description(&format!("Discard puzzle state and {}?", action))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
    }

    fn try_load_puzzle(&mut self, path: PathBuf) {
        match PuzzleController::load_file(&path) {
            Ok(p) => {
                self.puzzle = Puzzle::Rubiks4D(p);
                self.wants_repaint = true;

                self.set_status_ok(format!("Loaded log file from {}", path.display()));

                self.prefs.log_file = Some(path);
                self.prefs.needs_save = true;
            }
            Err(e) => show_error_dialog("Unable to load log file", e),
        }
    }
    fn try_save_puzzle(&mut self, path: &Path) {
        match &mut self.puzzle {
            Puzzle::Rubiks4D(p) => match p.save_file(path) {
                Ok(()) => {
                    self.prefs.log_file = Some(path.to_path_buf());
                    self.prefs.needs_save = true;

                    self.set_status_ok(format!("Saved log file to {}", path.display()));
                }
                Err(e) => show_error_dialog("Unable to save log file", e),
            },
            _ => show_error_dialog(
                "Unable to save log file",
                format!("Log files are only supported for {}.", PuzzleType::Rubiks4D),
            ),
        }
    }
    fn try_save_puzzle_as(&mut self) {
        if let Some(path) = file_dialog().save_file() {
            self.try_save_puzzle(&path)
        }
    }

    pub(crate) fn status_msg(&self) -> &str {
        &self.status_msg
    }
    fn set_status_ok(&mut self, msg: impl fmt::Display) {
        self.status_msg = msg.to_string()
    }
    fn set_status_err(&mut self, msg: impl fmt::Display) {
        self.status_msg = format!("Error: {}", msg)
    }
}

#[derive(Debug)]
pub(crate) enum AppEvent {
    Command(Command),

    Twist {
        face: Face,
        direction: TwistDirection,
        layer_mask: LayerMask,
    },
    Recenter(Face),
}
impl From<Command> for AppEvent {
    fn from(c: Command) -> Self {
        Self::Command(c)
    }
}

fn file_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new()
        .add_filter("Magic Cube 4D Log Files", &["log"])
        .add_filter("All files", &["*"])
}
fn show_error_dialog(title: &str, e: impl fmt::Display) {
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(&e.to_string())
        .show();
}
