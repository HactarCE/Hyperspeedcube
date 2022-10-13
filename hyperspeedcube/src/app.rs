use bitvec::bitvec;
use cgmath::Point2;
use itertools::Itertools;
use key_names::KeyMappingCode;
use ndpuzzle::puzzle::jumbling::JumblingPuzzleSpec;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use winit::event::{ElementState, ModifiersState, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};

use crate::commands::{Command, PuzzleCommand, PuzzleMouseCommand};
use crate::preferences::{Key, Keybind, PieceFilter, Preferences, ViewPreferences};
use crate::puzzle::*;
use crate::render::{GraphicsState, PuzzleRenderCache};

pub struct App {
    pub(crate) prefs: Preferences,

    events: EventLoopProxy<AppEvent>,

    pub(crate) puzzle: PuzzleController,
    pub(crate) render_cache: PuzzleRenderCache,
    pub(crate) puzzle_texture_size: (u32, u32),
    force_redraw: bool,

    /// Mouse cursor position relative to the puzzle texture. Each axis ranges
    /// from -1.0 to +1.0.
    pub(crate) cursor_pos: Option<Point2<f32>>,

    /// Set of pressed keys.
    pressed_keys: HashSet<Key>,
    /// Set of keys toggled on using buttons in the UI.
    toggled_keys: HashSet<Key>,
    /// Set of pressed modifier keys.
    pressed_modifiers: ModifiersState,
    /// Set of modifiers toggled on using buttons in the UI.
    toggled_modifiers: ModifiersState,

    /// Grips that are tied to a held key.
    transient_grips: HashMap<Key, Grip>,
    /// Grip that is more permanent.
    pub(crate) toggle_grip: Grip,

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
            toggled_keys: HashSet::default(),
            pressed_modifiers: ModifiersState::default(),
            toggled_modifiers: ModifiersState::default(),

