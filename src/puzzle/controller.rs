//! Puzzle wrapper that adds animation and undo history functionality.

use anyhow::anyhow;
use cgmath::{Matrix4, SquareMatrix};
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::time::Duration;

use super::{LayerMask, TwistDirection, TwistMetric};
use crate::preferences::Preferences;

/// If at least this much of a twist is animated in one frame, just skip the
/// animation to reduce unnecessary flashing.
const MIN_TWIST_DELTA: f32 = 1.0 / 3.0;

/// Interpolation functions.
pub mod interpolate {
    use std::f32::consts::PI;

    /// Function that maps a float from the range 0.0 to 1.0 to another float
    /// from 0.0 to 1.0.
    pub type InterpolateFn = fn(f32) -> f32;

    /// Interpolate using cosine from 0.0 to PI.
    pub const COSINE: InterpolateFn = |x| (1.0 - (x * PI).cos()) / 2.0;
    /// Interpolate using cosine from 0.0 to PI/2.0.
    pub const COSINE_ACCEL: InterpolateFn = |x| 1.0 - (x * PI / 2.0).cos();
    /// Interpolate using cosine from PI/2.0 to 0.0.
    pub const COSINE_DECEL: InterpolateFn = |x| ((1.0 - x) * PI / 2.0).cos();
}

use super::{rubiks4d_logfile::*, Face, Piece, Sticker};
use super::{traits::*, PuzzleType, Rubiks4D};
use interpolate::InterpolateFn;

const INTERPOLATION_FN: InterpolateFn = interpolate::COSINE;

/// Puzzle wrapper that adds animation and undo history functionality.
pub struct PuzzleController<P: PuzzleState> {
    /// State of the puzzle right before the twist being animated right now.
    ///
    /// `Box`ed so that this struct is always the same size.
    displayed: Box<P>,
    /// State of the puzzle with all twists applied to it (used for timing
    /// and undo).
    ///
    /// `Box`ed so that this struct is always the same size.
    latest: Box<P>,
    /// Queue of twists that transform the displayed state into the latest
    /// state.
    twists: VecDeque<P::Twist>,
    /// Maximum number of twists in the queue (reset when queue is empty).
    queue_max: usize,
    /// Progress of the animation in the current twist, from 0.0 to 1.0.
    progress: f32,

    /// Whether the puzzle has been modified since the last time the log file
    /// was saved.
    is_unsaved: bool,

    /// Whether the puzzle has been scrambled.
    scramble_state: ScrambleState,
    /// Scrmable twists.
    scramble: Vec<P::Twist>,
    /// Undo history.
    undo_buffer: Vec<P::Twist>,
    /// Redo history.
    redo_buffer: Vec<P::Twist>,
}
impl<P: PuzzleState> Default for PuzzleController<P> {
    fn default() -> Self {
        // #[derive(Default)] doesn't work because it uses bounds that are too
        // strict (`P::Twist: Default`).
        Self {
            displayed: Default::default(),
            latest: Default::default(),
            twists: Default::default(),
            queue_max: Default::default(),
            progress: Default::default(),

            is_unsaved: Default::default(),

            scramble_state: Default::default(),
            scramble: Default::default(),
            undo_buffer: Default::default(),
            redo_buffer: Default::default(),
        }
    }
}
impl<P: PuzzleState> Eq for PuzzleController<P> {}
impl<P: PuzzleState> PartialEq for PuzzleController<P> {
    fn eq(&self, other: &Self) -> bool {
        self.latest == other.latest
    }
}
impl<P: PuzzleState> PartialEq<P> for PuzzleController<P> {
    fn eq(&self, other: &P) -> bool {
        *self.latest == *other
    }
}
impl<P: PuzzleState> PuzzleController<P> {
    /// Constructs a new PuzzleController with a solved puzzle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a twist to the back of the twist queue.
    pub fn twist(&mut self, twist: P::Twist) {
        self.is_unsaved = true;
        self.redo_buffer.clear();
        if self.undo_buffer.last() == Some(&twist.rev()) {
            self.undo();
        } else {
            self.undo_buffer.push(twist);
            self.twists.push_back(twist);
            self.latest.twist(twist);
        }
    }
    /// Returns the twist currently being animated, along with a float between
    /// 0.0 and 1.0 indicating the progress on that animation.
    pub fn current_twist(&self) -> Option<(P::Twist, f32)> {
        if let Some(&twist) = self.twists.get(0) {
            Some((twist, INTERPOLATION_FN(self.progress)))
        } else {
            None
        }
    }

