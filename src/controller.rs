//! Puzzle wrapper that adds animation and undo history functionality.

use anyhow::{anyhow, bail};
use cgmath::{Matrix4, SquareMatrix};
use std::collections::VecDeque;
use std::path::Path;
use std::time::Duration;

/// If at least this much of a twist is animated in one frame, just skip the
/// animation to reduce unnecessary flashing.
const MIN_TWIST_DELTA: f32 = 1.0 / 3.0;

/// Higher number means faster exponential increase in twist speed.
const EXP_TWIST_FACTOR: f32 = 0.5;

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

use crate::mc4d_compat;
use crate::preferences::Preferences;
use crate::puzzle::{
    traits::*, Face, LayerMask, Piece, Puzzle, PuzzleType, Rubiks4D, Selection, Sticker, Twist,
    TwistDirection, TwistMetric,
};
use interpolate::InterpolateFn;

const TWIST_INTERPOLATION_FN: InterpolateFn = interpolate::COSINE;

/// Puzzle wrapper that adds animation and undo history functionality.
#[derive(Debug)]
pub struct PuzzleController {
    /// State of the puzzle right before the twist being animated right now.
    ///
    /// `Box`ed so that this struct is always the same size.
    displayed: Puzzle,
    /// State of the puzzle with all twists applied to it (used for timing
    /// and undo).
    ///
    /// `Box`ed so that this struct is always the same size.
    latest: Puzzle,
    /// Queue of twists that transform the displayed state into the latest
    /// state.
    twist_queue: VecDeque<Twist>,
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
    scramble: Vec<Twist>,
    /// Undo history.
    undo_buffer: Vec<Twist>,
    /// Redo history.
    redo_buffer: Vec<Twist>,

    /// Selected pieces/stickers.
    selection: Selection,
    /// Displayed alpha values for pieces.
    sticker_alphas: Vec<f32>,
}
impl Default for PuzzleController {
    fn default() -> Self {
        Self::new(PuzzleType::default())
    }
}
impl Eq for PuzzleController {}
impl PartialEq for PuzzleController {
    fn eq(&self, other: &Self) -> bool {
        self.latest == other.latest
    }
}
impl PartialEq<Puzzle> for PuzzleController {
    fn eq(&self, other: &Puzzle) -> bool {
        self.latest == *other
    }
}
impl PuzzleController {
    /// Constructs a new PuzzleController with a solved puzzle.
    pub fn new(ty: PuzzleType) -> Self {
        Self {
            displayed: Puzzle::new(ty),
            latest: Puzzle::new(ty),
            twist_queue: VecDeque::new(),
            queue_max: 0,
            progress: 0.0,

            is_unsaved: false,

            scramble_state: ScrambleState::None,
            scramble: vec![],
            undo_buffer: vec![],
            redo_buffer: vec![],

            selection: Selection::default(),
            sticker_alphas: vec![1.0; ty.stickers().len()],
        }
    }

