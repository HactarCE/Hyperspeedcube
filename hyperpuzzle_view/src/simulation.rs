use std::path::PathBuf;
use std::sync::{mpsc, Arc};

use float_ord::FloatOrd;
use hypermath::pga::{Axes, Motor};
use hypermath::{Vector, VectorRef};
use hyperprefs::AnimationPreferences;
use hyperpuzzle::{
    Axis, LayerMask, LayeredTwist, PerPiece, PieceMask, Puzzle, PuzzleState, ScrambleParams,
    ScrambleProgress, ScrambleType, ScrambledPuzzle, Timestamp,
};
use hyperpuzzle_log::Scramble;
use smallvec::smallvec;
use web_time::{Duration, Instant};

use super::animations::{BlockingAnimationState, TwistAnimation, TwistAnimationState};
use super::{Action, ReplayEvent, UndoBehavior};

const ASSUMED_FPS: f32 = 120.0;

/// Puzzle simulation, which manages the puzzle state, animations, undo stack,
/// etc.
#[derive(Debug)]
pub struct PuzzleSimulation {
    /// Latest puzzle state, not including any transient rotation.
    latest_state: PuzzleState,

    scramble_waiting: Option<(
        Arc<ScrambleProgress>,
        mpsc::Receiver<Option<ScrambledPuzzle>>,
    )>,

    /// Scramble applied to the puzzle initially.
    scramble: Option<Scramble>,
    /// Whether the puzzle has unsaved changes.
    has_unsaved_changes: bool,
    /// Stack of actions to undo.
    undo_stack: Vec<Action>,
    /// Stack of actions to redo.
    redo_stack: Vec<Action>,
    /// List of actions to save in the replay file.
    replay: Vec<ReplayEvent>,
    /// Whether the solve has been started.
    started: bool,
    /// Whether the puzzle has been solved.
    solved: bool,
    /// Whether the solved state has been handled by the UI.
    solved_state_handled: bool,
    /// Total duration from previous sessions.
    old_duration: Option<i64>,
    /// Time that the puzzle was loaded.
    load_time: Instant,
    /// Whether the solve is single-session.
    is_single_session: bool,

    /// Time of last frame, or `None` if we are not in the middle of an
    /// animation.
    last_frame_time: Option<Instant>,
    /// Twist animation state.
    twist_anim: TwistAnimationState,
    /// Blocking pieces animation state.
    blocking_anim: BlockingAnimationState,

    /// Twist drag state.
    partial_twist_drag_state: Option<PartialTwistDragState>,

    /// Latest visual piece transforms.
    cached_piece_transforms: PerPiece<Motor>,

    /// Last loaded/saved log file.
    pub last_log_file: Option<PathBuf>,
}
impl Drop for PuzzleSimulation {
    fn drop(&mut self) {
        if let Some((progress, _)) = &self.scramble_waiting {
            progress.request_cancel();
        }
    }
}
impl PuzzleSimulation {
    /// Constructs a new simulation with a fresh puzzle state.
    pub fn new(puzzle: &Arc<Puzzle>) -> Self {
        let latest_state = PuzzleState::new(Arc::clone(puzzle));
        let cached_piece_transforms = latest_state.piece_transforms();
        Self {
            latest_state,

            scramble_waiting: None,

            scramble: None,
            has_unsaved_changes: false,
            undo_stack: vec![],
            redo_stack: vec![],
            replay: vec![],
            started: false,
            solved: false,
            solved_state_handled: true,
            old_duration: Some(0),
            load_time: Instant::now(),
            is_single_session: true,

            last_frame_time: None,
            twist_anim: TwistAnimationState::default(),
            blocking_anim: BlockingAnimationState::default(),

            partial_twist_drag_state: None,

            cached_piece_transforms,

            last_log_file: None,
        }
    }

    /// Returns the latest puzzle state, after all animations have completed.
    pub fn puzzle(&self) -> &PuzzleState {
        &self.latest_state
    }
    /// Returns the puzzle type.
    pub fn puzzle_type(&self) -> &Arc<Puzzle> {
        self.latest_state.ty()
    }
    /// Returns the number of dimensions of the puzzle view.
    pub fn ndim(&self) -> u8 {
        self.puzzle_type().ndim()
    }

