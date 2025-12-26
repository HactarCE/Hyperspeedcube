use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use float_ord::FloatOrd;
use hyperdraw::GfxEffectParams;
use hypermath::pga::{Axes, Motor};
use hypermath::{Vector, VectorRef};
use hyperprefs::{AnimationPreferences, InterpolateFn};
use hyperpuzzle::Timestamp;
use hyperpuzzle::prelude::*;
use hyperpuzzle_log::{LogEvent, Scramble};
use nd_euclid::{NdEuclidSimState, PartialTwistDragState};
use smallvec::smallvec;
use web_time::{Duration, Instant};

mod nd_euclid;

use super::animations::{AnimationFromState, BlockingAnimationState, TwistAnimationState};
use super::{Action, ReplayEvent, UndoBehavior};
use crate::animations::SpecialAnimationState;

const ASSUMED_FPS: f32 = 120.0;

/// Puzzle simulation, which manages the puzzle state, animations, undo stack,
/// etc.
#[derive(Debug)]
pub struct PuzzleSimulation {
    /// Latest puzzle state, not including any transient rotation.
    latest_state: BoxDynPuzzleState,

    /// Extra state if this is an N-dimensional Euclidean puzzle.
    nd_euclid: Option<Box<NdEuclidSimState>>,

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
    ///
    /// This is `None` if loaded from a non-replay file.
    replay: Option<Vec<ReplayEvent>>,
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
    /// Special animation state.
    special_anim: SpecialAnimationState,