    /// Adds a twist to the back of the twist queue.
    pub fn twist(&mut self, twist: Twist) -> Result<(), &'static str> {
        if twist.ty() != self.ty() {
            return Err("puzzle type mismatch");
        }
        self.is_unsaved = true;
        self.redo_buffer.clear();
        if self.undo_buffer.last() == Some(&twist.rev()) {
            self.undo()
        } else {
            self.latest.twist(twist.clone())?;
            self.twist_queue.push_back(twist.clone());
            self.undo_buffer.push(twist);
            Ok(())
        }
    }
    /// Returns the twist currently being animated, along with a float between
    /// 0.0 and 1.0 indicating the progress on that animation.
    pub fn current_twist(&self) -> Option<(&Twist, f32)> {
        if let Some(twist) = self.twist_queue.get(0) {
            Some((twist, TWIST_INTERPOLATION_FN(self.progress)))
        } else {
            None
        }
    }

    /// Returns the state of the cube that should be displayed, not including
    /// the twist currently being animated.
    pub fn displayed(&self) -> &Puzzle {
        &self.displayed
    }
    /// Returns the state of the cube after all queued twists have been applied.
    pub fn latest(&self) -> &Puzzle {
        &self.latest
    }

    /// Returns the puzzle type.
    pub fn ty(&self) -> PuzzleType {
        self.latest.ty()
    }

    /// Performs a twist on the puzzle.
    pub fn do_twist_command(
        &mut self,
        face: Face,
        direction: TwistDirection,
        layer_mask: LayerMask,
    ) -> Result<(), &'static str> {
        self.twist(Twist::from_face_with_layers(
            face,
            direction.name(),
            layer_mask,
        )?)
    }
    /// Rotates the whole puzzle to put a face in the center of the view.
    pub fn do_recenter_command(&mut self, face: Face) -> Result<(), &'static str> {
        self.twist(Twist::from_face_recenter(face)?)
    }

    /// Sets the puzzle selection.
    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = selection;
    }
    /// Returns the opacity for a sticker.
    pub fn sticker_alpha(&self, sticker: Sticker) -> f32 {
        if let Some((twist, t)) = self.current_twist() {
            if twist.affects_piece(sticker.piece()) {
                let start = sticker;
                let end = twist.destination_sticker(start);
                return t * self.sticker_alphas[end.id()]
                    + (1.0 - t) * self.sticker_alphas[start.id()];
            }
        }
        self.sticker_alphas[sticker.id()]
    }

    /// Advances to the next frame, using the given time delta between this
    /// frame and the last. Returns whether the puzzle needs to be repainted.
    pub fn advance(&mut self, delta: Duration, prefs: &Preferences) -> bool {
        // Note that we can't just use `||` because that will short-circuit.
        let mut wants_repaint = false;
        wants_repaint |= self.advance_twist(delta, prefs);
        wants_repaint |= self.advance_alpha(delta, prefs);
        wants_repaint
    }

    fn advance_twist(&mut self, delta: Duration, prefs: &Preferences) -> bool {
        if self.twist_queue.is_empty() {
            self.queue_max = 0;
            // Nothing has changed, so don't request a repaint.
            return false;
        }
        if self.progress >= 1.0 {
            self.displayed
                .twist(self.twist_queue.pop_front().unwrap())
                .expect("failed to apply twist from twist queue");
            self.progress = 0.0;
            // Request repaint to finalize the twist.
            return true;
        }
        // Update queue_max.
        self.queue_max = std::cmp::max(self.queue_max, self.twist_queue.len());
        // duration is in seconds (per one twist); speed is (fraction of twist) per frame.
        let base_speed = delta.as_secs_f32() / prefs.interaction.twist_duration;
        // Twist exponentially faster if there are/were more twists in the queue.
        let speed_mod = match prefs.interaction.dynamic_twist_speed {
            true => ((self.twist_queue.len() - 1) as f32 * EXP_TWIST_FACTOR).exp(),
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
    fn advance_alpha(&mut self, delta: Duration, prefs: &Preferences) -> bool {
        let mut wants_repaint = false;

        let max_delta_alpha = delta.as_secs_f32() / prefs.interaction.fade_duration;
        for (sticker, alpha) in self.ty().stickers().iter().zip(&mut self.sticker_alphas) {
            let target = prefs.colors.sticker_opacity
                * if self.selection.has_sticker(*sticker) {
                    1.0
                } else {
                    prefs.colors.hidden_opacity
                };

            let diff = target - *alpha;

            if diff == 0.0 {
                continue;
            }
            wants_repaint = true;
            if diff.abs() <= max_delta_alpha {
                *alpha = target;
            } else {
                *alpha += max_delta_alpha.copysign(diff);
            }
        }

        wants_repaint
    }

    /// Skips the animations for all twists in the queue.
    pub fn catch_up(&mut self) {
        for twist in self.twist_queue.drain(..) {
            self.displayed
                .twist(twist)
                .expect("failed to apply twist from twist queue");
        }
        self.progress = 0.0;
        assert_eq!(self.displayed, self.latest);
    }

    /// Returns whether there is a twist to undo.
    pub fn has_undo(&self) -> bool {
        !self.undo_buffer.is_empty()
    }

    /// Returns whether there is a twist to redo.
    pub fn has_redo(&self) -> bool {
        !self.redo_buffer.is_empty()
    }

    /// Undoes one twist. Returns an error if there was nothing to undo or the
    /// twist could not be applied to the puzzle.
    pub fn undo(&mut self) -> Result<(), &'static str> {
        if let Some(twist) = self.undo_buffer.pop() {
            self.is_unsaved = true;
            self.latest.twist(twist.rev())?;
            self.twist_queue.push_back(twist.rev());
            self.redo_buffer.push(twist);
            Ok(())
        } else {
            Err("Nothing to undo")
        }
    }

    /// Redoes one twist. Returns an error if there was nothing to redo or the
    /// twist could not be applied to the puzzle.
    pub fn redo(&mut self) -> Result<(), &'static str> {
        if let Some(twist) = self.redo_buffer.pop() {
            self.is_unsaved = true;
            self.latest.twist(twist.clone())?;
            self.twist_queue.push_back(twist.clone());
            self.undo_buffer.push(twist);
            Ok(())
        } else {
            Err("Nothing to redo")
        }
    }

    /// Returns whether the puzzle has been modified since the lasts time the
    /// log file was saved.
    pub fn is_unsaved(&self) -> bool {
        self.is_unsaved
    }

    /// Returns the model transform for a piece, based on the current animation
    /// in progress.
    pub fn model_transform_for_piece(&self, piece: Piece) -> Matrix4<f32> {
        if let Some((twist, t)) = self.current_twist() {
            if twist.affects_piece(piece) {
                return twist.model_transform(t);
            }
        }
        Matrix4::identity()
    }

    /// Returns the number of twists applied to the puzzle.
    pub fn twist_count(&self, metric: TwistMetric) -> usize {
        let twists = self.undo_buffer.iter().cloned();
        let prev_twists = itertools::put_back(twists.clone().map(Some)).with_value(None);

        twists
            .zip(prev_twists)
            .filter(|(this, prev)| !this.can_combine(prev.as_ref(), metric))
            .count()
    }

    /// Loads a log file and returns the puzzle state.
    pub fn load_file(path: &Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let logfile = contents.parse::<mc4d_compat::LogFile>()?;

        let mut ret = Self {
            displayed: Rubiks4D::new().into(),
            latest: Rubiks4D::new().into(),

            scramble_state: logfile.scramble_state,

            ..Self::default()
        };
        for twist in logfile.scramble_twists {
            ret.twist(twist.into()).map_err(|e| anyhow!(e))?;
        }
        ret.scramble = ret.undo_buffer;
        ret.undo_buffer = vec![];
        ret.catch_up();
        for twist in logfile.solve_twists {
            ret.twist(twist.into()).map_err(|e| anyhow!(e))?;
        }

        Ok(ret)
    }

    /// Saves the puzzle state to a log file.
    pub fn save_file(&mut self, path: &Path) -> anyhow::Result<()> {
        match self.latest {
            Puzzle::Rubiks3D(_) => bail!("log files only supported for Rubik's 4D"),
            Puzzle::Rubiks4D(_) => {
                let logfile = mc4d_compat::LogFile {
                    scramble_state: self.scramble_state,
                    view_matrix: Matrix4::identity(),
                    scramble_twists: self
                        .scramble
                        .iter()
                        .map(|t| t.unwrap::<Rubiks4D>())
                        .collect(),
                    solve_twists: self
                        .undo_buffer
                        .iter()
                        .map(|t| t.unwrap::<Rubiks4D>())
                        .collect(),
                };
                std::fs::write(path, logfile.to_string())?;
                self.is_unsaved = false;

                Ok(())
            }
        }
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