    /// Returns the latest piece transforms.
    pub fn piece_transforms(&self) -> &PerPiece<Motor> {
        &self.cached_piece_transforms
    }
    /// Updates the piece transforms. This is called every frame that the puzzle
    /// is in motion.
    fn update_piece_transforms(&mut self, animation_prefs: &AnimationPreferences) {
        self.cached_piece_transforms = self
            .twist_anim
            .current()
            .map(|(anim, t)| {
                let t = animation_prefs.twist_interpolation.interpolate(t);
                let start = &anim.initial_transform;
                let end = &anim.final_transform;
                let m = Motor::slerp_infallible(start, end, t as _);
                anim.state.partial_piece_transforms(&anim.grip, &m)
            })
            .or_else(|| {
                self.partial_twist_drag_state.as_ref().map(|partial| {
                    self.latest_state
                        .partial_piece_transforms(&partial.grip, &partial.transform)
                })
            })
            .unwrap_or_else(|| self.latest_state.piece_transforms());
    }
    /// Updates the piece transforms, ignoring the current animation state. This
    /// is called after updating the puzzle state with no animation.
    fn skip_twist_animations(&mut self) {
        self.twist_anim = TwistAnimationState::default();
        self.cached_piece_transforms = self.latest_state.piece_transforms();
    }

