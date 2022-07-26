//! Puzzle wrapper that adds animation and undo history functionality.

use anyhow::Result;
use cgmath::InnerSpace;
use num_enum::FromPrimitive;
use std::collections::VecDeque;
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

use super::*;
use crate::commands::PARTIAL_SCRAMBLE_MOVE_COUNT_MAX;
use crate::preferences::InteractionPreferences;
use crate::util;
use interpolate::InterpolateFn;

const TWIST_INTERPOLATION_FN: InterpolateFn = interpolate::COSINE;

/// Puzzle wrapper that adds animation and undo history functionality.
#[derive(Delegate, Debug)]
#[delegate(PuzzleType, target = "latest")]
pub struct PuzzleController {
    /// State of the puzzle right before the twist being animated right now.
    displayed: Puzzle,
    /// State of the puzzle right after the twist being animated right now, or
    /// the same as `displayed` if there is no twist animation in progress.
    next_displayed: Puzzle, // TODO: use this
    /// State of the puzzle with all twists applied to it (used for timing and
    /// undo).
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
    /// Scramble twists.
    scramble: Vec<Twist>,
    /// Undo history.
    undo_buffer: Vec<HistoryEntry>,
    /// Redo history.
    redo_buffer: Vec<HistoryEntry>,

    /// Selected pieces/stickers.
    selection: TwistSelection,
    /// Sticker that the user is hovering over.
    hovered_sticker: Option<(Sticker, [Option<Twist>; 3])>,
    /// Sticker animation states, for interpolating when the user changes the
    /// selection or hovers over a different sticker.
    sticker_animation_states: Vec<StickerDecorAnim>,

