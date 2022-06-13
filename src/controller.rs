//! Puzzle wrapper that adds animation and undo history functionality.

use anyhow::{anyhow, bail};
use cgmath::{InnerSpace, Matrix4, SquareMatrix};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
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

use crate::commands::PARTIAL_SCRAMBLE_MOVE_COUNT_MAX;
use crate::mc4d_compat;
use crate::preferences::InteractionPreferences;
use crate::puzzle::{
    geometry, traits::*, Piece, ProjectedStickerGeometry, Puzzle, PuzzleType, Rubiks34, Selection,
    Sticker, StickerGeometryParams, Twist, TwistMetric,
};
use crate::util;
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
    /// Sticker that the user is hovering over.
    hovered_sticker: Option<Sticker>,
    /// Sticker animation states, for interpolating when the user changes the
    /// selection or hovers over a different sticker.
    sticker_animation_states: Vec<StickerDecorAnim>,

    /// Cached sticker geometry.
    cached_geometry: Option<Arc<Vec<ProjectedStickerGeometry>>>,
    cached_geometry_params: Option<StickerGeometryParams>,

    /// 0.0 for virtual view, 1.0 for physical view, and intermediate values
    /// interpolate.
    view_mode: f32,
    pub target_view_mode: f32,
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
            hovered_sticker: None,
            sticker_animation_states: vec![StickerDecorAnim::default(); ty.stickers().len()],

            cached_geometry: None,
            cached_geometry_params: None,

            view_mode: 0.0,
            target_view_mode: 1.0,
        }
    }
    /// Resets the puzzle.
    pub fn reset(&mut self) {
        *self = Self::new(self.ty());
    }

    /// Scramble some small number of moves.
    pub fn scramble_n(&mut self, n: usize) -> Result<(), &'static str> {
        self.reset();
        // Use a `while` loop instead of a `for` loop because moves may cancel.
        while self.undo_buffer.len() < n {
            self.twist(Twist::from_rng(self.ty()))?;
        }
        self.catch_up();
        self.scramble = std::mem::replace(&mut self.undo_buffer, vec![]);
        self.scramble_state = ScrambleState::Partial;
        Ok(())
    }
    /// Scramble the puzzle completely.
    pub fn scramble_full(&mut self) -> Result<(), &'static str> {
        self.reset();
        self.scramble_n(self.ty().scramble_moves_count())?;
        self.scramble_state = ScrambleState::Full;
        Ok(())
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

    /// Returns the puzzle selection.
    pub fn selection(&self) -> Selection {
        self.selection
    }
    /// Sets the puzzle selection.
    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = selection;
    }

    /// Sets the hovered stickers, in order from front to back.
    pub fn update_hovered_stickers(&mut self, hovered_stickers: impl IntoIterator<Item = Sticker>) {
        self.hovered_sticker = hovered_stickers
            .into_iter()
            .filter(|&sticker| {
                let sticker_mid_twist = match self.current_twist() {
                    // Use the selection after the twist.
                    Some((twist, t)) if t > 0.5 => twist.destination_sticker(sticker),
                    // Use the selection before the twist.
                    _ => sticker,
                };
                self.selection.has_sticker(sticker_mid_twist)
            })
            .next();
    }
    pub(crate) fn hovered_sticker(&self) -> Option<Sticker> {
        self.hovered_sticker
    }

    /// Returns the animation state for a sticker.
    pub fn sticker_animation_state(&self, sticker: Sticker) -> StickerDecorAnim {
        match self.current_twist() {
            None => self.sticker_animation_states[sticker.id()],
            Some((twist, t)) => {
                // Interpolate selected state between old and new sticker
                // positions.
                let old = self.sticker_animation_states[sticker.id()];
                let new = self.sticker_animation_states[twist.destination_sticker(sticker).id()];
                StickerDecorAnim {
                    selected: old.selected * (1.0 - t) + new.selected * t,
                    hovered: old.hovered,
                }
            }
        }
    }

    pub(crate) fn geometry(
        &mut self,
        mut params: StickerGeometryParams,
    ) -> Arc<Vec<ProjectedStickerGeometry>> {
        params.model_transform = Matrix4::identity();
        params.view_mode = self.view_mode;
        if self.cached_geometry_params != Some(params) {
            // Invalidate the cache.
            self.cached_geometry = None;
        }

        self.cached_geometry_params = Some(params);

        let ret = self.cached_geometry.take().unwrap_or_else(|| {
            log::trace!("Regenerating puzzle geometry");

            // Project stickers.
            let mut sticker_geometries: Vec<ProjectedStickerGeometry> = vec![];
            for piece in self.displayed().pieces() {
                params.model_transform = self.model_transform_for_piece(*piece);

                for sticker in piece.stickers() {
                    // Compute geometry, including vertex positions before 3D
                    // perspective projection.
                    let sticker_geom = match sticker.geometry(params) {
                        Some(s) => s,
                        None => continue, // behind camera; skip this sticker
                    };

                    // Compute vertex positions after 3D perspective projection.
                    let projected_verts = match sticker_geom
                        .verts
                        .iter()
                        .map(|&v| params.project_3d(v))
                        .collect::<Option<Vec<_>>>()
                    {
                        Some(s) => s,
                        None => continue, // behind camera; skip this sticker
                    };

                    let mut projected_front_polygons = vec![];
                    let mut projected_back_polygons = vec![];

                    for indices in &sticker_geom.polygon_indices {
                        let projected_normal =
                            geometry::polygon_normal_from_indices(&projected_verts, indices);
                        if projected_normal.z > 0.0 {
                            // This polygon is front-facing.
                            let lighting_normal =
                                geometry::polygon_normal_from_indices(&sticker_geom.verts, indices)
                                    .normalize();
                            let illumination =
                                params.ambient_light + lighting_normal.dot(params.light_vector);
                            projected_front_polygons.push(geometry::polygon_from_indices(
                                &projected_verts,
                                indices,
                                illumination,
                            ));
                        } else {
                            // This polygon is back-facing.
                            let illumination = 0.0; // don't care
                            projected_back_polygons.push(geometry::polygon_from_indices(
                                &projected_verts,
                                indices,
                                illumination,
                            ));
                        }
                    }

                    let (min_bound, max_bound) = util::min_and_max_bound(&projected_verts);

                    sticker_geometries.push(ProjectedStickerGeometry {
                        sticker,

                        verts: projected_verts.into_boxed_slice(),
                        min_bound,
                        max_bound,

                        front_polygons: projected_front_polygons.into_boxed_slice(),
                        back_polygons: projected_back_polygons.into_boxed_slice(),
                    });
                }
            }

            // Sort stickers by depth.
            geometry::sort_by_depth(&mut sticker_geometries);

            Arc::new(sticker_geometries)
        });

        self.cached_geometry = Some(Arc::clone(&ret));
        ret
    }

    /// Returns whether the puzzle is in the middle of an animation.
    pub fn is_animating(&self, prefs: &InteractionPreferences) -> bool {
        self.current_twist().is_some()
            || self
                .ty()
                .stickers()
                .iter()
                .map(|&sticker| self.sticker_animation_state_target(sticker, prefs))
                .ne(self.sticker_animation_states.iter().copied())
            || self.view_mode != self.target_view_mode
    }

    /// Advances the puzzle geometry and internal state to the next frame, using
    /// the given time delta between this frame and the last.
    pub fn update_geometry(&mut self, delta: Duration, prefs: &InteractionPreferences) {
        // if self.view_mode != self.target_view_mode && self.twist_queue.is_empty()
        //     || self.view_mode.fract() != 0.0
        // {
        add_delta_toward_target(
            &mut self.view_mode,
            self.target_view_mode,
            delta.as_secs_f32() / prefs.twist_duration,
        );
        //     return;
        // }

        if self.twist_queue.is_empty() {
            self.queue_max = 0;
            return;
        }

        // Invalidate the geometry cache.
        self.cached_geometry = None;

        if self.progress >= 1.0 {}
        // Update queue_max.
        self.queue_max = std::cmp::max(self.queue_max, self.twist_queue.len());
        // `twist_duration` is in seconds (per one twist); `base_speed` is
        // (fraction of) twists per frame.
        let base_speed = delta.as_secs_f32() / prefs.twist_duration;
        // Twist exponentially faster if there are/were more twists in the queue.
        let speed_mod = match prefs.dynamic_twist_speed {
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

            let twist = self.twist_queue.pop_front().unwrap();

            // Shuffle sticker hover states as necessary.
            let mut hover_states = vec![];
            for (i, state) in self.sticker_animation_states.iter_mut().enumerate() {
                if state.hovered != 0.0 {
                    hover_states.push((i, state.hovered));
                    state.hovered = 0.0;
                }
            }
            for (i, hovered) in hover_states {
                let sticker = Sticker::from_id(self.ty(), i).unwrap();
                self.sticker_animation_states[twist.destination_sticker(sticker).id()].hovered =
                    hovered;
            }

            self.displayed
                .twist(twist)
                .expect("failed to apply twist from twist queue");
            self.progress = 0.0;
        }
    }
    /// Advances the puzzle decorations (outlines and sticker opacities) to the
    /// next frame, using the given time delta between this frame and the last.
    pub fn update_decorations(&mut self, delta: Duration, prefs: &InteractionPreferences) {
        let max_delta_selected = delta.as_secs_f32() / prefs.selection_fade_duration;
        let max_delta_hovered = delta.as_secs_f32() / prefs.hover_fade_duration;

        for (i, &sticker) in self.ty().stickers().iter().enumerate() {
            let target = self.sticker_animation_state_target(sticker, prefs);
            let animation_state = &mut self.sticker_animation_states[i];
            add_delta_toward_target(
                &mut animation_state.selected,
                target.selected,
                max_delta_selected,
            );
            if target.hovered == 1.0 {
                // Always react instantly to a new hovered sticker.
                animation_state.hovered = 1.0;
            } else {
                add_delta_toward_target(
                    &mut animation_state.hovered,
                    target.hovered,
                    max_delta_hovered,
                );
            }
        }
    }
    fn sticker_animation_state_target(
        &self,
        sticker: Sticker,
        prefs: &InteractionPreferences,
    ) -> StickerDecorAnim {
        let is_selected = self.selection.has_sticker(sticker);
        let is_hovered = match self.hovered_sticker {
            Some(s) => match prefs.highlight_piece_on_hover {
                false => s == sticker,
                true => s.piece() == sticker.piece(),
            },
            None => false,
        };

        StickerDecorAnim {
            selected: if is_selected { 1.0 } else { 0.0 },
            hovered: if is_hovered { 1.0 } else { 0.0 },
        }
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
    /// Returns whether the puzzle has been fully scrambled, even if it has been solved.
    pub fn has_been_fully_scrambled(&self) -> bool {
        match self.scramble_state {
            ScrambleState::None => false,
            ScrambleState::Partial => false,
            ScrambleState::Full => true,
            ScrambleState::Solved => {
                self.scramble.len() >= self.ty().scramble_moves_count()
                    || self.scramble.len() > PARTIAL_SCRAMBLE_MOVE_COUNT_MAX
            }
        }
    }
    /// Returns whether the puzzle has been solved at some point.
    pub fn has_been_solved(&self) -> bool {
        self.scramble_state == ScrambleState::Solved
    }
    /// Returns whether the puzzle is currently in a solved configuration.
    pub fn is_solved(&self) -> bool {
        self.displayed.is_solved()
    }
    /// Checks whether the puzzle was scrambled and is now solved. If so,
    /// updates the scramble state, and returns `true`.
    pub fn check_just_solved(&mut self) -> bool {
        let has_been_scrambled = matches!(
            self.scramble_state,
            ScrambleState::Partial | ScrambleState::Full,
        );
        if has_been_scrambled && self.is_solved() {
            self.scramble_state = ScrambleState::Solved;
            true
        } else {
            false
        }
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
            displayed: Rubiks34::new().into(),
            latest: Rubiks34::new().into(),

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
            Puzzle::Rubiks34(_) => {
                let logfile = mc4d_compat::LogFile {
                    scramble_state: self.scramble_state,
                    view_matrix: Matrix4::identity(),
                    scramble_twists: self
                        .scramble
                        .iter()
                        .map(|t| t.unwrap::<Rubiks34>())
                        .collect(),
                    solve_twists: self
                        .undo_buffer
                        .iter()
                        .map(|t| t.unwrap::<Rubiks34>())
                        .collect(),
                };
                std::fs::write(path, logfile.to_string())?;
                self.is_unsaved = false;

                Ok(())
            }
            // TODO: support log files for 2^4 (and 3D puzzles as well)
            _ => bail!("log files only supported for 3^4"),
        }
    }

    /// Toggles between virtual and physical view of the 2^4 puzzle.
    pub fn switch_view(&mut self) -> Result<(), &'static str> {
        if self.ty() == PuzzleType::Rubiks24 {
            self.target_view_mode = 1.0 - self.target_view_mode;
            Ok(())
        } else {
            Err("only 2x2x2x2 has an alternate view")
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

/// Sticker decoration animation state. Each value is in the range 0.0..=1.0.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct StickerDecorAnim {
    /// Progress toward being selected.
    pub selected: f32,
    /// Progress toward being hovered.
    pub hovered: f32,
}
impl Default for StickerDecorAnim {
    fn default() -> Self {
        Self {
            selected: 1.0,
            hovered: 0.0,
        }
    }
}

fn add_delta_toward_target(current: &mut f32, target: f32, delta: f32) {
    if *current == target {
        // fast exit for the common case
    } else if !delta.is_finite() {
        *current = target;
    } else if *current + delta < target {
        *current += delta;
    } else if *current - delta > target {
        *current -= delta;
    } else {
        *current = target;
    }
}
