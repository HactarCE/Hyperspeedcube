use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, mpsc};

use float_ord::FloatOrd;
use hyperdraw::GfxEffectParams;
use hypermath::pga::Motor;
use hypermath::{Vector, VectorRef};
use hyperprefs::{AnimationPreferences, InterpolateFn};
use hyperpuzzle::Timestamp;
use hyperpuzzle::prelude::*;
use hyperpuzzle::symmetric::SymmetricTwistSystemEngineData;
use hyperpuzzle_log::{LogEvent, Scramble};
use hypuz_notation::Invert;
use itertools::Itertools;
use nd_euclid::{NdEuclidSimState, PartialTwistDragState};
use smallvec::smallvec;
use web_time::{Duration, Instant};

mod nd_euclid;

use super::animations::{AnimationFromState, BlockingAnimationState, TwistAnimationState};
use super::{Action, ReplayEvent, UndoBehavior};
use crate::animations::SpecialAnimationState;

const ASSUMED_FPS: f32 = 120.0;

static NEXT_SIM_ID: AtomicUsize = AtomicUsize::new(1);

/// Puzzle simulation, which manages the puzzle state, animations, undo stack,
/// etc.
#[derive(Debug)]
pub struct PuzzleSimulation {
    /// Latest puzzle state, not including any transient rotation.
    latest_state: BoxDynPuzzleState,

    /// Extra state if this is an N-dimensional Euclidean puzzle.
    nd_euclid: Option<Box<NdEuclidSimState>>,

    scramble_waiting: Option<(
        ScrambleType,
        Arc<ScrambleProgress>,
        mpsc::Receiver<Result<ScrambledPuzzle, ScrambleError>>,
    )>,
    scramble_error: Option<(ScrambleType, ScrambleError)>,

    /// Move counter.
    stm_counter: StmCounter,
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
    /// Whether the puzzle has been solved from a scramble.
    solved: bool,
    /// Whether the simulation has been reloaded since the puzzle was solved.
    has_been_reloaded_since_first_solved: bool,
    /// Total duration from previous sessions.
    old_duration: Option<i64>,
    /// Time that the puzzle was loaded.
    load_time: Instant,
    /// Whether the solve is single-session.
    is_single_session: bool,
    /// Whether the puzzle state is concealed.
    pub blindfolded: bool,
    /// Whether some action has been done that invalidates a filterless solve.
    invalidated_filterless: bool,
    /// Original color scheme used for the puzzle.
    ///
    /// This is used to detect color scheme changes which would invalidate a
    /// filterless solve.
    original_colors: Option<Vec<[u8; 3]>>,

    /// Time of the start of inspection.
    inspection_start_time: Option<Instant>,
    /// Time of the first move.
    first_move_time: Option<Instant>,
    /// Time of the completion of the solve.
    solve_complete_time: Option<Instant>,
    /// Whether the solve summary should be shown on the next frame.
    ///
    /// This is only ever `true` for one frame at a time, either when the puzzle
    /// is first solved or when requested by the user.
    request_to_show_solve_summary: bool,

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

    /// Timestamp signature, if any.
    pub tsa_signature_v3: Option<String>,
    /// Whether the solve has been saved to the autonamed file after completion.
    ///
    /// This field is only intended for use by downstream users of the crate; it
    /// is not modified or read by any code in this crate.
    pub saved_to_autonamed_file: bool,
    /// URL of the submission, if the solve has been uploaded to the
    /// leaderboards.
    ///
    /// This field is only intended for use by downstream users of the crate; it
    /// is not modified or read by any code in this crate.
    pub leaderboard_url: Option<String>,