    /// Returns the state of the cube that should be displayed, not including
    /// the twist currently being animated.
    pub fn displayed(&self) -> &P {
        &self.displayed
    }
    /// Returns the state of the cube after all queued twists have been applied.
    pub fn latest(&self) -> &P {
        &self.latest
    }
}

/// Methods for `PuzzleController` that do not depend on puzzle type.
#[enum_dispatch]
pub trait PuzzleControllerTrait {
    /// Returns the puzzle type.
    fn ty(&self) -> PuzzleType;

    /// Performs a twist on the puzzle.
    fn do_twist_command(
        &mut self,
        face: Face,
        direction: TwistDirection,
        layer_mask: LayerMask,
    ) -> anyhow::Result<()>;
    /// Rotates the whole puzzle to put a face in the center of the view.
    fn do_recenter_command(&mut self, face: Face) -> anyhow::Result<()>;

    /// Advances to the next frame, using the given time delta between this
    /// frame and the last. Returns whether the puzzle needs to be repainted.
    fn advance(&mut self, delta: Duration, prefs: &Preferences) -> bool;
    /// Skips the animations for all twists in the queue.
    fn catch_up(&mut self);

    /// Returns whether there is a twist to undo.
    fn has_undo(&self) -> bool;
    /// Returns whether there is a twist to redo.
    fn has_redo(&self) -> bool;
    /// Undoes one twist. Returns whether a twist was undone.
    fn undo(&mut self) -> bool;
    /// Redoes one twist. Returns whether a twist was redone.
    fn redo(&mut self) -> bool;

    /// Returns whether the puzzle has been modified since the lasts time the
    /// log file was saved.
    fn is_unsaved(&self) -> bool;

    /// Returns the model transform for a piece, based on the current animation
    /// in progress.
    fn model_transform_for_piece(&self, piece: Piece) -> Matrix4<f32>;
    /// Returns the face where the sticker at the given location belongs (i.e.
    /// corresponding to its color).
    fn get_sticker_color(&self, sticker: Sticker) -> Face;

    /// Returns the number of twists applied to the puzzle.
    fn twist_count(&self, metric: TwistMetric) -> usize;
}
impl<P: PuzzleState> PuzzleControllerTrait for PuzzleController<P> {
    fn ty(&self) -> PuzzleType {
        P::TYPE
    }

    fn do_twist_command(
        &mut self,
        face: Face,
        direction: TwistDirection,
        layer_mask: LayerMask,
    ) -> anyhow::Result<()> {
        self.twist(
            P::Twist::from_twist_command(face.try_into::<P>()?, direction.name(), layer_mask)
                .map_err(|e| anyhow!(e))?,
        );
        Ok(())
    }
    fn do_recenter_command(&mut self, face: Face) -> anyhow::Result<()> {
        self.twist(P::Twist::from_recenter_command(face.try_into::<P>()?).map_err(|e| anyhow!(e))?);
        Ok(())
    }

