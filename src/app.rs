use cgmath::Point2;
use itertools::Itertools;
use key_names::KeyMappingCode;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use winit::event::{ElementState, ModifiersState, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};

use crate::commands::{Command, PuzzleCommand};
use crate::preferences::{Key, Keybind, Preferences};
use crate::puzzle::*;
use crate::render::{GraphicsState, PuzzleRenderCache};

pub struct App {
    pub(crate) prefs: Preferences,

    events: EventLoopProxy<AppEvent>,

    pub(crate) puzzle: PuzzleController,
    pub(crate) render_cache: PuzzleRenderCache,
    pub(crate) puzzle_texture_size: (u32, u32),
    force_redraw: bool,

    /// Mouse cursor position relative to the puzzle texture.
    pub(crate) cursor_pos: Option<Point2<f32>>,
    /// Set of pressed keys.
    pressed_keys: HashSet<Key>,
    /// Set of pressed modifier keys.
    pressed_modifiers: ModifiersState,
    /// Grips that are tied to a held key.
    transient_grips: HashMap<Key, Grip>,
    /// Grip that is more permanent.
    pub(crate) toggle_grip: Grip,

    dragging_view_angle: bool,
    pub(crate) view_angle_offset: egui::Vec2,

    status_msg: String,
}
impl App {
    pub(crate) fn new(event_loop: &EventLoop<AppEvent>) -> Self {
        let mut this = Self {
            prefs: Preferences::load(None),

            events: event_loop.create_proxy(),

            puzzle: PuzzleController::default(),
            render_cache: PuzzleRenderCache::default(),
            puzzle_texture_size: (0, 0),
            force_redraw: true,

            cursor_pos: None,
            pressed_keys: HashSet::default(),
            pressed_modifiers: ModifiersState::default(),
            transient_grips: HashMap::default(),
            toggle_grip: Grip::default(),

            dragging_view_angle: false,
            view_angle_offset: egui::Vec2::default(),

            status_msg: String::default(),
        };

        // Always save preferences after opening.
        this.prefs.needs_save = true;

        // Load last open file.
        if let Some(path) = this.prefs.log_file.take() {
            this.try_load_puzzle(path);
        }

        this
    }

    pub(crate) fn request_redraw_puzzle(&mut self) {
        self.force_redraw = true;
    }
    pub(crate) fn draw_puzzle(&mut self, gfx: &mut GraphicsState) -> Option<wgpu::TextureView> {
        let ret = crate::render::draw_puzzle(self, gfx, self.force_redraw);
        self.force_redraw = false;
        ret
    }

    pub(crate) fn event(&self, event: impl Into<AppEvent>) {
        self.events
            .send_event(event.into())
            .expect("tried to send event but event loop doesn't exist")
    }

