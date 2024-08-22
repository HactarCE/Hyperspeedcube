use std::sync::Arc;

use float_ord::FloatOrd;
use hypermath::pga::{Axes, Motor};
use hypermath::{Vector, VectorRef};
use hyperpuzzle::{Axis, LayerMask, PerPiece, PieceMask, Puzzle, PuzzleState, Twist};
use instant::{Duration, Instant};

use super::animations::{BlockingAnimationState, TwistAnimation, TwistAnimationState};
use crate::preferences::{AnimationPreferences, InteractionPreferences, Preferences, Preset};

const ASSUMED_FPS: f32 = 120.0;

/// Puzzle simulation, which manages the puzzle state, animations, undo stack,
/// etc.
#[derive(Debug, Clone)]
pub struct PuzzleSimulation {
    /// Latest puzzle state, not including any transient rotation.
    latest_state: PuzzleState,

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

    pub interaction_prefs: Preset<InteractionPreferences>,
    pub animation_prefs: Preset<AnimationPreferences>,
}
impl PuzzleSimulation {
    pub fn new(puzzle: &Arc<Puzzle>, prefs: &Preferences) -> Self {
        let latest_state = PuzzleState::new(Arc::clone(puzzle));
        let cached_piece_transforms = latest_state.piece_transforms().clone();
        Self {
            latest_state,

            last_frame_time: None,
            twist_anim: TwistAnimationState::default(),
            blocking_anim: BlockingAnimationState::default(),

            partial_twist_drag_state: None,

            cached_piece_transforms,

            interaction_prefs: prefs.interaction.current_preset(),
            animation_prefs: prefs.animation.current_preset(),
        }
    }

    pub fn puzzle(&self) -> &PuzzleState {
        &self.latest_state
    }
    pub fn puzzle_type(&self) -> &Arc<Puzzle> {
        self.latest_state.ty()
    }
    pub fn ndim(&self) -> u8 {
        self.puzzle_type().ndim()
    }

    /// Returns the latest piece transforms.
    pub fn piece_transforms(&self) -> &PerPiece<Motor> {
        &self.cached_piece_transforms
    }
    /// Updates the piece transforms. This is called every frame that the puzzle
    /// is in motion.
    fn update_piece_transforms(&mut self) {
        self.cached_piece_transforms = self
            .twist_anim
            .current()
            .map(|(anim, t)| {
                let t = self
                    .animation_prefs
                    .value
                    .twist_interpolation
                    .interpolate(t);
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

    /// Executes a twist on the puzzle and queues the appropriate animation.
    ///
    /// Any in-progress partial twist is canceled.
    pub fn do_twist(&mut self, twist: Twist, layers: LayerMask) {
        let puzzle = Arc::clone(self.puzzle_type());
        let twist_info = &puzzle.twists[twist];
        let axis = twist_info.axis;
        let grip = self.latest_state.compute_gripped_pieces(axis, layers);

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

        match self.latest_state.do_twist(twist, layers) {
            Ok(new_state) => {
                let state = std::mem::replace(&mut self.latest_state, new_state);
                self.twist_anim.push(TwistAnimation {
                    state,
                    grip,
                    initial_transform,
                    final_transform: twist_info.transform.clone(),
                });
                self.blocking_anim.clear();
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
            }
        }
    }

    /// Advances the puzzle geometry and internal state to the next frame, using
    /// the given time delta between this frame and the last. Returns whether
    /// the puzzle must be redrawn.
    pub fn step(&mut self, prefs: &Preferences) -> bool {
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

        if self.twist_anim.proceed(delta, &self.animation_prefs.value) {
            self.update_piece_transforms();
            needs_redraw = true;
        }
        needs_redraw |= self.blocking_anim.proceed(&self.animation_prefs.value);

        if needs_redraw {
            self.last_frame_time = Some(now);
        } else {
            self.last_frame_time = None;
        }

        needs_redraw
    }

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
    pub fn update_partial_twist(&mut self, surface_normal: Vector, parallel_drag_delta: Vector) {
        let puzzle = Arc::clone(self.puzzle_type());
        if let Some(partial) = &mut self.partial_twist_drag_state {
            let axis_vector = &puzzle.axes[partial.axis].vector;
            let Some(v1) = surface_normal.cross_product_3d(axis_vector).normalize() else {
                return;
            };
            let Some(v2) = axis_vector.cross_product_3d(&v1).normalize() else {
                return;
            };
            // TODO: consider scaling by torque (i.e., radius)
            let angle = v1.dot(&parallel_drag_delta);
            let new_transform = Motor::from_angle_in_normalized_plane(3, &v2, &v1, angle);
            partial.transform = new_transform;
        }
        self.update_piece_transforms();
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
        })
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
                    self.do_twist(twist, partial.layers);
                } else {
                    // The identity twist is closer.
                    self.cancel_partial_twist()
                }
            } else {
                // There are no possible twists. Why did we even let the user
                // drag the mouse in the first place? Questions such as these
                // may never know an answer.
                self.cancel_partial_twist()
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