    fn advance(&mut self, delta: Duration, prefs: &Preferences) -> bool {
        if self.twists.is_empty() {
            self.queue_max = 0;
            // Nothing has changed, so don't request a repaint.
            return false;
        }
        if self.progress >= 1.0 {
            self.displayed.twist(self.twists.pop_front().unwrap());
            self.progress = 0.0;
            // Request repaint to finalize the twist.
            return true;
        }
        // Update queue_max.
        self.queue_max = std::cmp::max(self.queue_max, self.twists.len());
        // duration is in seconds (per one twist); speed is (fraction of twist) per frame.
        let base_speed = delta.as_secs_f32() / prefs.interaction.twist_duration;
        // Twist exponentially faster if there are/were more twists in the queue.
        let speed_mod = match prefs.interaction.dynamic_twist_speed {
            true => ((self.queue_max - 1) as f32).exp(),
            false => 1.0,
        };
        let mut twist_delta = base_speed * speed_mod;
        // Cap the twist delta at 1.0, and also handle the case where something
        // went wrong with the calculation (e.g., division by zero).
        if !(0.0..MIN_TWIST_DELTA).contains(&twist_delta) {
            twist_delta = 1.0; // Instantly complete the twist.
        }
        self.progress += twist_delta;
        if self.progress >= 1.0 {
            self.progress = 1.0;
        }
        // Request repaint.
        true
    }
    fn catch_up(&mut self) {
        for twist in self.twists.drain(..) {
            self.displayed.twist(twist);
        }
        self.progress = 0.0;
        assert_eq!(self.displayed, self.latest);
    }

    fn has_undo(&self) -> bool {
        !self.undo_buffer.is_empty()
    }
    fn has_redo(&self) -> bool {
        !self.redo_buffer.is_empty()
    }
    fn undo(&mut self) -> bool {
        if let Some(twist) = self.undo_buffer.pop() {
            self.is_unsaved = true;
            self.redo_buffer.push(twist);
            self.twists.push_back(twist.rev());
            self.latest.twist(twist.rev());
            true
        } else {
            false
        }
    }
    fn redo(&mut self) -> bool {
        if let Some(twist) = self.redo_buffer.pop() {
            self.is_unsaved = true;
            self.undo_buffer.push(twist);
            self.twists.push_back(twist);
            self.latest.twist(twist);
            true
        } else {
            false
        }
    }

    fn is_unsaved(&self) -> bool {
        self.is_unsaved
    }

    fn model_transform_for_piece(&self, piece: Piece) -> Matrix4<f32> {
        if let Some((twist, t)) = self.current_twist() {
            let p = piece.try_into::<P>().unwrap();
            if twist.affects_piece(p) {
                return twist.model_matrix(t);
            }
        }
        Matrix4::identity()
    }
    fn get_sticker_color(&self, sticker: Sticker) -> Face {
        let s = sticker.try_into::<P>().unwrap();
        self.displayed().get_sticker_color(s).into()
    }

    fn twist_count(&self, metric: TwistMetric) -> usize {
        let twists = self.undo_buffer.iter().copied();
        let prev_twists = itertools::put_back(twists.clone().map(Some)).with_value(None);

        twists
            .zip(prev_twists)
            .filter(|&(this, prev)| !this.can_combine(prev, metric))
            .count()
    }
}

impl PuzzleController<Rubiks4D> {
    /// Loads a log file and returns the puzzle state.
    pub fn load_file(path: &Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let logfile = contents.parse::<super::rubiks4d_logfile::LogFile>()?;

        let mut ret = Self {
            scramble_state: logfile.scramble_state,
            ..Self::default()
        };
        for twist in logfile.scramble_twists {
            ret.twist(twist);
        }
        ret.scramble = ret.undo_buffer;
        ret.undo_buffer = vec![];
        ret.catch_up();
        for twist in logfile.solve_twists {
            ret.twist(twist);
        }

        Ok(ret)
    }
    /// Saves the puzzle state to a log file.
    pub fn save_file(&mut self, path: &Path) -> io::Result<()> {
        let logfile = LogFile {
            scramble_state: self.scramble_state,
            view_matrix: Matrix4::identity(),
            scramble_twists: self.scramble.clone(),
            solve_twists: self.undo_buffer.clone(),
        };
        std::fs::write(path, logfile.to_string())?;
        self.is_unsaved = false;

        Ok(())
    }
}

/// Whether the puzzle has been scrambled.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScrambleState {
    /// Unscrambled.
    None = 0,
    /// Some small number of scramble twists.
    Partial = 1,
    /// Fully scrambled.
    Full = 2,
    /// Was solved by user even if not currently solved.
    Solved = 3,
}
impl Default for ScrambleState {
    fn default() -> Self {
        ScrambleState::None
    }
}