    pub(crate) fn handle_app_event(&mut self, event: AppEvent, control_flow: &mut ControlFlow) {
        self.clear_status();
        if let Err(e) = self.handle_app_event_internal(event, control_flow) {
            self.set_status_err(e);
        }
    }
    fn handle_app_event_internal(
        &mut self,
        event: AppEvent,
        control_flow: &mut ControlFlow,
    ) -> Result<(), String> {
        match event {
            AppEvent::Command(c) => match c {
                Command::Open => {
                    if self.confirm_discard_changes("open another file") {
                        if let Some(path) = file_dialog().pick_file() {
                            self.try_load_puzzle(path);
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
                    self.puzzle.undo()?;
                }
                Command::Redo => {
                    self.puzzle.redo()?;
                }
                Command::Reset => {
                    if self.confirm_discard_changes("reset puzzle") {
                        self.puzzle.reset();
                    }
                }

                Command::ScrambleN(n) => {
                    if self.confirm_discard_changes("scramble") {
                        // TODO: `n` is not validated. It may be `0` (which is
                        // harmless) or `usize::MAX` (which will freeze the
                        // program).
                        self.puzzle.scramble_n(n)?;
                        self.set_status_ok(format!(
                            "Scrambled with {} random {}",
                            n,
                            if n == 1 { "move" } else { "moves" }
                        ));
                    }
                }
                Command::ScrambleFull => {
                    if self.confirm_discard_changes("scramble") {
                        self.puzzle.scramble_full()?;
                        self.set_status_ok("Scrambled fully");
                    }
                }

                Command::NewPuzzle(puzzle_type) => {
                    if self.confirm_discard_changes("reset puzzle") {
                        self.puzzle = PuzzleController::new(puzzle_type);
                        self.prefs.log_file = None;
                        self.set_status_ok(format!("Loaded {}", puzzle_type));
                    }
                }

                Command::ToggleBlindfold => {
                    self.prefs.colors.blindfold ^= true;
                    if self.prefs.colors.blindfold {
                        self.puzzle.show(|_| true);
                    }
                    self.prefs.needs_save = true;
                    self.request_redraw_puzzle();
                }

                Command::None => (),
            },

            AppEvent::Twist(twist) => {
                self.puzzle.twist(twist)?;
            }

            AppEvent::Click(mouse_button) => {
                if self.pressed_modifiers() == ModifiersState::SHIFT {
                    if let Some(sticker) = self.puzzle.hovered_sticker() {
                        match mouse_button {
                            egui::PointerButton::Primary => self.puzzle.toggle_select(sticker),
                            egui::PointerButton::Secondary => self.puzzle.select(sticker),
                            egui::PointerButton::Middle => (),
                        }
                    } else {
                        self.puzzle.deselect_all();
                    }
                    return Ok(());
                }
                if self.puzzle.current_twist().is_none() {
                    if let Some(twists) = self.puzzle.hovered_twists() {
                        let twist = match mouse_button {
                            egui::PointerButton::Primary => twists.ccw,
                            egui::PointerButton::Secondary => twists.cw,
                            egui::PointerButton::Middle => twists.recenter,
                        };
                        if let Some(mut t) = twist {
                            t.layers = self.gripped_layers(t.layers);
                            self.puzzle.twist(t)?;
                        }
                    }
                }
            }
            AppEvent::Drag(delta) => {
                self.dragging_view_angle = true;
                self.view_angle_offset += delta * self.prefs.interaction.drag_sensitivity * 360.0;
                let pitch = self.prefs.view[self.puzzle.ty()].pitch;
                self.view_angle_offset.y =
                    self.view_angle_offset.y.clamp(-90.0 - pitch, 90.0 - pitch);
                self.view_angle_offset.x =
                    (self.view_angle_offset.x + 180.0).rem_euclid(360.0) - 180.0;
            }
            AppEvent::DragReleased => {
                self.dragging_view_angle = false;
            }

            AppEvent::StatusError(msg) => return Err(msg),
        }
        Ok(())
    }
    pub(crate) fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.event(Command::Exit),

            WindowEvent::DroppedFile(path) => {
                if self.confirm_discard_changes("open another file") {
                    self.try_load_puzzle(path.to_owned());
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.pressed_modifiers = *mods;
                // Sometimes we miss key events for modifiers when the left and
                // right modifiers are both pressed at once (at least in my
                // testing on Windows 11) so clean that up here just in case.
                self.remove_held_grips(|k| {
                    // If the grip requires a modifier and that modifier is not
                    // pressed, then remove the grip.
                    k.is_shift() && !mods.shift()
                        || k.is_ctrl() && !mods.ctrl()
                        || k.is_alt() && !mods.alt()
                        || k.is_logo() && !mods.logo()
                });
            }

            WindowEvent::KeyboardInput { input, .. } => {
                let sc = key_names::sc_to_key(input.scancode as u16);
                let vk = input.virtual_keycode;

                match input.state {
                    ElementState::Pressed => {
                        if let Some(sc) = sc {
                            self.pressed_keys.insert(Key::Sc(sc));
                        }
                        if let Some(vk) = vk {
                            self.pressed_keys.insert(Key::Vk(vk));
                        }

                        // Only allow one twist command per keypress. Don't use
                        // multiple keybinds for macros.
                        let mut done_twist_command = false;

                        let puzzle_keybinds = &self.prefs.puzzle_keybinds[self.puzzle.ty()];
                        for bind in self.resolve_keypress(puzzle_keybinds, sc, vk) {
                            let key = bind.key.key().unwrap();
                            match &bind.command {
                                PuzzleCommand::GripAxis(axis_name) => {
                                    match self.twist_axis_from_name(Some(axis_name)) {
                                        Ok(twist_axis) => {
                                            let new_grip = Grip::with_axis(twist_axis);
                                            self.transient_grips.insert(key, new_grip);
                                        }
                                        Err(e) => self.event(AppEvent::StatusError(e)),
                                    }
                                }
                                PuzzleCommand::GripLayers(layers) => {
                                    let new_grip = Grip::with_layers(
                                        layers.to_layer_mask(self.puzzle.layer_count()),
                                    );
                                    self.transient_grips.insert(key, new_grip);
                                }
                                PuzzleCommand::Twist {
                                    axis,
                                    direction,
                                    layers,
                                } => {
                                    if !done_twist_command {
                                        done_twist_command = true;
                                        let layers =
                                            layers.to_layer_mask(self.puzzle.layer_count());
                                        if let Err(e) =
                                            self.do_twist(axis.as_deref(), direction, layers)
                                        {
                                            self.event(AppEvent::StatusError(e));
                                        }
                                    }
                                }
                                PuzzleCommand::Recenter { axis } => {
                                    if !done_twist_command {
                                        done_twist_command = true;
                                        if let Err(e) = self.do_recenter(axis.as_deref()) {
                                            self.event(AppEvent::StatusError(e));
                                        }
                                    }
                                }

                                PuzzleCommand::None => return, // Do not try to match other keybinds.
                            }
                        }

                        for bind in self.resolve_keypress(&self.prefs.general_keybinds, sc, vk) {
                            match &bind.command {
                                Command::None => return, // Do not try to match other keybinds.

                                _ => self.event(bind.command.clone()),
                            }
                        }
                    }

                    ElementState::Released => {
                        if let Some(sc) = sc {
                            self.pressed_keys.remove(&Key::Sc(sc));
                        }
                        if let Some(vk) = vk {
                            self.pressed_keys.remove(&Key::Vk(vk));
                        }

                        // Remove grips for this held key.
                        self.remove_held_grips(|k| {
                            Some(k) == sc.map(Key::Sc) || Some(k) == vk.map(Key::Vk)
                        });
                    }
                }
            }

            _ => (),
        }
    }

    pub(crate) fn resolve_keypress<'a, C>(
        &self,
        keybinds: &'a [Keybind<C>],
        sc: Option<KeyMappingCode>,
        vk: Option<VirtualKeyCode>,
    ) -> Vec<&'a Keybind<C>> {
        let sc = sc.map(Key::Sc);
        let vk = vk.map(Key::Vk);