    /// Cached sticker geometry.
    cached_geometry: Option<Arc<Vec<ProjectedStickerGeometry>>>,
    cached_geometry_params: Option<StickerGeometryParams>,
}
impl Default for PuzzleController {
    fn default() -> Self {
        Self::new(PuzzleTypeEnum::default())
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
    pub fn new(ty: PuzzleTypeEnum) -> Self {
        Self {
            displayed: Puzzle::new(ty),
            next_displayed: Puzzle::new(ty),
            latest: Puzzle::new(ty),
            twist_queue: VecDeque::new(),
            queue_max: 0,
            progress: 0.0,

            is_unsaved: false,

            scramble_state: ScrambleState::None,
            scramble: vec![],
            undo_buffer: vec![],
            redo_buffer: vec![],

            selection: TwistSelection::default(),
            hovered_sticker: None,
            sticker_animation_states: vec![StickerDecorAnim::default(); ty.stickers().len()],

            cached_geometry: None,
            cached_geometry_params: None,
        }
    }
    /// Resets the puzzle.
    pub fn reset(&mut self) {
        *self = Self::new(self.ty());
    }

    /// Returns whether the puzzle has been scrambled, solved, etc..
    pub fn scramble_state(&self) -> ScrambleState {
        self.scramble_state
    }
    /// Reset and then scramble some number of moves.
    pub fn scramble_n(&mut self, n: usize) -> Result<(), &'static str> {
        self.reset();
        // Use a `while` loop instead of a `for` loop because moves may cancel.
        while self.undo_buffer.len() < n {
            self.twist(Twist::from_rng(self.ty()))?;
        }
        self.add_scramble_marker(ScrambleState::Partial);
        Ok(())
    }
    /// Reset and then scramble the puzzle completely.
    pub fn scramble_full(&mut self) -> Result<(), &'static str> {
        self.reset();
        self.scramble_n(self.scramble_moves_count())?;
        self.scramble_state = ScrambleState::Full;
        Ok(())
    }
    /// Marks the puzzle as scrambled.
    pub fn add_scramble_marker(&mut self, new_scramble_state: ScrambleState) {
        if new_scramble_state != ScrambleState::None {
            self.catch_up();
            self.scramble
                .extend(self.undo_buffer.drain(..).filter_map(HistoryEntry::twist));
            self.scramble_state = new_scramble_state;
        }
    }

    /// Adds a twist to the back of the twist queue.
    pub fn twist(&mut self, mut twist: Twist) -> Result<(), &'static str> {
        twist.layers = twist.layers & self.all_layers(); // Restrict layer mask.
        if twist.layers == LayerMask(0) {
            return Err("invalid layer mask");
        }

        self.is_unsaved = true;
        self.redo_buffer.clear();
        // Canonicalize twist.
        twist = self.canonicalize_twist(twist);
        if self.undo_buffer.last() == Some(&self.reverse_twist(twist).into()) {
            self.undo()
        } else {
            self.latest.twist(twist)?;
            self.twist_queue.push_back(twist);
            self.undo_buffer.push(twist.into());
            Ok(())
        }
    }
    /// Returns the twist currently being animated, along with a float between
    /// 0.0 and 1.0 indicating the progress on that animation.
    pub fn current_twist(&self) -> Option<(Twist, f32)> {
        if let Some(&twist) = self.twist_queue.get(0) {
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
    pub fn ty(&self) -> PuzzleTypeEnum {
        self.latest.ty()
    }

    /// Returns the puzzle selection.
    pub fn selection(&self) -> TwistSelection {
        self.selection
    }
    /// Sets the puzzle selection.
    pub fn set_selection(&mut self, selection: TwistSelection) {
        self.selection = selection;
    }

    /// Sets the hovered stickers, in order from front to back.
    pub fn update_hovered_stickers(
        &mut self,
        hovered_stickers: impl IntoIterator<Item = (Sticker, [Option<Twist>; 3])>,
    ) {
        self.hovered_sticker = hovered_stickers.into_iter().find(|&(sticker, _twists)| {
            let less_than_halfway = TWIST_INTERPOLATION_FN(self.progress) < 0.5;
            let puzzle_state = if less_than_halfway {
                self.displayed() // puzzle state before the twist
            } else {
                &self.next_displayed // puzzle state after the twist
            };
            self.selection.has_sticker(puzzle_state, sticker)
        });
    }
    pub(crate) fn hovered_sticker(&self) -> Option<Sticker> {
        self.hovered_sticker.map(|(sticker, _twists)| sticker)
    }
    pub(crate) fn hovered_sticker_twists(&self) -> [Option<Twist>; 3] {
        self.hovered_sticker
            .map(|(_sticker, twists)| twists)
            .unwrap_or([None; 3])
    }

    /// Returns the animation state for a sticker.
    pub fn sticker_animation_state(&self, sticker: Sticker) -> StickerDecorAnim {
        self.sticker_animation_states[sticker.0 as usize]
    }

    pub(crate) fn geometry(
        &mut self,
        params: StickerGeometryParams,
    ) -> Arc<Vec<ProjectedStickerGeometry>> {
        if self.cached_geometry_params != Some(params) {
            // Invalidate the cache.
            self.cached_geometry = None;
        }

        self.cached_geometry_params = Some(params);

        let ret = self.cached_geometry.take().unwrap_or_else(|| {
            log::trace!("Regenerating puzzle geometry");

            // Project stickers.
            let mut sticker_geometries: Vec<ProjectedStickerGeometry> = vec![];
            for sticker in (0..self.stickers().len() as _).map(Sticker) {
                // Compute geometry, including vertex positions before 3D
                // perspective projection.
                let sticker_geom = match self.displayed().sticker_geometry(sticker, params) {
                    Some(s) => s,
                    None => continue, // invisible; skip this sticker
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

                for (indices, twists) in sticker_geom
                    .polygon_indices
                    .iter()
                    .zip(sticker_geom.polygon_twists)
                {
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
                            twists,
                        ));
                    } else {
                        // This polygon is back-facing.
                        let illumination = 0.0; // don't care
                        projected_back_polygons.push(geometry::polygon_from_indices(
                            &projected_verts,
                            indices,
                            illumination,
                            [None; 3],
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
            || (0..self.ty().stickers().len() as _)
                .map(Sticker)
                .map(|sticker| self.sticker_animation_state_target(sticker, prefs))
                .ne(self.sticker_animation_states.iter().copied())
    }

    /// Advances the puzzle geometry and internal state to the next frame, using
    /// the given time delta between this frame and the last.
    pub fn update_geometry(&mut self, delta: Duration, prefs: &InteractionPreferences) {
        if self.twist_queue.is_empty() {
            self.queue_max = 0;
            return;
        }
        if self.progress == 0.0 {
            self.next_displayed = self.displayed().clone();
            self.next_displayed
                .twist(*self.twist_queue.front().unwrap())
                .expect("failed to apply twist from twist queue");
        }

        // Update queue_max.
        self.queue_max = std::cmp::max(self.queue_max, self.twist_queue.len());
        // duration is in seconds (per one twist); speed is (fraction of twist) per frame.
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
            self.twist_queue.pop_front();
            self.displayed = self.next_displayed.clone();
            self.progress = 0.0;

            // The puzzle state has changed, so invalidate the geometry cache.
            self.cached_geometry = None;
        }
    }
    /// Advances the puzzle decorations (outlines and sticker opacities) to the
    /// next frame, using the given time delta between this frame and the last.
    pub fn update_decorations(&mut self, delta: Duration, prefs: &InteractionPreferences) {
        let max_delta_selected = delta.as_secs_f32() / prefs.selection_fade_duration;
        let max_delta_hovered = delta.as_secs_f32() / prefs.hover_fade_duration;

        for i in 0..self.stickers().len() {
            let target = self.sticker_animation_state_target(Sticker(i as _), prefs);
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
        let is_selected = self.selection.has_sticker(self.latest(), sticker);
        let is_hovered = match self.hovered_sticker {
            Some((s, _)) => match prefs.highlight_piece_on_hover {
                false => s == sticker,
                true => self.info(s).piece == self.info(sticker).piece,
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
        if let Some(entry) = self.undo_buffer.pop() {
            self.is_unsaved = true;
            match entry {
                HistoryEntry::Twist(twist) => {
                    let rev = self.reverse_twist(twist);
                    self.latest.twist(rev)?;
                    self.twist_queue.push_back(rev);
                }
            }
            self.redo_buffer.push(entry);
            Ok(())
        } else {
            Err("Nothing to undo")
        }
    }
    /// Redoes one twist. Returns an error if there was nothing to redo or the
    /// twist could not be applied to the puzzle.
    pub fn redo(&mut self) -> Result<(), &'static str> {
        if let Some(entry) = self.redo_buffer.pop() {
            self.is_unsaved = true;
            match entry {
                HistoryEntry::Twist(twist) => {
                    self.latest.twist(twist)?;
                    self.twist_queue.push_back(twist);
                }
            }
            self.undo_buffer.push(entry);
            Ok(())
        } else {
            Err("Nothing to redo")
        }
    }

    /// Marks the puzzle as saved
    pub fn mark_saved(&mut self) {
        self.is_unsaved = false;
    }
    /// Returns whether the puzzle has been modified since the lasts time it was
    /// marked as saved.
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
                self.scramble.len() >= self.scramble_moves_count()
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

    /// Returns the number of twists applied to the puzzle, not including the scramble.
    pub fn twist_count(&self, metric: TwistMetric) -> usize {
        metric.count_twists(
            self,
            self.undo_buffer
                .iter()
                .copied()
                .filter_map(HistoryEntry::twist),
        )
    }
    /// Returns the moves used to scramble the puzzle.
    pub fn scramble(&self) -> &[Twist] {
        &self.scramble
    }
    /// Returns the twists and other actions applied to the puzzle, not
    /// including the scramble.
    pub fn undo_buffer(&self) -> &[HistoryEntry] {
        &self.undo_buffer
    }
    /// Returns the twists and other actions in the redo buffer.
    pub fn redo_buffer(&self) -> &[HistoryEntry] {
        &self.redo_buffer
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum HistoryEntry {
    Twist(Twist),
}
impl From<Twist> for HistoryEntry {
    fn from(twist: Twist) -> Self {
        Self::Twist(twist)
    }
}
impl HistoryEntry {
    pub fn twist(self) -> Option<Twist> {
        match self {
            HistoryEntry::Twist(twist) => Some(twist),
        }
    }
    pub fn to_string(self, notation: &NotationScheme) -> String {
        match self {
            HistoryEntry::Twist(twist) => notation.twist_to_string(twist),
        }
    }
}

/// Whether the puzzle has been scrambled.
#[derive(FromPrimitive, Debug, Default, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum ScrambleState {
    /// Unscrambled.
    #[default]
    None = 0,
    /// Some small number of scramble twists.
    Partial = 1,
    /// Fully scrambled.
    Full = 2,
    /// Was solved by user even if not currently solved.
    Solved = 3,
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