    /// Latest visual piece transforms.
    cached_render_data: BoxDynPuzzleStateRenderData,

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
        let latest_state = puzzle.new_solved_state();
        let cached_render_data = latest_state.render_data();
        Self {
            latest_state,

            nd_euclid: NdEuclidSimState::new(puzzle).map(Box::new),

            scramble_waiting: None,

            scramble: None,
            has_unsaved_changes: false,
            undo_stack: vec![],
            redo_stack: vec![],
            replay: Some(vec![ReplayEvent::StartSession {
                time: Some(Timestamp::now()),
            }]),
            started: false,
            solved: false,
            solved_state_handled: true,
            old_duration: Some(0),
            load_time: Instant::now(),
            is_single_session: true,

            last_frame_time: None,
            twist_anim: TwistAnimationState::default(),
            blocking_anim: BlockingAnimationState::default(),
            special_anim: SpecialAnimationState::default(),

            cached_render_data,

            last_log_file: None,
        }
    }

    /// Returns the latest puzzle state, after all animations have completed.
    pub fn puzzle(&self) -> &dyn PuzzleState {
        &*self.latest_state
    }
    /// Returns the puzzle type.
    pub fn puzzle_type(&self) -> &Arc<Puzzle> {
        self.latest_state.ty()
    }

    /// Returns the latest render data.
    ///
    /// # Panics
    ///
    /// Panics if the render data does not have the expected type.
    pub fn unwrap_render_data<T: PuzzleStateRenderData>(&self) -> &T {
        self.cached_render_data
            .downcast_ref::<T>()
            .expect("unexpected type for PuzzleStateRenderData")
    }
    /// Updates the piece transforms. This is called every frame that the puzzle
    /// is in motion.
    fn update_render_data(&mut self, animation_prefs: &AnimationPreferences) {
        self.cached_render_data = (|| {
            // Twist animation
            if let Some((anim, t)) = self.twist_anim.current() {
                let t = animation_prefs.twist_interpolation.interpolate(t);
                return anim.state.animated_render_data(&anim.anim, t);
            }

            // Partial twist drag
            if let Some(nd_euclid) = &self.nd_euclid {
                if let Some(ret) = nd_euclid.partial_twist_drag_render_state(&self.latest_state) {
                    return ret;
                }
            }

            // No animation
            self.latest_state.render_data()
        })();
    }
    /// Updates the piece transforms, ignoring the current animation state. This
    /// is called after updating the puzzle state with no animation.
    fn skip_twist_animations(&mut self) {
        self.twist_anim = TwistAnimationState::default();
        self.cached_render_data = self.latest_state.render_data();
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
    /// Returns whether the solve has replay events.
    pub fn has_replay(&self) -> bool {
        self.replay.is_some()
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
            hyperpuzzle_log::notation::format_twists(&ty.twists.names, twists),
        );
        self.scramble = Some(scramble.clone());
        // We could use `do_action_internal()` but that would recompute the
        // puzzle state, which isn't necessary.
        let time = Some(Timestamp::now());
        self.undo_stack.push(Action::Scramble { time });
        if let Some(replay_events) = &mut self.replay {
            replay_events.push(ReplayEvent::Scramble { time });
        }
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
        if let Some(replay_events) = &mut self.replay {
            replay_events.push(event.clone());
        }
        match event {
            ReplayEvent::Undo { .. } => self.undo(),
            ReplayEvent::Redo { .. } => self.redo(),
            ReplayEvent::Scramble { time } => {
                self.do_action(Action::Scramble { time });
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
            | ReplayEvent::DragTwist { .. }
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
            Action::Scramble { .. } => match &self.scramble {
                Some(scramble) => {
                    let ty = Arc::clone(self.puzzle_type());
                    for twist in
                        hyperpuzzle_log::notation::parse_twists(&ty.twists.names, &scramble.twists)
                    {
                        match twist {
                            Ok(twist) => match self.latest_state.do_twist_dyn(twist) {
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
            Action::Scramble { .. } => false, // shouldn't be possible
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
    /// This does **not** affect the undo stack. Use [`Self::do_event()`]
    /// instead if that's desired.
    fn do_twist(&mut self, twist: LayeredTwist) -> bool {
        let puzzle = Arc::clone(self.puzzle_type());
        let Ok(twist_info) = puzzle.twists.twists.get(twist.transform) else {
            return false;
        };
        let axis = twist_info.axis;
        let grip = self.latest_state.compute_gripped_pieces(axis, twist.layers);

        let mut nd_euclid_initial_transform = None;
        if let Some(nd_euclid) = &mut self.nd_euclid {
            if let Some(partial) = nd_euclid.partial_twist_drag_state.take() {
                if partial.grip == grip {
                    nd_euclid_initial_transform = Some(partial.transform);
                } else {
                    self.cancel_popped_partial_twist(partial);
                    // That call doesn't modify the puzzle state, so `grip` can
                    // stay the same.
                }
            }
        }

        match self.latest_state.do_twist_dyn(twist) {
            Ok(new_state) => {
                let state = std::mem::replace(&mut self.latest_state, new_state);
                self.blocking_anim.clear();

                if let Some(nd_euclid) = &self.nd_euclid {
                    let geom = &nd_euclid.geom;
                    self.twist_anim.push(AnimationFromState {
                        state,
                        anim: NdEuclidPuzzleAnimation {
                            pieces: grip,
                            initial_transform: nd_euclid_initial_transform
                                .unwrap_or_else(|| Motor::ident(geom.ndim())),
                            final_transform: geom.twist_transforms[twist.transform].clone(),
                        }
                        .into(),
                    });
                }

                true
            }
            Err(blocking_pieces) => {
                self.blocking_anim.set(blocking_pieces);

                if let Some(initial_transform) = nd_euclid_initial_transform {
                    let ndim = initial_transform.ndim();
                    self.twist_anim.push(AnimationFromState {
                        state: self.latest_state.clone_dyn(),
                        anim: NdEuclidPuzzleAnimation {
                            pieces: grip,
                            initial_transform,
                            final_transform: Motor::ident(ndim),
                        }
                        .into(),
                    });
                }

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
            self.update_render_data(animation_prefs);
            needs_redraw = true;
        }
        needs_redraw |= self.blocking_anim.proceed(animation_prefs);
        needs_redraw |= self.special_anim.proceed(delta);

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

    /// Returns the progress for the special animation.
    pub fn special_anim_t(&self) -> Option<f32> {
        self.special_anim
            .get()
            .map(|t| InterpolateFn::Cosine.interpolate(t))
    }

    /// Returns N-dimensional Euclidean simulation state, if applicable.
    pub fn nd_euclid(&self) -> Option<&NdEuclidSimState> {
        self.nd_euclid.as_deref()
    }

    /// Begins a partial twist, which is used for mouse drag twist input.
    pub fn begin_nd_euclid_partial_twist(&mut self, ndim: u8, axis: Axis, layers: LayerMask) {
        let Some(nd_euclid) = &mut self.nd_euclid else {
            return;
        };

        let grip = self.latest_state.compute_gripped_pieces(axis, layers);
        nd_euclid.partial_twist_drag_state = Some(PartialTwistDragState {
            axis,
            layers,
            grip,
            transform: Motor::ident(ndim),
        });
    }
    /// Updates a partial twist with a new cursor position.
    pub fn update_nd_euclid_partial_twist(
        &mut self,
        surface_normal: Vector,
        parallel_drag_delta: Vector,
        animation_prefs: &AnimationPreferences,
    ) {
        let Some(nd_euclid) = &mut self.nd_euclid else {
            return;
        };

        if let Some(partial) = &mut nd_euclid.partial_twist_drag_state {
            let Ok(axis_vector) = nd_euclid.geom.axis_vectors.get(partial.axis) else {
                return;
            };
            let Some(v1) = surface_normal.cross_product_3d(axis_vector).normalize() else {
                return;
            };
            let Some(v2) = axis_vector.cross_product_3d(&v1).normalize() else {
                return;
            };
            // TODO: consider scaling by torque (i.e., radius)
            let angle = v1.dot(&parallel_drag_delta);
            let new_transform = Motor::from_angle_in_normalized_plane(&v2, &v1, angle);
            partial.transform = new_transform;
        }
        self.update_render_data(animation_prefs);
    }
    /// Cancels a partial twist and animates the pieces back.
    ///
    /// If there is no partial twist active, then this does nothing.
    pub fn cancel_partial_twist(&mut self) {
        if let Some(nd_euclid) = &mut self.nd_euclid {
            if let Some(partial) = nd_euclid.partial_twist_drag_state.take() {
                self.cancel_popped_partial_twist(partial);
            }
        }
    }
    /// Cancels a partial twist that has already been removed.
    fn cancel_popped_partial_twist(&mut self, partial: PartialTwistDragState) {
        let ndim = partial.transform.ndim();
        self.twist_anim.push(AnimationFromState {
            state: self.latest_state.clone_dyn(),
            anim: NdEuclidPuzzleAnimation {
                pieces: partial.grip,
                initial_transform: partial.transform,
                final_transform: Motor::ident(ndim),
            }
            .into(),
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
        if let Some(nd_euclid) = &mut self.nd_euclid {
            if let Some(partial) = &nd_euclid.partial_twist_drag_state {
                let puzzle = self.latest_state.ty();

                let closest_twist = nd_euclid
                    .geom
                    .twist_transforms
                    .iter()
                    .filter(|&(twist, _)| puzzle.twists.twists[twist].axis == partial.axis)
                    .max_by_key(|&(_, twist_transform)| {
                        FloatOrd(Motor::dot(&partial.transform, twist_transform).abs())
                    });
                if let Some((twist, twist_transform)) = closest_twist {
                    let dot_with_twist = Motor::dot(&partial.transform, twist_transform).abs();
                    let dot_with_identity = partial.transform.get(Axes::SCALAR).abs();
                    if dot_with_twist > dot_with_identity {
                        let twist = LayeredTwist {
                            layers: partial.layers,
                            transform: twist,
                        };
                        let axis = partial.axis;
                        let time = Some(Timestamp::now());
                        self.do_event(ReplayEvent::DragTwist { time, axis });
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
    }

    /// Returns the special animation state.
    pub fn special_anim(&self) -> &SpecialAnimationState {
        &self.special_anim
    }
    /// Starts the special animation, if it is not already happening.
    pub fn start_special_anim(&mut self) {
        self.special_anim.start();
    }
    /// Returns the special effects to use for drawing for one frame.
    pub fn special_effects(&self) -> GfxEffectParams {
        if let Some(t) = self.special_anim_t() {
            use std::f32::consts::PI;

            let amount = (t * PI).sin();

            GfxEffectParams {
                chromatic_abberation: [amount / 8.0, amount / 12.0],
            }
        } else {
            GfxEffectParams::default()
        }
    }

    /// Returns the combined session time of the file, in milliseconds.
    pub fn file_duration(&self) -> Option<i64> {
        Some(self.old_duration? + self.load_time.elapsed().as_millis() as i64)
    }

    /// Returns a log file as a string.
    pub fn serialize(&self, replay: bool) -> hyperpuzzle_log::Solve {
        let puz = self.puzzle_type();

        let mut log = vec![];
        if replay && let Some(events) = &self.replay {
            for event in events {
                log.push(match event {
                    &ReplayEvent::Undo { time } => LogEvent::Undo { time },
                    &ReplayEvent::Redo { time } => LogEvent::Redo { time },
                    &ReplayEvent::Scramble { time } => LogEvent::Scramble { time },
                    &ReplayEvent::GizmoClick {
                        time,
                        layers,
                        target,
                        reverse,
                    } => LogEvent::Click {
                        time,
                        layers,
                        target: puz.twists.names[target].to_string(),
                        reverse,
                    },
                    &ReplayEvent::DragTwist { time, axis } => LogEvent::DragTwist {
                        time,
                        axis: puz.axes().names[axis].to_string(),
                    },
                    ReplayEvent::Twists(twists) => {
                        let mut s = hyperpuzzle_log::notation::format_twists(
                            &puz.twists.names,
                            twists.iter().copied(),
                        );
                        if twists.len() > 1 {
                            s.insert(0, '(');
                            s.push(')');
                        }
                        LogEvent::Twists(s)
                    }
                    &ReplayEvent::StartSolve { time, duration } => {
                        LogEvent::StartSolve { time, duration }
                    }
                    &ReplayEvent::EndSolve { time, duration } => {
                        LogEvent::EndSolve { time, duration }
                    }
                    &ReplayEvent::StartSession { time } => LogEvent::StartSession { time },
                    &ReplayEvent::EndSession { time } => LogEvent::EndSession { time },
                });
            }
            log.push(LogEvent::EndSession {
                time: Some(Timestamp::now()),
            });
        } else {
            for action in &self.undo_stack {
                match action {
                    &Action::Scramble { time } => log.push(LogEvent::Scramble { time }),
                    Action::Twists(twists) => {
                        if twists.is_empty() {
                            continue;
                        }
                        let mut s = hyperpuzzle_log::notation::format_twists(
                            &puz.twists.names,
                            twists.iter().copied(),
                        );
                        if twists.len() > 1 {
                            s.insert(0, '(');
                            s.push(')');
                        }
                        if let Some(LogEvent::Twists(twists_str)) = log.last_mut() {
                            *twists_str += " ";
                            *twists_str += &s;
                        } else {
                            log.push(LogEvent::Twists(s));
                        }
                    }
                    Action::StartSolve { time, duration } => {
                        log.push(LogEvent::StartSolve {
                            time: *time,
                            duration: *duration,
                        });
                    }
                    Action::EndSolve { time, duration } => {
                        log.push(LogEvent::EndSolve {
                            time: *time,
                            duration: *duration,
                        });
                    }
                }
            }
        }

        hyperpuzzle_log::Solve {
            replay: Some(replay),
            puzzle: hyperpuzzle_log::LogPuzzle {
                id: puz.meta.id.clone(),
                version: puz.meta.version.to_string(),
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
            replay,
            puzzle: _,
            solved: _,
            duration,
            scramble,
            log,
        } = solve;

        log::trace!("Loading file ...");

        let mut ret = Self::new(puzzle);

        let is_replay = replay.unwrap_or(false);
        if !is_replay {
            ret.replay = None;
        }

        for event in log {
            match event {
                &LogEvent::Scramble { time } => {
                    log::trace!("Applying scramble {scramble:?}");
                    ret.reset();
                    if !is_replay {
                        ret.replay = None;
                    }
                    ret.scramble = scramble.clone();
                    ret.replay_event(ReplayEvent::Scramble { time });
                }
                &LogEvent::Click {
                    time,
                    layers,
                    ref target,
                    reverse,
                } => {
                    // TODO: handle errors
                    let Some(target) = puzzle.twists.names.id_from_name(target) else {
                        continue;
                    };
                    ret.replay_event(ReplayEvent::GizmoClick {
                        time,
                        layers,
                        target,
                        reverse,
                    });
                }
                &LogEvent::DragTwist { time, ref axis } => {
                    // TODO: handle errors
                    let Some(axis) = puzzle.axes().names.id_from_name(axis) else {
                        continue;
                    };
                    ret.replay_event(ReplayEvent::DragTwist { time, axis });
                }
                LogEvent::Twists(twists_str) => {
                    for group in hyperpuzzle_log::notation::parse_grouped_twists(
                        &puzzle.twists.names,
                        twists_str,
                    ) {
                        // TODO: handle errors
                        let group = group.into_iter().filter_map(Result::ok).collect();
                        log::trace!("Applying twist group {group:?} from {twists_str:?}");
                        ret.replay_event(ReplayEvent::Twists(group));
                    }
                }
                &LogEvent::Undo { time } => ret.replay_event(ReplayEvent::Undo { time }),
                &LogEvent::Redo { time } => ret.replay_event(ReplayEvent::Redo { time }),
                LogEvent::StartSolve { time, duration } => {
                    ret.started = true;
                    ret.replay_event(ReplayEvent::StartSolve {
                        time: *time,
                        duration: *duration,
                    });
                }
                LogEvent::EndSolve { time, duration } => {
                    ret.solved = true;
                    ret.solved_state_handled = true;
                    ret.replay_event(ReplayEvent::EndSolve {
                        time: *time,
                        duration: *duration,
                    });
                }
                LogEvent::StartSession { time } => {
                    ret.replay_event(ReplayEvent::StartSession { time: *time });
                }
                LogEvent::EndSession { time } => {
                    ret.replay_event(ReplayEvent::EndSession { time: *time });
                }
            }
        }

        if is_replay {
            ret.replay_event(ReplayEvent::StartSession {
                time: Some(Timestamp::now()),
            });
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