            transient_grips: HashMap::default(),
            toggle_grip: Grip::default(),

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
                    if let Some(ty) = PUZZLE_REGISTRY.lock().get(&puzzle_type) {
                        if self.confirm_discard_changes("reset puzzle") {
                            self.puzzle = PuzzleController::new(ty);
                            self.set_status_ok(format!("Loaded {}", puzzle_type));
                        }
                    } else {
                        show_error_dialog("Missing puzzle", "Puzzle does not exist");
                    }
                }

                Command::ToggleBlindfold => {
                    self.prefs.colors.blindfold ^= true;
                    if self.prefs.colors.blindfold {
                        self.puzzle.visible_pieces_mut().fill(true);
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
                let modifiers_mask = self.modifiers_mask(None, None);
                let matching_mousebind = self.prefs.mousebinds.iter().find(|bind| {
                    egui::PointerButton::from(bind.button) == mouse_button
                        && bind.mods() & modifiers_mask == self.pressed_modifiers() & modifiers_mask
                });
                if let Some(bind) = matching_mousebind {
                    match bind.command {
                        PuzzleMouseCommand::TwistCw => self.click_twist(|tw| tw.cw)?,
                        PuzzleMouseCommand::TwistCcw => self.click_twist(|tw| tw.ccw)?,
                        PuzzleMouseCommand::Recenter => self.click_twist(|tw| tw.recenter)?,
                        PuzzleMouseCommand::SelectPiece => {
                            if let Some(sticker) = self.puzzle.hovered_sticker() {
                                self.puzzle.toggle_select(sticker);
                            } else {
                                self.puzzle.deselect_all();
                            }
                        }
                        PuzzleMouseCommand::None => (),
                    }
                }
            }
            AppEvent::Drag(delta) => {
                let delta = delta * self.prefs.interaction.drag_sensitivity * 360.0;
                self.puzzle.add_view_angle_offset(
                    [delta.x, delta.y],
                    self.prefs.view(self.puzzle.ty()),
                    self.pressed_modifiers().shift(),
                );
            }
            AppEvent::DragReleased => {
                if self.prefs.interaction.snap_on_release {
                    self.puzzle.snap_view_angle_offset();
                }
            }

            AppEvent::Scroll(delta) => {
                let scale = &mut self.prefs.view_mut(&self.puzzle.ty()).scale;
                *scale = (*scale * (delta.y / 256.0).exp()).clamp(
                    *ViewPreferences::SCALE_RANGE.start(),
                    *ViewPreferences::SCALE_RANGE.end(),
                );
                self.prefs.needs_save = true;
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
                    // self.try_load_puzzle(path.to_owned());
                    if let Ok(s) = std::fs::read_to_string(path) {
                        match serde_yaml::from_str::<JumblingPuzzleSpec>(&s) {
                            Ok(spec) => {
                                let name = spec.name.clone();
                                PUZZLE_REGISTRY
                                    .lock()
                                    .insert(name.clone(), spec.build().expect("Sadness"));
                                self.event(Command::NewPuzzle(name))
                            }
                            Err(e) => show_error_dialog("Error loading puzzle", e),
                        }
                    }
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.pressed_modifiers = *mods;
                // Sometimes we miss key events for modifiers when the left and
                // right modifiers are both pressed at once (at least in my
                // testing on Windows 11) so clean that up here just in case.
                let mods = self.pressed_modifiers();
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

                        self.handle_key_press(sc, vk);
                    }

                    ElementState::Released => {
                        if let Some(sc) = sc {
                            self.pressed_keys.remove(&Key::Sc(sc));
                        }
                        if let Some(vk) = vk {
                            self.pressed_keys.remove(&Key::Vk(vk));
                        }

                        self.handle_key_release(sc, vk);
                    }
                }
            }

            _ => (),
        }
    }

    fn click_twist(
        &mut self,
        get_twist: fn(ClickTwists) -> Option<Twist>,
    ) -> Result<(), &'static str> {
        if self.puzzle.current_twist().is_none() {
            if let Some(twists) = self.puzzle.hovered_twists() {
                if let Some(mut t) = get_twist(twists) {
                    t.layers = self.gripped_layers(t.layers);
                    self.puzzle.twist(t)?;
                }
            }
        }
        Ok(())
    }

    fn handle_key_press(&mut self, sc: Option<KeyMappingCode>, vk: Option<VirtualKeyCode>) {
        // Only allow one twist command per keypress. Don't use
        // multiple keybinds for macros.
        let mut done_twist_command = false;

        // Sometimes users will bind a twist command and another command to the
        // same key, so if the twist command fails due to an incomplete grip
        // then the other command will execute. For that reason, errors that
        // result from an incomplete grip should only be shown if no other
        // command succeeded.
        let mut success = false;
        let mut grip_error = None;

        let active_puzzle_keybinds =
            self.prefs.puzzle_keybinds[self.puzzle.ty()].get_active_keybinds();
        for bind in self.resolve_keypress(active_puzzle_keybinds, sc, vk) {
            let key = bind.key.key().unwrap();
            match &bind.command {
                PuzzleCommand::Grip { axis, layers } => {
                    let mut new_grip = Grip::default();

                    if let Some(axis_name) = axis {
                        match self.twist_axis_from_name(Some(axis_name)) {
                            Ok(twist_axis) => {
                                new_grip.toggle_axis(twist_axis, true);
                            }
                            Err(e) => self.event(AppEvent::StatusError(e)),
                        }
                    }

                    new_grip.layers = Some(layers.to_layer_mask(self.puzzle.ty().layer_count))
                        .filter(|&l| l != LayerMask(0));

                    self.transient_grips.insert(key, new_grip);

                    success = true;
                }
                PuzzleCommand::Twist {
                    axis,
                    direction,
                    layers,
                } => {
                    if !done_twist_command {
                        self.puzzle.snap_view_angle_offset();
                        let layers = layers.to_layer_mask(self.puzzle.ty().layer_count);
                        match self.do_twist(axis.as_deref(), direction, layers) {
                            Ok(()) => {
                                done_twist_command = true;
                                success = true;
                            }
                            Err(e) => grip_error = Some(e),
                        }
                    }
                }
                PuzzleCommand::Recenter { axis } => {
                    if !done_twist_command {
                        self.puzzle.snap_view_angle_offset();
                        match self.do_recenter(axis.as_deref()) {
                            Ok(()) => {
                                done_twist_command = true;
                                success = true;
                            }
                            Err(e) => grip_error = Some(e),
                        }
                    }
                }

                PuzzleCommand::Filter { mode, filter_name } => {
                    let piece_filters = &self.prefs.piece_filters[self.puzzle.ty()];
                    let preset = match piece_filters.iter().find(|p| p.preset_name == *filter_name)
                    {
                        Some(p) => p.value.clone(),
                        None if filter_name == "Everything" => PieceFilter {
                            visible_pieces: bitvec![1; self.puzzle.ty().pieces.len()],
                            hidden_opacity: None,
                        },
                        None => {
                            self.set_status_err(format!(
                                "Unable to find piece filter {filter_name:?}"
                            ));
                            return;
                        }
                    };
                    let piece_set = preset.visible_pieces.clone();
                    let current = self.puzzle.visible_pieces();
                    let new_piece_set = match mode {
                        crate::commands::FilterMode::ShowExactly => {
                            if let Some(opacity) = preset.hidden_opacity {
                                self.prefs.opacity.hidden = opacity;
                                self.prefs.needs_save = true;
                                self.force_redraw = true;
                            }
                            piece_set
                        }
                        crate::commands::FilterMode::Show => piece_set | current,
                        crate::commands::FilterMode::Hide => !piece_set & current,
                        crate::commands::FilterMode::HideAllExcept => piece_set & current,
                        crate::commands::FilterMode::Toggle => {
                            if (piece_set.clone() & current).any() {
                                !piece_set & current
                            } else {
                                piece_set | current
                            }
                        }
                    };
                    self.puzzle.set_visible_pieces(&new_piece_set);

                    success = true;
                }

                PuzzleCommand::KeybindSet { keybind_set_name } => {
                    let set_name = keybind_set_name.clone();
                    let puzzle_keybinds = &mut self.prefs.puzzle_keybinds[self.puzzle.ty()];
                    if puzzle_keybinds.get(&set_name).is_some() {
                        puzzle_keybinds.active = set_name.clone();
                        self.set_status_ok(format!("Switched to {set_name} keybinds"));
                    } else {
                        self.set_status_err(format!("No keybind set named {set_name}"));
                    }
                    return; // Do not try to match other keybinds.
                }

                PuzzleCommand::None => return, // Do not try to match other keybinds.
            }
        }

        for bind in self.resolve_keypress(&self.prefs.global_keybinds, sc, vk) {
            match &bind.command {
                Command::None => return, // Do not try to match other keybinds.

                _ => {
                    self.event(bind.command.clone());

                    success = true;
                }
            }
        }

        // If no keybinding succeeded but at least one failed with an error,
        // then display that error.
        if !success {
            if let Some(e) = grip_error {
                self.event(AppEvent::StatusError(e));
            }
        }
    }
    fn handle_key_release(&mut self, sc: Option<KeyMappingCode>, vk: Option<VirtualKeyCode>) {
        // Remove grips for this held key.
        self.remove_held_grips(|k| Some(k) == sc.map(Key::Sc) || Some(k) == vk.map(Key::Vk));
    }

    pub(crate) fn resolve_keypress<'a, C>(
        &self,
        keybinds: impl IntoIterator<Item = &'a Keybind<C>>,
        sc: Option<KeyMappingCode>,
        vk: Option<VirtualKeyCode>,
    ) -> Vec<&'a Keybind<C>> {
        let sc = sc.map(Key::Sc);
        let vk = vk.map(Key::Vk);

        let modifiers_mask = self.modifiers_mask(sc, vk);

        keybinds
            .into_iter()
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
    fn modifiers_mask(&self, sc: Option<Key>, vk: Option<Key>) -> ModifiersState {
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
        ignored_keys.fold(
            // Consider all modifiers, but don't distinguish left vs. right.
            ModifiersState::SHIFT
                | ModifiersState::CTRL
                | ModifiersState::ALT
                | ModifiersState::LOGO,
            // Ignore held greps and the key currently being pressed.
            |mods, key_to_ignore| mods & !key_to_ignore.modifier_bit(),
        )
    }

    fn twist_axis_from_name(&self, name: Option<&str>) -> Result<TwistAxis, String> {
        let name = name.ok_or("No twist axis gripped")?;
        self.puzzle
            .ty()
            .twists
            .axis_from_symbol(name)
            .ok_or_else(|| format!("Unknown twist axis {name:?}"))
    }
    fn twist_direction_from_name(&self, name: &str) -> Result<TwistDirection, String> {
        self.puzzle
            .ty()
            .twists
            .direction_from_name(name)
            .ok_or_else(|| format!("Unknown twist direction {name:?}"))
    }

    /// If `preferred` is supplied, returns the twist axis with that name;
    /// otherwise, returns the gripped twist axis if exactly one twist axis is
    /// gripped; otherwise returns `None`.
    pub(crate) fn gripped_twist_axis(&self, preferred: Option<&str>) -> Result<TwistAxis, String> {
        if let Some(name) = preferred {
            return self
                .puzzle
                .ty()
                .twists
                .axis_from_symbol(name)
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
    pub(crate) fn do_recenter(&self, _twist_axis: Option<&str>) -> Result<(), String> {
        // let axis = self.gripped_twist_axis(twist_axis)?;
        // self.event(AppEvent::Twist(self.puzzle.make_recenter_twist(axis)?));
        // TODO: recenter
        Ok(())
    }

    pub(crate) fn pressed_keys(&self) -> &HashSet<Key> {
        &self.pressed_keys
    }
    pub(crate) fn pressed_modifiers(&self) -> ModifiersState {
        self.pressed_modifiers | self.toggled_modifiers
    }
    pub(crate) fn toggle_key(&mut self, sc: Option<KeyMappingCode>, vk: Option<VirtualKeyCode>) {
        let maybe_vk = vk.map(Key::Vk);
        let maybe_sc = sc.map(Key::Sc);

        let mods = maybe_vk.map(|k| k.modifier_bit()).unwrap_or_default()
            | maybe_sc.map(|k| k.modifier_bit()).unwrap_or_default();

        let is_pressed = maybe_vk
            .map(|k| self.toggled_keys.contains(&k))
            .unwrap_or(false)
            || maybe_sc
                .map(|k| self.toggled_keys.contains(&k))
                .unwrap_or(false);

        if is_pressed {
            if let Some(k) = maybe_vk {
                self.toggled_keys.remove(&k);
            }
            if let Some(k) = maybe_sc {
                self.toggled_keys.remove(&k);
            }
            self.toggled_modifiers.remove(mods);
            self.handle_key_release(sc, vk);
        } else {
            if let Some(k) = maybe_vk {
                self.toggled_keys.insert(k);
            }
            if let Some(k) = maybe_sc {
                self.toggled_keys.insert(k);
            }
            self.toggled_modifiers |= mods;
            self.handle_key_press(sc, vk);
        }
    }

    pub(crate) fn frame(&mut self, _delta: Duration) {
        self.puzzle.set_grip(self.grip());

        if self.puzzle.check_just_solved() {
            self.set_status_ok("Solved!");
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

    fn confirm_discard_changes(&mut self, action: &str) -> bool {
        let mut needs_save = self.puzzle.is_unsaved();

        if self.prefs.interaction.confirm_discard_only_when_scrambled
            && !self.puzzle.has_been_fully_scrambled()
        {
            needs_save = false;
        }

        let confirm = !needs_save
            || rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description(&format!("Discard puzzle state and {}?", action))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
        if confirm {
            self.prefs.log_file = None;
            self.prefs.needs_save = true;
        }
        confirm
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
    /// Drag event with a per-frame delta, sent every frame until the drag ends
    /// (even if the delta is zero).
    Drag(egui::Vec2),
    DragReleased,

    Scroll(egui::Vec2),

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