    /// Returns whether the puzzle has unsaved changes.
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }
    /// Mark the puzzle as having no unsaved changes.
    pub fn clear_unsaved_changes(&mut self) {
        self.has_unsaved_changes = false;
    }

    /// Returns the scramble, or `None` if the puzzle has not been scrambled.
    ///
    /// To scramble the puzzle, call [`Self::scramble()`].
    pub fn get_scramble(&self) -> &Option<Scramble> {
        &self.scramble
    }
    /// Returns whether the puzzle has been fully scrambled.
    pub fn has_been_fully_scrambled(&self) -> bool {
        // TODO: only if "scramble" actually appears in the log file
        self.scramble
            .as_ref()
            .is_some_and(|scramble| scramble.ty == ScrambleType::Full)
    }

    /// Resets the puzzle state and replay log.
    pub fn reset(&mut self) {
        *self = Self::new(self.puzzle_type());
    }
    /// Resets and scrambles the puzzle.
    pub fn scramble(&mut self, params: ScrambleParams) {
        let ty = Arc::clone(self.puzzle_type());
        let progress = Arc::new(ScrambleProgress::new());
        let (tx, rx) = mpsc::channel();
        self.scramble_waiting = Some((Arc::clone(&progress), rx));
        std::thread::spawn(move || {
            // ignore channel error
            let _ = tx.send(ty.new_scrambled_with_progress(params, Some(progress)));
        });
    }
    /// Returns progress on scrambling the puzzle.
    pub fn scramble_progress(&mut self) -> Option<Arc<ScrambleProgress>> {
        let (progress, rx) = self.scramble_waiting.as_ref()?;
        match rx.try_recv() {
            Err(mpsc::TryRecvError::Empty) => Some(Arc::clone(progress)), // still waiting
            Err(mpsc::TryRecvError::Disconnected) | Ok(None) => {
                log::error!("error scrambling puzzle");
                self.scramble = None;
                None
            }
            Ok(Some(scrambled)) => {
                self.recv_scramble(scrambled);
                None
            }
        }
    }
    fn recv_scramble(&mut self, scrambled: ScrambledPuzzle) {
        let ScrambledPuzzle {
            params,
            twists,
            state,
        } = scrambled;

        self.reset();
        let ty = self.puzzle_type();
        let scramble = Scramble::new(
            params,
            hyperpuzzle_log::notation::format_twists(&ty.twists, twists),
        );
        self.scramble = Some(scramble.clone());
        self.undo_stack.push(Action::Scramble);
        self.replay.push(ReplayEvent::Scramble);
        self.latest_state = state;
        self.skip_twist_animations();
        self.solved_state_handled = false;
    }
    /// Plays a replay event on the puzzle.
    pub fn do_event(&mut self, event: ReplayEvent) {
        if matches!(event, ReplayEvent::Twists(_)) && self.scramble.is_some() && !self.started {
            self.do_event(ReplayEvent::StartSolve {
                time: Some(Timestamp::now()),
                duration: self.file_duration(),
            });
        }
        self.replay_event(event);
    }
    /// Plays a replay event on the puzzle when deserializing.
    fn replay_event(&mut self, event: ReplayEvent) {
        self.replay.push(event.clone());
        match event {
            ReplayEvent::Undo => self.undo(),
            ReplayEvent::Redo => self.redo(),
            ReplayEvent::Scramble => {
                self.do_action(Action::Scramble);
            }
            ReplayEvent::Twists(twists) => {
                self.do_action(Action::Twists(twists));
            }
            ReplayEvent::StartSolve { time, duration } => {
                self.do_action(Action::StartSolve { time, duration });
            }
            ReplayEvent::EndSolve { time, duration } => {
                self.do_action(Action::EndSolve { time, duration });
            }
            ReplayEvent::GizmoClick { .. }
            | ReplayEvent::DragTwist
            | ReplayEvent::StartSession { .. }
            | ReplayEvent::EndSession { .. } => (),
        }
    }

    /// Returns whether there is an action available to undo.
    pub fn has_undo(&self) -> bool {
        for action in self.undo_stack.iter().rev() {
            match action.undo_behavior() {
                UndoBehavior::Action => return true,
                UndoBehavior::Marker => continue, // find the next action
                UndoBehavior::Boundary => return false, // cannot undo
            }
        }
        false
    }
    /// Returns whether there is an action available to redo.
    pub fn has_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    fn undo(&mut self) {
        // Keep undoing until we find an action that can be undone.
        while let Some(action) = self.undo_stack.pop() {
            match action.undo_behavior() {
                UndoBehavior::Action => {
                    if self.undo_action_internal(&action) {
                        self.redo_stack.push(action);
                    }
                    break;
                }
                UndoBehavior::Marker => {
                    if self.undo_action_internal(&action) {
                        self.redo_stack.push(action);
                    }
                }
                UndoBehavior::Boundary => {
                    self.undo_stack.push(action); // oops, put it back!
                    break;
                }
            }
        }
    }
    fn redo(&mut self) {
        // Keep redoing until we find an action that can be redone.
        while let Some(action) = self.redo_stack.pop() {
            if self.do_action_internal(&action) {
                self.undo_stack.push(action);
                break;
            }
        }
    }

    /// Does an undoable action and saves it to the undo stack.
    ///
    /// Clears the redo stack if applicable.
    fn do_action(&mut self, action: Action) {
        self.has_unsaved_changes = true;
        match action.undo_behavior() {
            UndoBehavior::Action => self.redo_stack.clear(),
            UndoBehavior::Marker | UndoBehavior::Boundary => (),
        }
        self.undo_stack.push(action.clone());
        self.do_action_internal(&action);
    }
    /// Does an undoable action. Returns whether the action should be saved to
    /// the undo stack.
    fn do_action_internal(&mut self, action: &Action) -> bool {
        self.has_unsaved_changes = true;
        match action {
            Action::Scramble => match &self.scramble {
                Some(scramble) => {
                    let ty = Arc::clone(self.puzzle_type());
                    for twist in
                        hyperpuzzle_log::notation::parse_twists(&ty.twist_by_name, &scramble.twists)
                    {
                        match twist {
                            Ok(twist) => match self.latest_state.do_twist(twist) {
                                Ok(new_state) => self.latest_state = new_state,
                                Err(e) => {
                                    log::error!(
                                        "twist {twist:?} blocked in scramble due to pieces {e:?}"
                                    );
                                }
                            },
                            Err(e) => log::error!("error parsing twist in scramble: {e}"),
                        }
                    }
                    self.skip_twist_animations();
                    true
                }
                None => false,
            },
            Action::Twists(twists) => {
                let mut any_effect = false;
                for &twist in twists {
                    any_effect |= self.do_twist(twist);
                }
                if any_effect && !self.solved && self.scramble.is_some() && self.is_solved() {
                    self.do_event(ReplayEvent::EndSolve {
                        time: Some(Timestamp::now()),
                        duration: self.file_duration(),
                    });
                }
                any_effect
            }
            Action::StartSolve { .. } => {
                self.started = true;
                true
            }
            Action::EndSolve { .. } => {
                self.solved = true;
                true
            }
        }
    }
    /// Undoes an action. Returns whether the action should be saved to the redo
    /// stack.
    fn undo_action_internal(&mut self, action: &Action) -> bool {
        self.has_unsaved_changes = true;
        let puz = self.puzzle_type();
        match action {
            Action::Scramble => false, // shouldn't be possible
            Action::Twists(twists) => self.do_action_internal(&Action::Twists(
                twists
                    .iter()
                    .rev()
                    .filter_map(|twist| twist.rev(puz).ok())
                    .collect(),
            )),
            Action::StartSolve { .. } => false, // shouldn't be possible
            Action::EndSolve { .. } => {
                self.solved = false;
                true
            }
        }
    }

    /// Executes a twist on the puzzle and queues the appropriate animation.
    /// Returns whether the twist was successful.
    ///
    /// Any in-progress partial twist is canceled.
    ///
    /// This does **not** affect the undo stack. Use [`Self::event()`] instead
    /// if that's desired.
    fn do_twist(&mut self, twist: LayeredTwist) -> bool {
        let puzzle = Arc::clone(self.puzzle_type());
        let Ok(twist_info) = puzzle.twists.get(twist.transform) else {
            return false;
        };
        let axis = twist_info.axis;
        let grip = self.latest_state.compute_gripped_pieces(axis, twist.layers);

        let mut initial_transform = Motor::ident(self.ndim());
        if let Some(partial) = self.partial_twist_drag_state.take() {
            if partial.grip == grip {
                initial_transform = partial.transform;
            } else {
                self.cancel_popped_partial_twist(partial);
                // That call doesn't modify the puzzle state, so `grip` can stay
                // the same.
            }
        }

        match self.latest_state.do_twist(twist) {
            Ok(new_state) => {
                let state = std::mem::replace(&mut self.latest_state, new_state);
                self.twist_anim.push(TwistAnimation {
                    state,
                    grip,
                    initial_transform,
                    final_transform: twist_info.transform.clone(),
                });
                self.blocking_anim.clear();
                true
            }
            Err(blocking_pieces) => {
                if !initial_transform.is_ident() {
                    self.twist_anim.push(TwistAnimation {
                        state: self.latest_state.clone(),
                        grip,
                        initial_transform,
                        final_transform: Motor::ident(self.ndim()),
                    });
                }
                self.blocking_anim.set(blocking_pieces);
                false
            }
        }
    }

    /// Advances the puzzle geometry and internal state to the next frame, using
    /// the given time delta between this frame and the last. Returns whether
    /// the puzzle must be redrawn.
    pub fn step(&mut self, animation_prefs: &AnimationPreferences) -> bool {
        let now = Instant::now();
        let delta = match self.last_frame_time {
            Some(then) => now - then,
            None => Duration::from_secs_f32(1.0 / ASSUMED_FPS),
        };

        let mut needs_redraw = false;

        // TODO: animate view angle offset
        // // Animate view angle offset.
        // if !self.view_angle.is_frozen {
        //     let offset = &mut self.view_angle.current;

        //     let decay_multiplier =
        // VIEW_ANGLE_OFFSET_DECAY_RATE.powf(delta.as_secs_f32());
        //     let new_offset = Quaternion::one().slerp(*offset, decay_multiplier);
        //     if offset.s == new_offset.s {
        //         // Stop the animation once we're not making any more progress.
        //         *offset = Quaternion::one();
        //     } else {
        //         *offset = new_offset;
        //     }
        // }

        if self.twist_anim.proceed(delta, animation_prefs) {
            self.update_piece_transforms(animation_prefs);
            needs_redraw = true;
        }
        needs_redraw |= self.blocking_anim.proceed(animation_prefs);

        if needs_redraw {
            self.last_frame_time = Some(now);
        } else {
            self.last_frame_time = None;
        }

        needs_redraw
    }

    /// Returns the state of the blocking pieces animation.
    pub fn blocking_pieces_anim(&self) -> &BlockingAnimationState {
        &self.blocking_anim
    }

    /// Returns the currently active partial twist.
    pub fn partial_twist(&self) -> &Option<PartialTwistDragState> {
        &self.partial_twist_drag_state
    }
    /// Begins a partial twist, which is used for mouse drag twist input.
    pub fn begin_partial_twist(&mut self, axis: Axis, layers: LayerMask) {
        let grip = self.latest_state.compute_gripped_pieces(axis, layers);
        self.partial_twist_drag_state = Some(PartialTwistDragState {
            axis,
            layers,
            grip,
            transform: Motor::ident(self.ndim()),
        });
    }
    /// Updates a partial twist with a new cursor position.
    pub fn update_partial_twist(
        &mut self,
        surface_normal: Vector,
        parallel_drag_delta: Vector,
        animation_prefs: &AnimationPreferences,
    ) {
        let puzzle = Arc::clone(self.puzzle_type());
        if let Some(partial) = &mut self.partial_twist_drag_state {
            let Ok(axis) = puzzle.axes.get(partial.axis) else {
                return;
            };
            let Some(v1) = surface_normal.cross_product_3d(&axis.vector).normalize() else {
                return;
            };
            let Some(v2) = axis.vector.cross_product_3d(&v1).normalize() else {
                return;
            };
            // TODO: consider scaling by torque (i.e., radius)
            let angle = v1.dot(&parallel_drag_delta);
            let new_transform = Motor::from_angle_in_normalized_plane(3, &v2, &v1, angle);
            partial.transform = new_transform;
        }
        self.update_piece_transforms(animation_prefs);
    }
    /// Cancels a partial twist and animates the pieces back.
    ///
    /// If there is no partial twist active, then this does nothing.
    pub fn cancel_partial_twist(&mut self) {
        if let Some(partial) = self.partial_twist_drag_state.take() {
            self.cancel_popped_partial_twist(partial);
        }
    }
    /// Cancels a partial twist that has already been removed.
    fn cancel_popped_partial_twist(&mut self, partial: PartialTwistDragState) {
        self.twist_anim.push(TwistAnimation {
            state: self.latest_state.clone(),
            grip: partial.grip,
            initial_transform: partial.transform,
            final_transform: Motor::ident(self.ndim()),
        });
    }
    /// Returns whether there's a twist animation queued or animating currently.
    pub fn has_twist_anim_queued(&self) -> bool {
        self.twist_anim.current().is_some()
    }
    /// Confirms a partial twist, completing whatever move is closest (or
    /// canceling it, if the identity twist is closest).
    ///
    /// If there is no partial twist active, then this does nothing.
    pub fn confirm_partial_twist(&mut self) {
        if let Some(partial) = &self.partial_twist_drag_state {
            let closest_twist = self
                .puzzle_type()
                .twists
                .iter()
                .filter(|(_, twist_info)| twist_info.axis == partial.axis)
                .max_by_key(|(_, twist_info)| {
                    FloatOrd(Motor::dot(&partial.transform, &twist_info.transform).abs())
                });
            if let Some((twist, twist_info)) = closest_twist {
                let dot_with_twist = Motor::dot(&partial.transform, &twist_info.transform).abs();
                let dot_with_identity = partial.transform.get(Axes::SCALAR).abs();
                if dot_with_twist > dot_with_identity {
                    let twist = LayeredTwist {
                        layers: partial.layers,
                        transform: twist,
                    };
                    self.do_event(ReplayEvent::DragTwist);
                    self.do_event(ReplayEvent::Twists(smallvec![twist]));
                } else {
                    // The identity twist is closer.
                    self.cancel_partial_twist();
                }
            } else {
                // There are no possible twists. Why did we even let the user
                // drag the mouse in the first place? Questions such as these
                // may never know an answer.
                self.cancel_partial_twist();
            }
        }
    }

    /// Returns the combined session time of the file, in milliseconds.
    pub fn file_duration(&self) -> Option<i64> {
        Some(self.old_duration? + self.load_time.elapsed().as_millis() as i64)
    }

    /// Returns a log file as a string.
    pub fn serialize(&self) -> hyperpuzzle_log::Solve {
        let puz = self.puzzle_type();

        let mut log = vec![];
        for action in &self.undo_stack {
            match action {
                Action::Scramble => log.push(hyperpuzzle_log::LogEvent::Scramble),
                Action::Twists(twists) => {
                    if twists.is_empty() {
                        continue;
                    }
                    let mut s = hyperpuzzle_log::notation::format_twists(
                        &puz.twists,
                        twists.iter().copied(),
                    );
                    if twists.len() > 1 {
                        s.insert(0, '(');
                        s.push(')');
                    }
                    if let Some(hyperpuzzle_log::LogEvent::Twists(twists_str)) = log.last_mut() {
                        *twists_str += " ";
                        *twists_str += &s;
                    } else {
                        log.push(hyperpuzzle_log::LogEvent::Twists(s));
                    }
                }
                Action::StartSolve { time, duration } => {
                    log.push(hyperpuzzle_log::LogEvent::StartSolve {
                        time: *time,
                        duration: *duration,
                    });
                }
                Action::EndSolve { time, duration } => {
                    log.push(hyperpuzzle_log::LogEvent::EndSolve {
                        time: *time,
                        duration: *duration,
                    });
                }
            }
        }

        hyperpuzzle_log::Solve {
            puzzle: hyperpuzzle_log::Puzzle {
                id: puz.id.clone(),
                version: puz.version.to_string(),
            },
            solved: self
                .undo_stack
                .iter()
                .any(|event| matches!(event, Action::EndSolve { .. })),
            duration: self.file_duration(),
            scramble: self.scramble.clone(),
            log,
        }
    }
    /// Loads a log file from a string.
    pub fn deserialize(puzzle: &Arc<Puzzle>, solve: &hyperpuzzle_log::Solve) -> Self {
        let hyperpuzzle_log::Solve {
            puzzle: _,
            solved: _,
            duration,
            scramble,
            log,
        } = solve;

        log::trace!("Loading file ...");

        let mut ret = Self::new(puzzle);
        for event in log {
            match event {
                hyperpuzzle_log::LogEvent::Scramble => {
                    log::trace!("Applying scramble {scramble:?}");
                    ret.reset();
                    ret.scramble = scramble.clone();
                    ret.replay_event(ReplayEvent::Scramble);
                }
                hyperpuzzle_log::LogEvent::Click {
                    layers,
                    target,
                    reverse,
                } => {
                    // TODO: handle errors
                    let Some(target) = puzzle.twist_by_name.get(target) else {
                        continue;
                    };
                    ret.replay_event(ReplayEvent::GizmoClick {
                        layers: *layers,
                        target: *target,
                        reverse: *reverse,
                    });
                }
                hyperpuzzle_log::LogEvent::Twists(twists_str) => {
                    for group in hyperpuzzle_log::notation::parse_grouped_twists(
                        &puzzle.twist_by_name,
                        twists_str,
                    ) {
                        // TODO: handle errors
                        let group = group.into_iter().filter_map(Result::ok).collect();
                        log::trace!("Applying twist group {group:?} from {twists_str:?}");
                        ret.replay_event(ReplayEvent::Twists(group));
                    }
                }
                hyperpuzzle_log::LogEvent::StartSolve { time, duration } => {
                    ret.started = true;
                    ret.replay_event(ReplayEvent::StartSolve {
                        time: *time,
                        duration: *duration,
                    });
                }
                hyperpuzzle_log::LogEvent::EndSolve { time, duration } => {
                    ret.solved = true;
                    ret.solved_state_handled = true;
                    ret.replay_event(ReplayEvent::EndSolve {
                        time: *time,
                        duration: *duration,
                    });
                }
                hyperpuzzle_log::LogEvent::StartSession { time } => {
                    ret.replay_event(ReplayEvent::StartSession { time: *time });
                }
                hyperpuzzle_log::LogEvent::EndSession { time } => {
                    ret.replay_event(ReplayEvent::EndSession { time: *time });
                }
            }
        }
        ret.old_duration = *duration;
        ret.has_unsaved_changes = false;
        ret.is_single_session = false;

        ret.skip_twist_animations();

        ret
    }

    /// Returns whether the puzzle is _currently_ solved.
    pub fn is_solved(&self) -> bool {
        self.latest_state.is_solved()
    }
    /// Returns whether the puzzle was _just_ solved.
    ///
    /// This returns `true` at most once until the simulation is recreated.
    pub fn handle_newly_solved_state(&mut self) -> bool {
        self.solved && !std::mem::replace(&mut self.solved_state_handled, true)
    }
}

#[derive(Debug, Clone)]
pub struct PartialTwistDragState {
    pub axis: Axis,
    pub layers: LayerMask,
    pub grip: PieceMask,
    pub transform: Motor,
}