    /// Puzzle simulation ID, unique to the process. This is used to reset
    /// certain cosmetic settings (such as filters).
    pub sim_id: usize,
}
impl Drop for PuzzleSimulation {
    fn drop(&mut self) {
        if let Some((_, progress, _)) = &self.scramble_waiting {
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
            scramble_error: None,

            stm_counter: StmCounter::new(),
            scramble: None,
            has_unsaved_changes: false,
            undo_stack: vec![],
            redo_stack: vec![],
            replay: Some(vec![ReplayEvent::StartSession {
                time: Some(Timestamp::now()),
            }]),
            started: false,
            solved: false,
            has_been_reloaded_since_first_solved: false,
            old_duration: Some(0),
            load_time: Instant::now(),
            is_single_session: true,
            blindfolded: false,
            invalidated_filterless: false,
            original_colors: None,

            inspection_start_time: None,
            first_move_time: None,
            solve_complete_time: None,
            request_to_show_solve_summary: false,

            last_frame_time: None,
            twist_anim: TwistAnimationState::default(),
            blocking_anim: BlockingAnimationState::default(),
            special_anim: SpecialAnimationState::default(),

            cached_render_data,

            last_log_file: None,

            tsa_signature_v3: None,
            saved_to_autonamed_file: false,
            leaderboard_url: None,

            sim_id: NEXT_SIM_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
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
            if let Some(nd_euclid) = &self.nd_euclid
                && let Some(ret) = nd_euclid.partial_twist_drag_render_state(&self.latest_state)
            {
                return ret;
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
    /// Marks the puzzle as having no unsaved changes.
    pub fn clear_unsaved_changes(&mut self) {
        self.has_unsaved_changes = false;
    }
    /// Marks the puzzle as having unsaved changes.
    pub fn mark_unsaved(&mut self) {
        self.has_unsaved_changes = true;
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
    pub fn scramble(&mut self, ty: ScrambleType, online: bool) {
        if let Some((_, progress, _)) = &self.scramble_waiting {
            progress.request_cancel();
        }

        let puzzle_type = Arc::clone(self.puzzle_type());
        let progress = Arc::new(ScrambleProgress::new());
        let (tx, rx) = mpsc::channel();
        self.scramble_waiting = Some((ty, Arc::clone(&progress), rx));
        self.scramble_error = None;
        std::thread::spawn(move || {
            let params = if online && ty == ScrambleType::Full {
                ScrambleParams::from_randomness_beacon(ty)
            } else {
                Ok(ScrambleParams::new(ty))
            };
            let scrambled_puzzle = params
                .and_then(|params| puzzle_type.new_scrambled_with_progress(params, Some(progress)));
            let _ = tx.send(scrambled_puzzle); // ignore channel error
        });
    }
    /// Returns progress on scrambling the puzzle.
    pub fn scramble_progress(&mut self) -> Option<Arc<ScrambleProgress>> {
        let (ty, progress, rx) = self.scramble_waiting.take()?;
        match rx.try_recv() {
            Err(mpsc::TryRecvError::Empty) => {
                self.scramble_waiting = Some((ty, Arc::clone(&progress), rx));
                Some(progress) // still waiting
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.scramble_error =
                    Some((ty, ScrambleError::Other("channel disconnected".to_string())));
                self.scramble = None;
                None
            }
            Ok(Err(e)) => {
                self.scramble_error = Some((ty, e));
                self.scramble = None;
                None
            }
            Ok(Ok(scrambled)) => {
                self.recv_scramble(scrambled);
                None
            }
        }
    }
    /// Returns an error encountered while scrambling the puzzle, if there is
    /// one.
    pub fn scramble_error(&self) -> &Option<(ScrambleType, ScrambleError)> {
        &self.scramble_error
    }
    fn recv_scramble(&mut self, scrambled: ScrambledPuzzle) {
        let ScrambledPuzzle {
            params,
            twists,
            state,
        } = scrambled;

        self.reset();
        let scramble = Scramble::new(
            params,
            twists.into_iter().map(|mv| mv.to_string()).join(" "),
        );
        self.scramble = Some(scramble.clone());
        self.scramble_error = None;
        // We could use `do_action_internal()` but that would recompute the
        // puzzle state, which isn't necessary.
        self.undo_stack.push(Action::Scramble {
            time: scramble.time,
        });
        self.replay = Some(vec![
            ReplayEvent::Scramble {
                time: scramble.time,
            },
            ReplayEvent::StartSession {
                time: Some(Timestamp::now()),
            },
        ]);
        self.latest_state = state;
        self.skip_twist_animations();
        self.inspection_start_time = Some(Instant::now());
        self.first_move_time = None;
        self.solve_complete_time = None;
    }
    /// Plays a replay event on the puzzle.
    pub fn do_event(&mut self, event: ReplayEvent) {
        if matches!(event, ReplayEvent::Twists(_)) && self.scramble.is_some() && !self.started {
            self.do_event(ReplayEvent::StartSolve {
                time: Some(Timestamp::now()),
                duration: self.file_duration(),
            });
        }
        self.replay_event(event, false);
    }
    /// Plays a replay event on the puzzle when deserializing.
    fn replay_event(&mut self, event: ReplayEvent, is_replaying: bool) {
        let is_no_op = match event {
            ReplayEvent::Undo { .. } => !self.has_undo(),
            ReplayEvent::Redo { .. } => !self.has_redo(),
            ReplayEvent::InvalidateFilterless { .. } => self.invalidated_filterless,
            _ => false,
        };
        if is_no_op {
            return; // Do not push to replay events
        }

        if let Some(replay_events) = &mut self.replay {
            if event.is_cosmetic() {
                // Remove previous similar event
                let mut recent_cosmetic_events = (0..replay_events.len())
                    .rev()
                    .take_while(|&i| replay_events[i].is_cosmetic());
                if let Some(i) = recent_cosmetic_events.find(|&i| {
                    std::mem::discriminant(&replay_events[i]) == std::mem::discriminant(&event)
                }) {
                    replay_events.remove(i);
                }
            }

            replay_events.push(event.clone());
        }

        match event {
            ReplayEvent::Undo { .. } => self.undo(is_replaying),
            ReplayEvent::Redo { .. } => self.redo(is_replaying),
            ReplayEvent::Scramble { time } => {
                self.do_action(Action::Scramble { time }, is_replaying);
            }
            ReplayEvent::Twists(twists) => {
                self.do_action(
                    Action::Twists {
                        old_stm_counter: self.stm_counter.clone(),
                        twists,
                    },
                    is_replaying,
                );
            }
            ReplayEvent::SetBlindfold { enabled, .. } => self.blindfolded = enabled, /* TODO: actually have an effect */
            ReplayEvent::InvalidateFilterless { .. } => self.invalidated_filterless = true,
            ReplayEvent::StartSolve { time, duration } => {
                self.do_action(Action::StartSolve { time, duration }, is_replaying);
                self.stm_counter.reset();
            }
            ReplayEvent::EndSolve { time, duration } => {
                self.do_action(Action::EndSolve { time, duration }, is_replaying);
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

    fn undo(&mut self, is_replaying: bool) {
        // Keep undoing until we find an action that can be undone.
        while let Some(action) = self.undo_stack.pop() {
            match action.undo_behavior() {
                UndoBehavior::Action => {
                    if self.undo_action_internal(&action, is_replaying) {
                        self.redo_stack.push(action);
                    }
                    break;
                }
                UndoBehavior::Marker => {
                    if self.undo_action_internal(&action, is_replaying) {
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
    fn redo(&mut self, is_replaying: bool) {
        // Keep redoing until we find an action that can be redone.
        while let Some(action) = self.redo_stack.pop() {
            if self.do_action_internal(&action, is_replaying) {
                self.undo_stack.push(action);
                break;
            }
        }
    }

    /// Does an undoable action and saves it to the undo stack.
    ///
    /// Clears the redo stack if applicable.
    fn do_action(&mut self, mut action: Action, is_replaying: bool) {
        if let Action::Twists { twists, .. } = &mut action {
            let puz = self.puzzle_type();
            // TODO: don't filter here; filter as we apply moves
            twists.retain(|twist| {
                puz.twists
                    .axis_from_move_family(&twist.transform.family)
                    .is_some_and(|axis| {
                        let layers_info = puz.axis_layers[axis];
                        !twist.layers.to_layer_mask(layers_info).is_empty()
                    })
            });
            if twists.is_empty() {
                if let Some(replay) = &mut self.replay {
                    replay.pop(); // And don't save the twists
                }
                return;
            }
        }

        if matches!(action, Action::Twists { .. }) && self.first_move_time.is_none() {
            self.first_move_time = Some(Instant::now());
        }

        self.has_unsaved_changes = true;
        match action.undo_behavior() {
            UndoBehavior::Action => self.redo_stack.clear(),
            UndoBehavior::Marker | UndoBehavior::Boundary => (),
        }
        self.undo_stack.push(action.clone());
        let any_effect = self.do_action_internal(&action, is_replaying);
        if !any_effect {
            self.undo_stack.pop(); // Actually don't save the action
            if let Some(replay) = &mut self.replay {
                replay.pop(); // And don't save the twists
            }
        }
    }
    /// Does an undoable action. Returns whether the action should be saved to
    /// the undo stack.
    fn do_action_internal(&mut self, action: &Action, is_replaying: bool) -> bool {
        self.has_unsaved_changes = true;
        match action {
            Action::Scramble { .. } => match &self.scramble {
                Some(scramble) => {
                    let scramble_twists = match hypuz_notation::parse_notation(
                        &scramble.twists,
                        hypuz_notation::Features::MAXIMAL,
                    ) {
                        Ok(node_list) => node_list,
                        Err(errors) => {
                            for e in errors {
                                log::error!("Error parsing scramble twists: {e}");
                            }

                            return false;
                        }
                    };
                    for node in scramble_twists.0 {
                        match node.into_move() {
                            Some(mv) => match self.latest_state.do_twist_dyn(&mv) {
                                Ok(new_state) => self.latest_state = new_state,
                                Err(e) => {
                                    log::error!(
                                        "Twist {mv:?} blocked in scramble due to pieces {e:?}",
                                    );
                                }
                            },
                            None => {
                                log::error!("Unsupported notation element in scramble");
                            }
                        }
                    }
                    self.skip_twist_animations();
                    true
                }
                None => false,
            },
            Action::Twists {
                old_stm_counter: _,
                twists,
            } => {
                let mut any_effect = false;
                for twist in twists {
                    any_effect |= self.do_twist(twist);
                }
                if any_effect
                    && !is_replaying
                    && !self.solved
                    && self.scramble.is_some()
                    && self.is_solved()
                {
                    self.solve_complete_time = Some(Instant::now());
                    self.do_event(ReplayEvent::EndSolve {
                        time: Some(Timestamp::now()),
                        duration: self.file_duration(),
                    });
                    if !self.has_been_reloaded_since_first_solved() {
                        self.request_to_show_solve_summary();
                    }
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
    fn undo_action_internal(&mut self, action: &Action, is_replaying: bool) -> bool {
        self.has_unsaved_changes = true;
        match action {
            Action::Scramble { .. } => false, // shouldn't be possible
            Action::Twists {
                old_stm_counter,
                twists,
            } => {
                let ret = self.do_action_internal(
                    &Action::Twists {
                        old_stm_counter: StmCounter::new(), // ignored
                        twists: twists
                            .iter()
                            .rev()
                            .filter_map(|twist| twist.clone().inv().ok())
                            .collect(),
                    },
                    is_replaying,
                );
                self.stm_counter = old_stm_counter.clone();
                ret
            }
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
    fn do_twist(&mut self, twist: &Move) -> bool {
        let puzzle = Arc::clone(self.puzzle_type());
        let Some(axis) = puzzle.twists.axis_from_move_family(&twist.transform.family) else {
            return false;
        };
        let layers_info = puzzle.axis_layers[axis];
        let layer_mask = twist.layers.to_layer_mask(layers_info);
        let grip = self.latest_state.compute_gripped_pieces(axis, &layer_mask);

        let mut nd_euclid_initial_transform = None;
        if let Some(nd_euclid) = &mut self.nd_euclid
            && let Some(partial) = nd_euclid.partial_twist_drag_state.take()
        {
            if partial.grip == grip {
                nd_euclid_initial_transform = Some(partial.transform);
            } else {
                self.cancel_popped_partial_twist(partial);
                // That call doesn't modify the puzzle state, so `grip` can
                // stay the same.
            }
        }

        match self.latest_state.do_twist_dyn(twist) {
            Ok(new_state) => {
                let Some(axis) = puzzle.twists.axis_from_move_family(&twist.transform.family)
                else {
                    return false;
                };
                let layers_info = puzzle.axis_layers[axis];

                let state = std::mem::replace(&mut self.latest_state, new_state);

                self.stm_counter
                    .count_twist(axis, twist.layers.to_layer_mask(layers_info));

                self.blocking_anim.clear();

                if let Some(nd_euclid) = &self.nd_euclid
                    && let Some(twist_id) =
                        puzzle.twists.names.id_from_name(&twist.transform.family)
                {
                    let geom = &nd_euclid.geom;
                    self.twist_anim.push(AnimationFromState {
                        state,
                        anim: NdEuclidPuzzleAnimation {
                            pieces: grip,
                            initial_transform: nd_euclid_initial_transform
                                .unwrap_or_else(|| Motor::ident(geom.ndim())),
                            final_transform: geom.twist_transforms[twist_id]
                                .powi(twist.multiplier.into()),
                        }
                        .into(),
                    });
                } else if let Some(symmetric) = &puzzle
                    .twists
                    .engine_data
                    .downcast_ref::<SymmetricTwistSystemEngineData>()
                    && let Ok(m) = symmetric.twist_motor(twist)
                {
                    self.twist_anim.push(AnimationFromState {
                        state,
                        anim: NdEuclidPuzzleAnimation {
                            pieces: grip,
                            initial_transform: nd_euclid_initial_transform
                                .unwrap_or_else(|| Motor::ident(symmetric.ndim())),
                            final_transform: m,
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

        let grip = self.latest_state.compute_gripped_pieces(axis, &layers);
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
        if let Some(nd_euclid) = &mut self.nd_euclid
            && let Some(partial) = nd_euclid.partial_twist_drag_state.take()
        {
            self.cancel_popped_partial_twist(partial);
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
        if let Some(nd_euclid) = &mut self.nd_euclid
            && let Some(partial) = &nd_euclid.partial_twist_drag_state
        {
            let puzzle = self.latest_state.ty();

            let closest_twist = nd_euclid
                .geom
                .twist_transforms
                .iter()
                .filter(|&(twist, _)| puzzle.twists.twists[twist].axis == partial.axis)
                .flat_map(|(twist, twist_transform)| {
                    let max_multiplier = puzzle.twists.twists[twist].scramble_max_multiplier;
                    std::iter::successors(Some(twist_transform.clone()), move |t| {
                        Some(t * twist_transform)
                    })
                    .take(max_multiplier.unwrap_or(Multiplier(1)).0 as usize)
                    .enumerate()
                    .map(move |(i, twist_transform)| {
                        let notation_transform =
                            notation::Transform::new(&puzzle.twists.names[twist], None);
                        let mv = Move {
                            layers: LayerPrefix::DEFAULT,
                            transform: notation_transform,
                            multiplier: Multiplier((i + 1) as _),
                        };
                        (mv, twist_transform)
                    })
                })
                .max_by_key(|(_, twist_transform)| {
                    FloatOrd(Motor::dot(&partial.transform, twist_transform).abs())
                });
            if let Some((mut mv, twist_transform)) = closest_twist {
                let dot_with_twist = Motor::dot(&partial.transform, &twist_transform).abs();
                let dot_with_identity = partial.transform.get(hypermath::pga::Axes::SCALAR).abs();
                if dot_with_twist > dot_with_identity {
                    mv.layers = LayerPrefix::from(&partial.layers);
                    let axis = partial.axis;
                    let time = Some(Timestamp::now());
                    self.do_event(ReplayEvent::DragTwist { time, axis });
                    self.do_event(ReplayEvent::Twists(smallvec![mv]));
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

    /// Returns the combined session time of the file in milliseconds.
    pub fn file_duration(&self) -> Option<i64> {
        Some(self.old_duration? + self.load_time.elapsed().as_millis() as i64)
    }

    /// Returns the inspection duration, if currently during inspection.
    pub fn inspection_duration(&self) -> Option<Duration> {
        if self.first_move_time.is_none() {
            Some(self.inspection_start_time?.elapsed())
        } else {
            None
        }
    }

    /// Returns the speedsolve duration, if currently speedsolving or after
    /// completion of a speedsolve.
    pub fn speedsolve_duration(&self) -> Option<Duration> {
        self.is_single_session.then_some(
            self.solve_complete_time
                .unwrap_or_else(Instant::now)
                .saturating_duration_since(self.first_move_time?),
        )
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
                        ref layers,
                        ref target,
                        reverse,
                    } => LogEvent::Click {
                        time,
                        layers: layers.clone(),
                        target: target.to_string(),
                        reverse,
                    },
                    &ReplayEvent::DragTwist { time, axis } => LogEvent::DragTwist {
                        time,
                        axis: puz.axes().names[axis].to_string(),
                    },
                    ReplayEvent::Twists(twists) => {
                        let mut s = twists.iter().map(|mv| mv.to_string()).join(" ");
                        if twists.len() > 1 {
                            s.insert(0, '(');
                            s.push(')');
                        }
                        LogEvent::Twists(s)
                    }
                    &ReplayEvent::SetBlindfold { time, enabled } => {
                        LogEvent::SetBlindfold { time, enabled }
                    }
                    &ReplayEvent::InvalidateFilterless { time } => {
                        LogEvent::InvalidateFilterless { time }
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
                    Action::Twists {
                        old_stm_counter: _,
                        twists,
                    } => {
                        if twists.is_empty() {
                            continue;
                        }
                        let mut s = twists.iter().map(|mv| mv.to_string()).join(" ");
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
                id: puz.meta.id.to_string(),
                version: puz.meta.version.to_string(),
            },
            solved: self
                .undo_stack
                .iter()
                .any(|event| matches!(event, Action::EndSolve { .. })),
            duration: self.file_duration(),
            scramble: self.scramble.clone(),
            log,
            tsa_signature_v1: None,
            tsa_signature_v2: None,
            tsa_signature_v3: self.tsa_signature_v3.clone(),
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
            tsa_signature_v1: _,
            tsa_signature_v2: _,
            tsa_signature_v3,
        } = solve;

        log::trace!("Loading file ...");

        let mut ret = Self::new(puzzle);

        let is_replay = replay.unwrap_or(false);
        ret.replay = is_replay.then(Vec::new);

        for event in log {
            match event {
                &LogEvent::Scramble { time } => {
                    log::trace!("Applying scramble {scramble:?}");
                    ret.reset();
                    ret.replay = is_replay.then(Vec::new);
                    ret.scramble = scramble.clone();
                    ret.replay_event(ReplayEvent::Scramble { time }, true);
                }
                &LogEvent::Click {
                    time,
                    ref layers,
                    ref target,
                    reverse,
                } => {
                    ret.replay_event(
                        ReplayEvent::GizmoClick {
                            time,
                            layers: layers.clone(),
                            target: target.clone(),
                            reverse,
                        },
                        true,
                    );
                }
                &LogEvent::DragTwist { time, ref axis } => {
                    // TODO: handle errors
                    let Some(axis) = puzzle.axes().names.id_from_name(axis) else {
                        continue;
                    };
                    ret.replay_event(ReplayEvent::DragTwist { time, axis }, true);
                }
                LogEvent::Twists(twists_str) => {
                    match hypuz_notation::parse_notation(
                        twists_str,
                        hypuz_notation::Features::MAXIMAL,
                    ) {
                        Ok(node_list) => {
                            let group = node_list
                                .0
                                .into_iter()
                                .filter_map(|node| {
                                    node.into_move().or_else(|| {
                                        log::error!("Unsupported twist notation");
                                        None
                                    })
                                })
                                .collect();
                            log::trace!("Applying twist group {group:?} from {twists_str:?}");
                            ret.replay_event(ReplayEvent::Twists(group), true);
                        }
                        Err(errors) => {
                            log::error!(
                                "Error parsing twist group {twists_str:?}: {}",
                                errors.iter().join(", "),
                            );
                        }
                    }
                }
                &LogEvent::Undo { time } => ret.replay_event(ReplayEvent::Undo { time }, true),
                &LogEvent::Redo { time } => ret.replay_event(ReplayEvent::Redo { time }, true),
                &LogEvent::SetBlindfold { time, enabled } => {
                    ret.replay_event(ReplayEvent::SetBlindfold { time, enabled }, true);
                }
                &LogEvent::InvalidateFilterless { time } => {
                    ret.replay_event(ReplayEvent::InvalidateFilterless { time }, true);
                }
                LogEvent::Macro { time: _ } => log::error!("Macros are unsupported"), /* TODO apply macro */
                LogEvent::StartSolve { time, duration } => {
                    ret.started = true;
                    ret.replay_event(
                        ReplayEvent::StartSolve {
                            time: *time,
                            duration: *duration,
                        },
                        true,
                    );
                }
                LogEvent::EndSolve { time, duration } => {
                    ret.solved = true;
                    ret.replay_event(
                        ReplayEvent::EndSolve {
                            time: *time,
                            duration: *duration,
                        },
                        true,
                    );
                }
                LogEvent::StartSession { time } => {
                    ret.replay_event(ReplayEvent::StartSession { time: *time }, true);
                }
                LogEvent::EndSession { time } => {
                    ret.replay_event(ReplayEvent::EndSession { time: *time }, true);
                }
            }
        }

        ret.replay_event(
            ReplayEvent::StartSession {
                time: Some(Timestamp::now()),
            },
            true,
        );

        ret.old_duration = *duration;
        ret.has_unsaved_changes = false;
        ret.is_single_session = false;
        ret.request_to_show_solve_summary = false;
        if ret.solved {
            ret.has_been_reloaded_since_first_solved = true;
        }
        ret.tsa_signature_v3 = tsa_signature_v3.clone();

        ret.skip_twist_animations();

        ret
    }

    /// Returns the number of moves applied to the puzzle, measured in Slice
    /// Turn Metric.
    pub fn stm_count(&self) -> u64 {
        self.stm_counter.count
    }

    /// Returns whether the puzzle is _currently_ solved.
    pub fn is_solved(&self) -> bool {
        self.latest_state.is_solved()
    }
    /// Returns whether the puzzle has _ever_ been solved.
    pub fn has_been_solved(&self) -> bool {
        self.solved
    }
    /// Returns whether the log file has been reloaded since the puzzle was
    /// first solved.
    pub fn has_been_reloaded_since_first_solved(&self) -> bool {
        self.has_been_reloaded_since_first_solved
    }
    /// Requests to show the solve summary.
    pub fn request_to_show_solve_summary(&mut self) {
        self.request_to_show_solve_summary = true;
    }
    /// Returns whether the solve summary should be shown, and clears the flag.
    pub fn handle_solve_summary_request(&mut self) -> bool {
        std::mem::take(&mut self.request_to_show_solve_summary)
    }
    /// Updates the sticker colors and invalidates the no-filters status of the
    /// solve if they have changed.
    pub fn update_colors(&mut self, sticker_colors: &[[u8; 3]]) {
        if !self.invalidated_filterless {
            if self.original_colors.is_none() {
                self.original_colors = Some(sticker_colors.to_vec());
            } else if self.original_colors.as_deref() != Some(sticker_colors) {
                self.invalidate_filterless();
            }
        }
    }
    /// Invalidates the no-filters status of the solve.
    pub fn invalidate_filterless(&mut self) {
        self.do_event(ReplayEvent::InvalidateFilterless {
            time: Some(Timestamp::now()),
        });
    }
    /// Returns whether the solve is valid for a no-filters solve.
    pub fn has_been_filterless(&self) -> bool {
        !self.invalidated_filterless
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const EXAMPLE_REPLAY_FILE: &'static str = r#"
// Hyperspeedcube puzzle log
version 2
program name=Hyperspeedcube version="2.0.0-zeta.5"
solve {
    replay #true
    puzzle id=ft_cube:2 version="1.0.1"
    solved #true
    duration 9359
    scramble full version=1 time="2026-03-03T00:11:32.500Z" seed="2026-03-03T00:11:32.500Z_yKebWcuWfKqEog3YY/UtaAzDp4JKrIELvl6dMIKMzDY=" {
        twists "2L2 {1-2}B2 2B {1-2}R 2B B' L2 2B 2R2 {1-2}U2 {1-2}R U2 R 2L2 F {1-2}L F2 B' F F F2 L R2 U2 R' 2L {1-2}R B' F {1-2}D' {1-2}D2 2L2 {1-2}R D2 {1-2}D' F' {1-2}U 2L' 2R2 {1-2}F' {1-2}R2 L R' 2B 2F 2L' 2B2 2F R2 {1-2}D2 2R L {1-2}R' 2D' B2 {1-2}R' 2B' {1-2}R F2 2D2 2B 2B2 2U2 U2 R 2U2 {1-2}L2 {1-2}R U' 2F2 {1-2}L2 R 2L {1-2}F {1-2}R' F' F {1-2}R' {1-2}F {1-2}R2 {1-2}B' U' F D2 2U D' L L 2L {1-2}F B' 2F' {1-2}B2 {1-2}R' R2 2F' {1-2}F' D L2 {1-2}F {1-2}R' {1-2}F' {1-2}R D L' {1-2}F B' L {1-2}F2 2D' F2 2F' L2 {1-2}D' 2L2 2R2 {1-2}R' 2U U2 2B' 2U2 R {1-2}F {1-2}L' B2 U2 {1-2}D' U2 L 2L2 B' 2F2 {1-2}U F' 2R2 2D 2R' {1-2}L2 {1-2}L {1-2}D {1-2}F' {1-2}R R F' 2L2 {1-2}B2 2F 2F2 R2 {1-2}U' {1-2}F' 2R' L2 {1-2}D2 2B2 2U' {1-2}B2 F' 2D' {1-2}D' 2U' B2 2L L' U2 R' 2B2 2L {1-2}R2 2F' 2F2 {1-2}L {1-2}L2 D2 D2 {1-2}R2 2U 2B2 {1-2}D' 2F2 F' 2D2 L' 2L 2L' 2R {1-2}D2 B' U2 {1-2}U2 {1-2}B' B 2R' 2D' D2 {1-2}U' L2 {1-2}L2 {1-2}B' {1-2}D2 {1-2}F2 {1-2}R' {1-2}L2 R2 2F' {1-2}D {1-2}B' 2D' F' {1-2}R {1-2}B' 2F' {1-2}L2 2F 2F D2 2B' {1-2}D R' {1-2}U' {1-2}F' 2R D' B {1-2}U {1-2}B2 {1-2}B2 2B D 2B' {1-2}B' {1-2}D 2U2 {1-2}B {1-2}U 2F' 2B2 {1-2}L' B' B2 {1-2}U2 2R {1-2}F2 2F2 {1-2}L2 2B {1-2}U2 {1-2}L2 F' {1-2}D' 2R' {1-2}L' {1-2}D L2 R2 {1-2}F {1-2}B' 2D' 2L2 2L2 B2 {1-2}R 2U' {1-2}F B 2F 2D L' {1-2}L 2D2 {1-2}D U2 2L' L' 2F' 2D' {1-2}D' 2R' {1-2}L 2R {1-2}F2 {1-2}U' 2R2 R {1-2}D' F2 2F' {1-2}L R' D2 {1-2}F' 2L' {1-2}B2 2D U2 R 2L' {1-2}B D2 L' D {1-2}R' 2R' R' {1-2}F' L' 2R2 2B' 2L2 2U' 2F2 2L' {1-2}F' R2 U' {1-2}D {1-2}D' B 2R D' 2U' 2F {1-2}D2 L2 {1-2}B2 L {1-2}D2 2U' L2 2R 2L U2 {1-2}D' 2D2 2U' {1-2}L {1-2}B 2F' 2B' {1-2}L 2L F2 U' {1-2}D {1-2}R' F' B' R 2R2 D2 {1-2}L2 {1-2}U2 2R 2B {1-2}F2 {1-2}D2 {1-2}D2 R' D' 2F2 B {1-2}U {1-2}D {1-2}D' D' U' 2D2 2U L2 {1-2}D2 2F2 {1-2}B' {1-2}B' R2 2D2 F2 F' {1-2}L 2B2 L 2U' L D2 L' {1-2}B' 2F F' 2F2 {1-2}U {1-2}F2 2L' F' U2 {1-2}F2 {1-2}B2 {1-2}R' 2F R {1-2}D' F2 D2 2L 2U2 L2 L2 2D' F2 2B B {1-2}F2 2B2 2U' {1-2}B2 U2 L' {1-2}B {1-2}R' {1-2}F2 {1-2}B' D' {1-2}D B' 2U {1-2}L L 2B2 F' {1-2}F2 U' {1-2}D2 U2 U' {1-2}F {1-2}U2 2F {1-2}L2 {1-2}L 2D 2L' {1-2}R2 2D L' {1-2}U' U2 {1-2}L2 {1-2}F 2U' 2L L {1-2}R D 2L {1-2}D L2 R2 2U' {1-2}L2 {1-2}D L 2D {1-2}F' F2 2R B' 2R {1-2}D' 2L 2R D 2L2 R' L' {1-2}L' B' 2R B B {1-2}F2 {1-2}D' B 2U' {1-2}F2 2R2 2B D2 2R F' {1-2}D2 2D' 2F2 R2 U2 2D {1-2}L2 {1-2}L L2 2D' {1-2}L2 {1-2}D {1-2}U2 {1-2}B2 D2 2L' {1-2}R' L' B2 2D B2 {1-2}L' U U {1-2}D' {1-2}F' {1-2}R D2 {1-2}L' 2F2 2D 2B {1-2}D' 2B F {1-2}F' R' 2F R2 {1-2}B {1-2}D' U L {1-2}F L' {1-2}U 2U' 2B' 2F' 2U D' 2R' 2L' L 2D L B' B2 2R2 B2 L' 2B 2F2 R' 2U' 2B B' B' {1-2}L' {1-2}B {1-2}U U2 U2 D' 2F L {1-2}F' D' 2R2 U {1-2}B2 B2 {1-2}U' 2B' 2L2 D U2 F2 2L2 2B {1-2}F {1-2}L' R' {1-2}B 2F2 2B' {1-2}R' D D 2R' 2L D B2 {1-2}D D2 2F' 2D2 D 2D' D2 R' 2R R' F R' 2R {1-2}F2 2R2 {1-2}L 2B' {1-2}F' 2F' F2 2F' 2D 2B 2R2 B {1-2}F2 {1-2}F' {1-2}F {1-2}R2 2L' D2 R 2R' 2U B' U2 {1-2}B U F {1-2}B2 2R' {1-2}F' 2B' 2L2 L' U' 2U2 L2 2R2 R 2B F {1-2}R' F {1-2}B' {1-2}R' {1-2}R2 B' U2 D D2 2B 2B2 2D' F F' {1-2}R' 2U' 2F2 2D F {1-2}L 2F' {1-2}R2 {1-2}D {1-2}F B B 2U {1-2}U' 2D2 {1-2}R2 2U 2B2 {1-2}R 2R2 D2 U' {1-2}U2 2R' U 2U F R' B' {1-2}R2 {1-2}D {1-2}U D 2D2 2D F B D2 {1-2}F2 {1-2}R' 2D D 2F2 2F2 {1-2}R' 2U' {1-2}L2 D' {1-2}F2 2R' {1-2}R' {1-2}U 2U L2 2F' U2 2D 2D2 {1-2}L {1-2}F {1-2}R {1-2}U 2U' 2F2 2L 2R {1-2}B {1-2}B' {1-2}B {1-2}B' {1-2}F F {1-2}U L2 2L2 D' {1-2}L' 2B2 2U' {1-2}D R2 U' {1-2}R {1-2}D 2L2 D2 {1-2}F F' F D2 2L2 D' {1-2}R' {1-2}U2 {1-2}R2 2B L' 2L U' {1-2}L F2 2R' B2 {1-2}U' 2U R2 {1-2}U2 B' D' F2 2D {1-2}F U' {1-2}R' 2U {1-2}L 2F2 U' F' {1-2}U' 2F' R2 {1-2}F2 B R' {1-2}B' D' L {1-2}F' {1-2}L R2 2D' 2L' D' 2F D {1-2}U' L2 2F' {1-2}D2 2F' 2F2 {1-2}U {1-2}U2 L2 2L2 2D2 2U' 2B' 2B' U 2F2 2F F2 2L' 2F {1-2}U' 2L' U U U2 {1-2}L2 F U B 2L' U 2D' R 2U2 {1-2}D 2U 2R' {1-2}B {1-2}F' F' R' {1-2}B 2D {1-2}L R2 R2 {1-2}R {1-2}F2 {1-2}U D' {1-2}L' R2 2B' {1-2}B' R {1-2}L' L 2D' B' 2D2 2F2 F2 D2 R2 U' {1-2}U 2U' {1-2}D' 2D2 {1-2}L' D D F' 2F 2L 2U' L2 2L2 {1-2}D' 2R2 L R2 {1-2}D2 2B' 2F2 U' {1-2}D2 {1-2}B {1-2}R' {1-2}D2 R2 F' D2 R' 2D2 R' 2U2 F2 B' 2B2 {1-2}U' 2D {1-2}F' L L2 {1-2}F2 R 2L' {1-2}R B2 B2 {1-2}L {1-2}U' 2F F 2F2 2F {1-2}L 2U2 {1-2}B' {1-2}D' F2 2D' F' 2F' 2L 2D2 D' {1-2}U' 2F {1-2}D2 D2 2L' {1-2}D R2 2L 2L 2R' 2D' 2B' {1-2}D2 2R {1-2}L B' {1-2}L 2B2 2F 2D2 {1-2}L 2D' {1-2}U' 2D 2F' F' 2D 2D' {1-2}D 2L' {1-2}F2 U2 {1-2}L 2U {1-2}U2 {1-2}U' {1-2}R2 2B2 {1-2}R 2R' D D 2B D2 {1-2}U2 {1-2}F' R2 {1-2}F' D2 B2 {1-2}L2 2D2 {1-2}D {1-2}R' {1-2}D 2L2 2U 2R' F2 U' 2R {1-2}B' U' {1-2}R' {1-2}R2 2R 2U' 2D' 2F' {1-2}F' {1-2}F2 B' {1-2}F' 2B 2L {1-2}F {1-2}R2 B 2R' 2R'"
        drand_round_v1 round=26564442 signature=b2afc3059c9758ab7f605b8e7eb714b044c18140438be3e1d594db6770896b3d0de74a7fa9ea2a0138dacbb034159e2c
    }
    log {
        scramble time="2026-03-03T00:11:32.500Z"
        start-session time="2026-03-03T00:11:32.515Z"
        click time="2026-03-03T00:11:36.433Z" layers=1 target=D reverse=#true
        start-solve time="2026-03-03T00:11:36.433Z" duration=3917
        twists D'
        click time="2026-03-03T00:11:36.662Z" layers=1 target=R
        twists R
        click time="2026-03-03T00:11:36.877Z" layers=1 target=D
        twists D
        click time="2026-03-03T00:11:37.162Z" layers=1 target=R reverse=#true
        twists R'
        click time="2026-03-03T00:11:37.269Z" layers=1 target=R reverse=#true
        twists R'
        click time="2026-03-03T00:11:38.998Z" layers=1 target=L
        twists L
        click time="2026-03-03T00:11:39.115Z" layers=1 target=L
        twists L
        click time="2026-03-03T00:11:39.512Z" layers=1 target=D
        twists D
        click time="2026-03-03T00:11:39.775Z" layers=1 target=L
        twists L
        click time="2026-03-03T00:11:39.873Z" layers=1 target=L
        twists L
        click time="2026-03-03T00:11:40.158Z" layers=1 target=D
        twists D
        click time="2026-03-03T00:11:40.248Z" layers=1 target=D
        twists D
        click time="2026-03-03T00:11:40.470Z" layers=1 target=F
        twists F
        click time="2026-03-03T00:11:40.568Z" layers=1 target=F
        twists F
        click time="2026-03-03T00:11:40.775Z" layers=1 target=D
        twists D
        click time="2026-03-03T00:11:41.123Z" layers=1 target=F
        twists F
        click time="2026-03-03T00:11:41.234Z" layers=1 target=F
        twists F
        click time="2026-03-03T00:11:41.874Z" layers=1 target=D
        twists D
        end-solve time="2026-03-03T00:11:41.874Z" duration=9358
        end-session time="2026-03-03T00:11:41.875Z"
    }
}
    "#;

    #[test]
    fn test_puzzle_sim_replay_round_trip() {
        hyperpuzzle::load_global_catalog();
        let (log_file, _) = hyperpuzzle_log::deserialize(EXAMPLE_REPLAY_FILE).unwrap();
        let original_solve = &log_file.solves[0];
        let puzzle = hyperpuzzle::catalog()
            .build_blocking(&original_solve.puzzle.id.parse().unwrap())
            .unwrap();
        let sim = PuzzleSimulation::deserialize(&puzzle, original_solve);
        let mut reserialized_solve = sim.serialize(true);

        // Remove the new start-session and end-session events
        reserialized_solve.log.pop();
        reserialized_solve.log.pop();

        // The duration is allowed to change
        reserialized_solve.duration = original_solve.duration;

        pretty_assertions::assert_eq!(original_solve, &reserialized_solve);
    }

    #[test]
    fn test_puzzle_twist_with_no_effect() {
        hyperpuzzle::load_global_catalog();
        let puzzle = hyperpuzzle::catalog()
            .build_blocking(&"ft_cube(2)".parse().unwrap())
            .unwrap();
        let mut sim = PuzzleSimulation::new(&puzzle);
        sim.do_event(ReplayEvent::Twists(parse_twists("U")));
        sim.do_event(ReplayEvent::Twists(parse_twists("3R")));
        sim.do_event(ReplayEvent::Twists(parse_twists("D")));
        sim.do_event(ReplayEvent::Undo { time: None });
        sim.do_event(ReplayEvent::Twists(parse_twists("U'")));
        assert!(sim.is_solved());
        let replay = sim.replay.as_ref().unwrap();
        assert!(matches!(replay[0], ReplayEvent::StartSession { .. }));
        assert_eq!(replay[1], ReplayEvent::Twists(parse_twists("U")));
        // 3R not saved
        assert_eq!(replay[2], ReplayEvent::Twists(parse_twists("D")));
        assert!(matches!(replay[3], ReplayEvent::Undo { .. }));
        assert_eq!(replay[4], ReplayEvent::Twists(parse_twists("U'")));
    }

    fn parse_twists(s: &str) -> smallvec::SmallVec<[Move; 1]> {
        hypuz_notation::parse_notation(s, hypuz_notation::Features::MAXIMAL)
            .unwrap()
            .0
            .into_iter()
            .map(|node| node.into_move().expect("unsupported notation"))
            .collect()
    }
}
