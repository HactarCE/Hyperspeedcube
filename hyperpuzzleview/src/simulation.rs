use std::sync::Arc;

use float_ord::FloatOrd;
use hypermath::pga::{Axes, Motor};
use hypermath::{Vector, VectorRef};
use hyperprefs::AnimationPreferences;
use hyperpuzzle::{
    Axis, LayerMask, LayeredTwist, PerPiece, PieceMask, Puzzle, PuzzleState, ScrambleInfo,
    ScrambleType,
};
use hyperpuzzlelog::Scramble;
use smallvec::{smallvec, SmallVec};
use web_time::{Duration, Instant};

use super::animations::{BlockingAnimationState, TwistAnimation, TwistAnimationState};
use super::{Action, ReplayEvent};

const ASSUMED_FPS: f32 = 120.0;

/// Puzzle simulation, which manages the puzzle state, animations, undo stack,
/// etc.
#[derive(Debug, Clone)]
pub struct PuzzleSimulation {
    /// Latest puzzle state, not including any transient rotation.
    latest_state: PuzzleState,

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

    /// Time of last frame, or `None` if we are not in the middle of an animation.
    last_frame_time: Option<Instant>,
    /// Twist animation state.
    twist_anim: TwistAnimationState,
    /// Blocking pieces animation state.
    blocking_anim: BlockingAnimationState,

    /// Twist drag state.
    partial_twist_drag_state: Option<PartialTwistDragState>,

    /// Latest visual piece transforms.
    cached_piece_transforms: PerPiece<Motor>,
}
impl PuzzleSimulation {
    /// Constructs a new simulation with a fresh puzzle state.
    pub fn new(puzzle: &Arc<Puzzle>) -> Self {
        let latest_state = PuzzleState::new(Arc::clone(puzzle));
        let cached_piece_transforms = latest_state.piece_transforms().clone();
        Self {
            latest_state,

            scramble: None,
            has_unsaved_changes: false,
            undo_stack: vec![],
            redo_stack: vec![],
            replay: vec![],

            last_frame_time: None,
            twist_anim: TwistAnimationState::default(),
            blocking_anim: BlockingAnimationState::default(),

            partial_twist_drag_state: None,

            cached_piece_transforms,
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
            .unwrap_or_else(|| self.latest_state.piece_transforms().clone());
    }
    /// Updates the piece transforms, ignoring the current animation state. This
    /// is called after updating the puzzle state with no animation.
    fn update_piece_transforms_static(&mut self) {
        self.cached_piece_transforms = self.latest_state.piece_transforms().clone();
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
        self.scramble
            .as_ref()
            .is_some_and(|scramble| scramble.info.ty == ScrambleType::Full)
    }

    /// Resets the puzzle state and replay log.
    pub fn reset(&mut self) {
        *self = Self::new(self.puzzle_type());
    }
    /// Resets and scrambles the puzzle.
    ///
    /// This generates the appropriate [`ReplayEvent::Scramble`] and then
    /// executes it.
    pub fn scramble(&mut self, scramble_info: ScrambleInfo) {
        let ty = self.puzzle_type();
        let (twists, _state) = ty.new_scrambled(scramble_info);
        self.event(ReplayEvent::Scramble(Scramble {
            info: scramble_info,
            twists: hyperpuzzlelog::notation::format_twists(&ty.twists, twists),
        }));
    }
    /// Plays a replay event on the puzzle.
    pub fn event(&mut self, event: ReplayEvent) {
        self.replay.push(event.clone());
        match event {
            ReplayEvent::Undo => self.undo(),
            ReplayEvent::Redo => self.redo(),
            ReplayEvent::Scramble(scramble) => {
                self.reset();
                let ty = Arc::clone(self.puzzle_type());
                for twist in
                    hyperpuzzlelog::notation::parse_twists(&ty.twist_by_name, &scramble.twists)
                {
                    match twist {
                        Ok(twist) => match self.latest_state.do_twist(twist) {
                            Ok(new_state) => self.latest_state = new_state,
                            Err(e) => {
                                log::error!(
                                    "twist {twist:?} blocked in scramble due to pieces {e:?}"
                                )
                            }
                        },
                        Err(e) => log::error!("error parsing twist in scramble: {e}"),
                    }
                }
                self.update_piece_transforms_static();
                self.replay.push(ReplayEvent::Scramble(scramble));
            }
            ReplayEvent::Twists(twists) => self.do_action(Action::Twists(twists)),
            _ => (),
        }
    }

    /// Returns whether there is an action available to undo.
    pub fn has_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }
    /// Returns whether there is an action available to redo.
    pub fn has_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            if let Some(reverse_action) = self.do_action_internal(action) {
                self.redo_stack.push(reverse_action);
            }
        }
    }
    fn redo(&mut self) {
        if let Some(action) = self.redo_stack.pop() {
            if let Some(reverse_action) = self.do_action_internal(action) {
                self.undo_stack.push(reverse_action);
            }
        }
    }

    /// Does an undoable action and saves it to the undo stack.
    ///
    /// Clears the redo stack if applicable.
    fn do_action(&mut self, action: Action) {
        self.has_unsaved_changes = true;
        self.redo_stack.clear();
        if let Some(reverse_action) = self.do_action_internal(action) {
            self.undo_stack.push(reverse_action);
        }
    }
    /// Does an undoable action and then returns the reverse action.
    fn do_action_internal(&mut self, action: Action) -> Option<Action> {
        self.has_unsaved_changes = true;
        match action {
            Action::Twists(twists) => {
                let reverse_twists = twists
                    .iter()
                    .rev()
                    .map_while(|&twist| self.do_twist(twist))
                    .collect::<SmallVec<_>>();
                (!reverse_twists.is_empty()).then(|| Action::Twists(reverse_twists))
            }
        }
    }

    /// Executes a twist on the puzzle and queues the appropriate animation.
    /// Returns the inverse twist if successful.
    ///
    /// Any in-progress partial twist is canceled.
    ///
    /// This does **not** affect the undo stack. Use [`Self::event()`] instead
    /// if that's desired.
    fn do_twist(&mut self, twist: LayeredTwist) -> Option<LayeredTwist> {
        let puzzle = Arc::clone(self.puzzle_type());
        let twist_info = puzzle.twists.get(twist.transform).ok()?;
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
                Some(LayeredTwist {
                    layers: twist.layers,
                    transform: twist_info.reverse,
                })
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
                None
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

        //     let decay_multiplier = VIEW_ANGLE_OFFSET_DECAY_RATE.powf(delta.as_secs_f32());
        //     let new_offset = Quaternion::one().slerp(*offset, decay_multiplier);
        //     if offset.s == new_offset.s {
        //         // Stop the animation once we're not making any more progress.
        //         *offset = Quaternion::one();
        //     } else {
        //         *offset = new_offset;
        //     }
        // }

        if self.twist_anim.proceed(delta, animation_prefs) {
            self.update_piece_transforms(&animation_prefs);
            needs_redraw = true;
        }
        needs_redraw |= self.blocking_anim.proceed(&animation_prefs);

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
        self.update_piece_transforms(&animation_prefs);
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
                    self.event(ReplayEvent::DragTwist);
                    self.event(ReplayEvent::Twists(smallvec![twist]));
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

#[derive(Debug, Clone)]
pub struct PartialTwistDragState {
    pub axis: Axis,
    pub layers: LayerMask,
    pub grip: PieceMask,
    pub transform: Motor,
}