        // Sometimes, we want to ignore certain modifier keys when resolving a
        // keypress -- in particular, if another keybind has already consumed
        // the modifier.
        //
        // For example, if `Shift` is bound to "grip layer 2," then a keybind
        // bound to `A` will still match `Shift`+`A` because `Shift` is in the
        // "ignored modifiers" set.
        //
        // A modifier is also ignored when matching its own key, hence
        // `.chain(&sc).chain(&vk)`. For example, the shift modifier is ignored
        // when matching the shift key.
        let ignored_keys = self.transient_grips.keys().chain(&sc).chain(&vk);
        let modifiers_mask = ignored_keys.fold(
            // Consider all modifiers, but don't distinguish left vs. right.
            ModifiersState::SHIFT
                | ModifiersState::CTRL
                | ModifiersState::ALT
                | ModifiersState::LOGO,
            // Ignore held greps and the key currently being pressed.
            |mods, key_to_ignore| mods & !key_to_ignore.modifier_bit(),
        );

        keybinds
            .iter()
            .filter(move |bind| {
                let key_combo = bind.key;
                let key = key_combo.key();
                let key_matches = (sc.is_some() && sc == key) || (vk.is_some() && vk == key);
                let mods_match =
                    key_combo.mods() & modifiers_mask == self.pressed_modifiers() & modifiers_mask;
                key_matches && mods_match
            })
            .collect()
    }

    fn twist_axis_from_name(&self, name: Option<&str>) -> Result<TwistAxis, String> {
        let name = name.ok_or("No twist axis gripped")?;
        self.puzzle
            .ty()
            .twist_axis_from_name(name)
            .ok_or_else(|| format!("Unknown twist axis {name:?}"))
    }
    fn twist_direction_from_name(&self, name: &str) -> Result<TwistDirection, String> {
        self.puzzle
            .twist_direction_from_name(name)
            .ok_or_else(|| format!("Unknown twist direction {name:?}"))
    }

    /// If `preferred` is supplied, returns the twist axis with that name;
    /// otherwise, returns the gripped twist axis if exactly one twist axis is
    /// gripped; otherwise returns `None`.
    pub(crate) fn gripped_twist_axis(&self, preferred: Option<&str>) -> Result<TwistAxis, String> {
        if let Some(name) = preferred {
            return self
                .puzzle
                .twist_axis_from_name(name)
                .ok_or_else(|| format!("Unknown twist axis {name:?}"));
        }
        self.grip().axes.iter().copied().exactly_one().map_err(|e| {
            if e.len() == 0 {
                "No twist axis gripped".to_string()
            } else {
                "Too many twist axes gripped".to_string()
            }
        })
    }
    /// If `fallback` is non-default, returns that; otherwise, returns the
    /// gripped layers.
    pub(crate) fn gripped_layers(&self, fallback: LayerMask) -> LayerMask {
        if fallback != LayerMask::default() {
            fallback
        } else {
            self.grip().layers.unwrap_or_default()
        }
    }

    pub(crate) fn do_twist(
        &self,
        twist_axis: Option<&str>,
        direction: &str,
        layers: LayerMask,
    ) -> Result<(), String> {
        self.event(AppEvent::Twist(Twist {
            axis: self.gripped_twist_axis(twist_axis)?,
            direction: self.twist_direction_from_name(direction)?,
            layers: self.gripped_layers(layers),
        }));
        Ok(())
    }
    pub(crate) fn do_recenter(&self, twist_axis: Option<&str>) -> Result<(), String> {
        let axis = self.gripped_twist_axis(twist_axis)?;
        self.event(AppEvent::Twist(self.puzzle.make_recenter_twist(axis)?));
        Ok(())
    }

    pub(crate) fn pressed_keys(&self) -> &HashSet<Key> {
        &self.pressed_keys
    }
    pub(crate) fn pressed_modifiers(&self) -> ModifiersState {
        self.pressed_modifiers
    }

    pub(crate) fn frame(&mut self, delta: Duration) {
        self.puzzle.set_grip(self.grip());
        if self.puzzle.check_just_solved() {
            self.set_status_ok("Solved!");
        }
        if !self.dragging_view_angle {
            self.view_angle_offset *= 0.02_f32.powf(delta.as_secs_f32());
            if self.view_angle_offset.length_sq() < 0.01 {
                self.view_angle_offset = egui::Vec2::ZERO;
            }
        }
    }

    fn confirm_load_puzzle(&self, warnings: &[String]) -> bool {
        warnings.is_empty()
            || rfd::MessageDialog::new()
                .set_title("Errors loading file")
                .set_description(&format!(
                    "The following errors were encountered \
                     while loading this file. Load anyway?\n\n{}",
                    warnings.iter().join("\n"),
                ))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
    }

    fn confirm_discard_changes(&self, action: &str) -> bool {
        let mut needs_save = self.puzzle.is_unsaved();

        if self.prefs.interaction.confirm_discard_only_when_scrambled
            && !self.puzzle.has_been_fully_scrambled()
        {
            needs_save = false;
        }

        !needs_save
            || rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description(&format!("Discard puzzle state and {}?", action))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
    }

    fn try_load_puzzle(&mut self, path: PathBuf) {
        match crate::logfile::load_file(&path) {
            Ok((p, warnings)) => {
                if self.confirm_load_puzzle(&warnings) {
                    self.puzzle = p;

                    self.set_status_ok(format!("Loaded log file from {}", path.display()));

                    self.prefs.log_file = Some(path);
                    self.prefs.needs_save = true;
                }
            }
            Err(e) => show_error_dialog(
                "Unable to load log file",
                format!("Unable to load log file:\n\n{e}"),
            ),
        }
    }
    fn try_save_puzzle(&mut self, path: &Path) {
        match crate::logfile::save_file(path, &mut self.puzzle) {
            Ok(()) => {
                self.prefs.log_file = Some(path.to_path_buf());
                self.prefs.needs_save = true;

                self.set_status_ok(format!("Saved log file to {}", path.display()));
            }
            Err(e) => show_error_dialog("Unable to save log file", e),
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
    fn clear_status(&mut self) {
        self.status_msg = String::new();
    }
    fn set_status_ok(&mut self, msg: impl fmt::Display) {
        self.status_msg = msg.to_string()
    }
    fn set_status_err(&mut self, msg: impl fmt::Display) {
        self.status_msg = format!("Error: {}", msg)
    }

    pub(crate) fn grip(&self) -> Grip {
        let mut ret = self
            .transient_grips
            .values()
            .fold(Grip::default(), |a, b| a | b);
        ret.axes.extend(&self.toggle_grip.axes);
        if ret.layers.is_none() {
            ret.layers = self.toggle_grip.layers;
        }
        ret
    }
    fn remove_held_grips(&mut self, mut remove_if: impl FnMut(Key) -> bool) {
        self.transient_grips.retain(|&k, _v| !remove_if(k));
    }
}

#[derive(Debug)]
pub(crate) enum AppEvent {
    Command(Command),

    Twist(Twist),

    Click(egui::PointerButton),
    Drag(egui::Vec2),
    DragReleased,

    StatusError(String),
}
impl From<Command> for AppEvent {
    fn from(c: Command) -> Self {
        Self::Command(c)
    }
}
impl From<Twist> for AppEvent {
    fn from(t: Twist) -> Self {
        Self::Twist(t)
    }
}

fn file_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new()
        .add_filter("Hyperspeedcube Log Files", &["hsc", "log"])
        .add_filter("All files", &["*"])
}
fn show_error_dialog(title: &str, e: impl fmt::Display) {
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(&e.to_string())
        .show();
}
