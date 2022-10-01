//! Puzzle wrapper that adds animation and undo history functionality.

use anyhow::Result;
use bitvec::bitvec;
use bitvec::slice::BitSlice;
use bitvec::vec::BitVec;
use cgmath::InnerSpace;
use ndpuzzle::math::Rotor;
use num_enum::FromPrimitive;
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque};
use std::ops::{BitOr, BitOrAssign};
use std::sync::Arc;
use std::time::Duration;

/// If at least this much of a twist is animated in one frame, just skip the
/// animation to reduce unnecessary flashing.
const MIN_TWIST_DELTA: f32 = 1.0 / 3.0;

/// Higher number means faster exponential increase in twist speed.
const EXP_TWIST_FACTOR: f32 = 0.5;

/// Higher number means slower exponential decay of view angle offset.
const VIEW_ANGLE_OFFSET_DECAY_RATE: f32 = 0.02_f32;

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
use crate::preferences::{InteractionPreferences, Preferences, ViewPreferences};
use crate::util;
use interpolate::InterpolateFn;

const TWIST_INTERPOLATION_FN: InterpolateFn = interpolate::COSINE;

/// Puzzle wrapper that adds animation and undo history functionality.
#[derive(Debug)]
pub struct PuzzleController {
    /// Latest puzzle state, not including any transient rotation.
    puzzle: Box<dyn PuzzleState>,
    /// Twist animation state.
    twist_anim: TwistAnimationState,
    /// View settings animation state.
    view_settings_anim: ViewSettingsAnimState,
    /// View angle animation state.
    view_angle: ViewAngleAnimState,

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

    /// Sticker that the user is hovering over.
    hovered_sticker: Option<Sticker>,
    /// Twists from the hovered sticker.
    hovered_twists: Option<ClickTwists>,

    /// Grip, which controls which pieces will be twisted.
    grip: Grip,
    /// Set of selected stickers.
    selection: HashSet<Sticker>,
    /// Set of non-hidden pieces.
    visible_pieces: BitVec,
    /// Set of non-hidden pieces to preview when hovering over a piece filter
    /// button.
    visible_pieces_preview: Option<BitVec>,
    /// Opacity of hidden pieces preview when hovering over a piece filter
    /// buton.
    hidden_pieces_preview_opacity: Option<f32>,

    /// Piece states, such as whether a piece is hidden. All values are
    /// represented as `f32` for animation.
    visual_piece_states: Vec<VisualPieceState>,

    /// Cached sticker geometry.
    cached_geometry: Option<Arc<Vec<ProjectedStickerGeometry>>>,
    cached_geometry_params: Option<StickerGeometryParams>,
}
impl Default for PuzzleController {
    fn default() -> Self {
        Self::new(
            PUZZLE_REGISTRY
                .lock()
                .get(crate::DEFAULT_PUZZLE)
                .expect("No default puzzle"),
        )
    }
}
impl PuzzleController {
    /// Constructs a new PuzzleController with a solved puzzle.
    pub fn new(ty: &PuzzleType) -> Self {
        Self {
            puzzle: ty.new(),
            twist_anim: TwistAnimationState::default(),
            view_settings_anim: ViewSettingsAnimState::default(),
            view_angle: ViewAngleAnimState::default(),

            is_unsaved: false,

            scramble_state: ScrambleState::None,
            scramble: vec![],
            undo_buffer: vec![],
            redo_buffer: vec![],

            hovered_sticker: None,
            hovered_twists: None,

            grip: Grip::default(),
            selection: HashSet::new(),
            visible_pieces: bitvec![1; ty.pieces.len()],
            visible_pieces_preview: None,
            hidden_pieces_preview_opacity: None,

            visual_piece_states: vec![VisualPieceState::default(); ty.pieces.len()],

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

        // Set a reasonable limit on the number of moves.
        const MAX_SCRAMBLE_LEN: usize = 10_000;
        if n > MAX_SCRAMBLE_LEN {
            return Err("Cannot scramble more than 10,000 moves");
        }

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
        self.scramble_n(self.ty().scramble_moves_count)?;
        self.scramble_state = ScrambleState::Full;
        Ok(())
    }
    /// Marks the puzzle as scrambled.
    pub fn add_scramble_marker(&mut self, new_scramble_state: ScrambleState) {
        self.skip_twist_animations();
        self.scramble
            .extend(self.undo_buffer.drain(..).filter_map(HistoryEntry::twist));
        if new_scramble_state == ScrambleState::None {
            // This is technically invalid? But I've seen some older MC4D log files that do this, so just assume it's a full scramble.
            self.scramble_state = ScrambleState::Full;
        } else {
            self.scramble_state = new_scramble_state;
        }
    }

    /// Adds a twist to the back of the twist queue.
    pub fn twist(&mut self, twist: Twist) -> Result<(), &'static str> {
        self._twist(twist, true)
    }
    /// Adds a twist to the back of the twist queue. Does not cancel adjacent
    /// twists.
    pub fn twist_no_collapse(&mut self, twist: Twist) -> Result<(), &'static str> {
        self._twist(twist, false)
    }
    fn _twist(&mut self, mut twist: Twist, collapse: bool) -> Result<(), &'static str> {
        twist.layers &= self.ty().all_layers(); // Restrict layer mask.
        if twist.layers == LayerMask(0) {
            return Err("invalid layer mask");
        }

        self.is_unsaved = true;
        self.redo_buffer.clear();
        // Canonicalize twist.
        twist = self.ty().canonicalize_twist(twist);
        if collapse && self.undo_buffer.last() == Some(&self.ty().reverse_twist(twist).into()) {
            // This twist is the reverse of the last one, so just undo the last
            // one.
            self.undo()
        } else {
            self.animate_twist(twist)?;
            self.undo_buffer.push(twist.into());
            Ok(())
        }
    }
    /// Applies a twist to the puzzle and queues it for animation. Does _not_
    /// handle undo/redo stack or `is_unsaved`.
    fn animate_twist(&mut self, twist: Twist) -> Result<(), &'static str> {
        let old_state = self.puzzle.clone();
        self.puzzle.twist(twist)?;
        self.twist_anim.queue.push_back(TwistAnimation {
            state: old_state,
            twist,
        });

        // Invalidate the cache.
        self.cached_geometry = None;

        Ok(())
    }
    /// Returns the twist currently being animated, along with a float between
    /// 0.0 and 1.0 indicating the progress on that animation.
    pub fn current_twist(&self) -> Option<(Twist, f32)> {
        self.twist_anim
            .queue
            .get(0)
            .map(|anim| (anim.twist, TWIST_INTERPOLATION_FN(self.twist_anim.progress)))
    }

    /// Returns the state of the cube that should be displayed, not including
    /// the twist currently being animated (if there is one).
    pub fn displayed(&self) -> &dyn PuzzleState {
        match self.twist_anim.queue.get(0) {
            Some(anim) => &*anim.state,
            None => &*self.puzzle,
        }
    }
    /// Returns the state of the cube that should be displayed after the twist
    /// currently being animated (if there is one).
    pub fn next_displayed(&self) -> &dyn PuzzleState {
        match self.twist_anim.queue.get(1) {
            Some(anim) => &*anim.state,
            None => &*self.puzzle,
        }
    }
    /// Returns the state of the cube after all queued twists have been applied.
    pub fn latest(&self) -> &dyn PuzzleState {
        &*self.puzzle
    }

    /// Returns the puzzle type.
    pub fn ty(&self) -> &Arc<PuzzleType> {
        self.puzzle.ty()
    }

    /// Returns the puzzle grip.
    pub fn grip(&self) -> &Grip {
        &self.grip
    }
    /// Sets the puzzle grip.
    pub fn set_grip(&mut self, grip: Grip) {
        if grip != self.grip && !grip.axes.is_empty() {
            self.snap_view_angle_offset();
        }
        self.grip = grip;
    }

    /// Sets the view angle offset.
    pub fn add_view_angle_offset(
        &mut self,
        offset: [f32; 2],
        view_prefs: &ViewPreferences,
        shift: bool,
    ) {
        const X: u8 = 0;
        const Y: u8 = 1;
        let z = if shift { 3 } else { 2 };

        let prefs_view_angle = view_prefs.view_angle();
        let mut offset = Rotor::from_angle_in_axis_plane(z, Y, offset[1].to_radians())
            * Rotor::from_angle_in_axis_plane(X, z, offset[0].to_radians());
        if shift {
            offset = offset.reverse();
        }
        self.view_angle.current =
            prefs_view_angle.reverse() * offset * prefs_view_angle * &self.view_angle.current;
        self.view_angle.target = self.view_angle.current.clone();
        self.view_angle.dragging = true;
    }
    /// Snaps the view angle offset target to the nearest rotation candidate
    /// defined for the puzzle.
    pub fn snap_view_angle_offset(&mut self) {
        if !self.view_angle.dragging {
            self.view_angle.target = self
                .ty()
                .twists
                .nearest_orientation(&self.view_angle.current);
        }
    }

    /// Adds an animation to the view settings animation queue.
    pub fn animate_from_view_settings(&mut self, view_prefs: ViewPreferences) {
        self.view_settings_anim.queue.push_back(view_prefs);
    }

    /// Returns whether this sticker can be hovered.
    fn is_sticker_hoverable(&self, sticker: Sticker) -> bool {
        let less_than_halfway = TWIST_INTERPOLATION_FN(self.twist_anim.progress) < 0.5;
        let puzzle_state = if less_than_halfway {
            self.displayed() // puzzle state before the twist
        } else {
            self.next_displayed() // puzzle state after the twist
        };
        let piece = self.ty().info(sticker).piece;
        self.grip
            .has_piece(puzzle_state, piece)
            .unwrap_or_else(|| self.is_visible(piece))
    }

    /// Sets the hovered stickers, in order from front to back.
    pub fn update_hovered_sticker(
        &mut self,
        stickers_under_cursor: impl IntoIterator<Item = (Sticker, ClickTwists)>,
    ) {
        let hovered = stickers_under_cursor
            .into_iter()
            .find(|&(sticker, _twists)| self.is_sticker_hoverable(sticker));

        self.hovered_sticker = hovered.map(|(sticker, _twists)| sticker);
        self.hovered_twists = hovered.map(|(_sticker, twists)| twists);
    }
    pub(crate) fn hovered_sticker(&self) -> Option<Sticker> {
        self.hovered_sticker
    }
    pub(crate) fn hovered_twists(&self) -> Option<ClickTwists> {
        self.hovered_twists
    }

    /// Returns the current animated view settings, given the static settings
    /// stored in the preferences file.
    pub(crate) fn view_prefs<'a>(&mut self, prefs: &'a Preferences) -> Cow<'a, ViewPreferences> {
        // Use animated view settings.
        let old_view_prefs = prefs.view(self.ty());
        while self.view_settings_anim.queue.back() == Some(&*old_view_prefs) {
            // No need to animate this one! It's the same as what we're
            // currently displaying;
            self.view_settings_anim.queue.pop_back();
        }
        if let Some(old) = self.view_settings_anim.queue.get(0) {
            let new = self
                .view_settings_anim
                .queue
                .get(1)
                .unwrap_or(old_view_prefs);
            let t = self.view_settings_anim.progress;
            Cow::Owned(ViewPreferences::interpolate(old, new, t))
        } else {
            Cow::Borrowed(old_view_prefs)
        }
    }
    pub(crate) fn geometry(&mut self, prefs: &Preferences) -> Arc<Vec<ProjectedStickerGeometry>> {
        let view_prefs = self.view_prefs(prefs);

        let params = StickerGeometryParams::new(
            &view_prefs,
            self.ty(),
            self.current_twist(),
            &self.view_angle.current,
        );

        if self.cached_geometry_params.as_ref() != Some(&params) {
            // Invalidate the cache.
            self.cached_geometry = None;
        }

        self.cached_geometry_params = Some(params.clone());

        let ret = self.cached_geometry.take().unwrap_or_else(|| {
            log::trace!("Regenerating puzzle geometry");

            // Project stickers.
            let mut sticker_geometries: Vec<ProjectedStickerGeometry> = vec![];
            for sticker in (0..self.ty().stickers.len() as _).map(Sticker) {
                let piece = self.ty().info(sticker).piece;
                let vis_piece = self.visual_piece_state(piece);
                if !self.is_sticker_hoverable(sticker) && vis_piece.opacity(prefs) == 0.0 {
                    continue;
                }

                // Compute geometry, including vertex positions in N-dimensional
                // space.
                let sticker_geom = match self.displayed().sticker_geometry(sticker, &params) {
                    Some(s) => s,
                    None => continue, // invisible; skip this sticker
                };

                // Compute vertex positions after transformation and projection
                // down to 2D.
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
                            ClickTwists::default(), // don't care
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

    /// Advances the puzzle geometry and internal state to the next frame, using
    /// the given time delta between this frame and the last.
    pub fn update_geometry(&mut self, delta: Duration, prefs: &InteractionPreferences) {
        // `twist_duration` is in seconds (per one twist); `base_speed` is
        // fraction of twist per frame.
        let base_speed = delta.as_secs_f32() / prefs.twist_duration;

        // Animate view settings.
        self.view_settings_anim.proceed(base_speed);

        // Animate view angle offset.
        self.view_angle.dragging = false;
        if self.view_angle.current != self.view_angle.target {
            let current = &mut self.view_angle.current;

            let decay_multiplier = VIEW_ANGLE_OFFSET_DECAY_RATE.powf(delta.as_secs_f32());
            let new_offset = self.view_angle.target.slerp(current, decay_multiplier);
            if current.s() == new_offset.s() {
                // Stop the animation once we're not making any more progress.
                *current = self.view_angle.target.clone();
            } else {
                *current = new_offset;
            }
        }

        // Animate twist.
        let anim = &mut self.twist_anim;
        if anim.queue.is_empty() {
            anim.queue_max = 0;
        } else {
            // Update queue_max.
            anim.queue_max = std::cmp::max(anim.queue_max, anim.queue.len());
            // Twist exponentially faster if there are/were more twists in the
            // queue.
            let speed_mod = match prefs.dynamic_twist_speed {
                true => ((anim.queue.len() - 1) as f32 * EXP_TWIST_FACTOR).exp(),
                false => 1.0,
            };
            let mut twist_delta = base_speed * speed_mod;
            // Cap the twist delta at 1.0, and also handle the case where
            // something went wrong with the calculation (e.g., division by
            // zero).
            if !(0.0..MIN_TWIST_DELTA).contains(&twist_delta) {
                twist_delta = 1.0; // Instantly complete the twist.
            }
            self.twist_anim.proceed(twist_delta);
        }
    }
    /// Advances the puzzle decorations (outlines and sticker opacities) to the
    /// next frame, using the given time delta between this frame and the last.
    /// Returns whether the decorations changed, in which case a redraw is
    /// needed.
    pub fn update_decorations(&mut self, delta: Duration, prefs: &Preferences) -> bool {
        let mut changed = false;

        let delta = delta.as_secs_f32() / prefs.interaction.other_anim_duration;

        for piece in (0..self.ty().pieces.len() as _).map(Piece) {
            let logical_state = self.logical_piece_state(piece);

            let gripped = self.grip.has_piece(&*self.puzzle, piece);
            let hidden = logical_state.preview_hidden.unwrap_or(logical_state.hidden);
            let stickers = &self.ty().info(piece).stickers;
            let target = VisualPieceState {
                gripped: (gripped == Some(true)) as u8 as f32,
                ungripped: (gripped == Some(false)) as u8 as f32,
                hidden: hidden as u8 as f32,
                selected: stickers.iter().any(|s| self.selection.contains(s)) as u8 as f32,
                hovered: stickers.iter().any(|&s| Some(s) == self.hovered_sticker) as u8 as f32,

                hidden_opacity_override: self.hidden_pieces_preview_opacity,
            };

            /// Adds or subtracts up to `delta` to reach `target`. Returns
            /// `true` if `current` changed.
            fn approach_target(current: &mut f32, target: f32, delta: f32) -> bool {
                if *current == target {
                    false
                } else {
                    if !delta.is_finite() {
                        *current = target; // recovery from invalid state
                    } else if *current + delta < target {
                        *current += delta;
                    } else if *current - delta > target {
                        *current -= delta;
                    } else {
                        *current = target;
                    }
                    true
                }
            }

            let current = &mut self.visual_piece_states[piece.0 as usize];
            let was_visible = current.opacity(prefs) != 0.0;
            changed |= approach_target(&mut current.gripped, target.gripped, delta);
            changed |= approach_target(&mut current.ungripped, target.ungripped, delta);
            changed |= approach_target(&mut current.hidden, target.hidden, delta);
            changed |= approach_target(&mut current.selected, target.selected, delta);
            changed |= approach_target(&mut current.hovered, target.hovered, delta);
            if current.hovered < target.hovered {
                // Highlight hovered sticker instantly for better responsiveness.
                changed |= approach_target(&mut current.hovered, target.hovered, f32::INFINITY);
            }
            if current.hidden_opacity_override != target.hidden_opacity_override {
                // I don't know how to animate this easily, so don't bother trying.
                current.hidden_opacity_override = target.hidden_opacity_override;
                changed = true;
            }
            let is_visible = current.opacity(prefs) != 0.0;
            if was_visible != is_visible {
                // If a piece changes from invisible to visible, then it might need to be
                // re-added to the geometry, so invalidate the cache.
                self.cached_geometry = None;
            }
        }

        changed
    }
    /// Returns the logical state for a piece.
    pub fn logical_piece_state(&self, piece: Piece) -> LogicalPieceState {
        LogicalPieceState {
            hidden: !self.visible_pieces[piece.0 as usize],
            preview_hidden: self
                .visible_pieces_preview
                .as_ref()
                .map(|bits| !bits[piece.0 as usize]),
        }
    }
    /// Returns the visual state for a piece.
    pub fn visual_piece_state(&self, piece: Piece) -> VisualPieceState {
        self.visual_piece_states[piece.0 as usize]
    }

    /// Returns the set of non-hidden pieces.
    pub fn visible_pieces(&self) -> &BitSlice {
        &self.visible_pieces
    }
    /// Returns a mutable reference to the set of non-hidden pieces.
    pub fn visible_pieces_mut(&mut self) -> &mut BitSlice {
        &mut self.visible_pieces
    }
    /// Sets the set of non-hidden pieces.
    pub fn set_visible_pieces(&mut self, visible_pieces: &BitSlice) {
        self.visible_pieces = visible_pieces.to_bitvec();
        self.visible_pieces.resize(self.ty().pieces.len(), false);
    }
    /// Sets the set of non-hidden pieces.
    pub fn set_visible_pieces_preview(
        &mut self,
        visible_pieces: Option<&BitSlice>,
        hidden_opacity: Option<f32>,
    ) {
        self.visible_pieces_preview = visible_pieces.map(|bits| {
            let mut bv = bits.to_bitvec();
            bv.resize(self.ty().pieces.len(), false);
            bv
        });
        self.hidden_pieces_preview_opacity = hidden_opacity;
    }
    /// Returns whether a piece is hidden.
    pub fn is_visible(&self, piece: Piece) -> bool {
        self.visible_pieces[piece.0 as usize]
    }
    /// Returns whether any piece is hidden.
    pub fn is_any_piece_hidden(&self) -> bool {
        !self.visible_pieces.all()
    }

    /// Returns the set of selected stickers
    pub fn selection(&self) -> &HashSet<Sticker> {
        &self.selection
    }
    /// Toggles whether a sticker is selected.
    pub fn toggle_select(&mut self, sticker: Sticker) {
        if self.selection.contains(&sticker) {
            self.deselect(sticker)
        } else {
            self.select(sticker)
        }
    }
    /// Selects a sticker.
    pub fn select(&mut self, sticker: Sticker) {
        self.selection.insert(sticker);
    }
    /// Deselects a sticker.
    pub fn deselect(&mut self, sticker: Sticker) {
        self.selection.remove(&sticker);
    }
    /// Deselects all stickers.
    pub fn deselect_all(&mut self) {
        self.selection = HashSet::new();
    }

    /// Skips the animations for all twists in the queue.
    pub fn skip_twist_animations(&mut self) {
        self.twist_anim.queue.clear();
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
                    let rev = self.ty().reverse_twist(twist);
                    self.animate_twist(rev)?;
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
                HistoryEntry::Twist(twist) => self.animate_twist(twist)?,
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
                self.scramble.len() >= self.ty().scramble_moves_count
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
        self.puzzle.is_solved()
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
            self.ty(),
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

#[derive(Debug, Default, Clone)]
struct TwistAnimationState {
    /// Queue of twist animations to be displayed.
    queue: VecDeque<TwistAnimation>,
    /// Maximum number of animations in the queue (reset when queue is empty).
    queue_max: usize,
    /// Progress of the animation in the current twist, from 0.0 to 1.0.
    progress: f32,
}
impl TwistAnimationState {
    fn proceed(&mut self, delta_t: f32) {
        self.progress += delta_t;
        if self.progress >= 1.0 {
            self.progress = 0.0;
            self.queue.pop_front();
        }
    }
}

#[derive(Debug, Clone)]
struct TwistAnimation {
    /// Puzzle state before twist.
    state: Box<dyn PuzzleState>,
    /// Twist to animate.
    twist: Twist,
}

#[derive(Debug, Default, Clone)]
struct ViewSettingsAnimState {
    /// Queue of view settings animations to be displayed. Each element is a
    /// pair of the view settings before the animation. Once there is only one
    /// element in the queue, the animation will proceed to the view settings
    /// stored in the application preferences and then stop there.
    queue: VecDeque<ViewPreferences>,
    /// Progress of the current animation, from 0.0 to 1.0.
    progress: f32,
}
impl ViewSettingsAnimState {
    /// Removes intermediate animations.
    ///
    /// For example, if the user switches from preset A to preset B, then we
    /// want to animate from A to B. If during the animation from A to B, the
    /// user selects preset C, we should finish the animation from A to B, then
    /// animate from B to C. But if the user also selects preset D during that
    /// animation, then we shouldn't animate from A to B to C to D; we can skip
    /// C. In that example, this method would replace the animations from B to C
    /// and C to D with a single animation from B to D.
    fn remove_intermediate(&mut self) {
        // In the example above, preset D is stored in the current settings, and
        // presets A and B (what we're currently animating between) are at the
        // front of the queue, so just delete everything in the queue after
        // index 2.
        self.queue.truncate(2);
    }
    fn proceed(&mut self, delta_t: f32) {
        if self.queue.is_empty() {
            self.progress = 0.0;
        } else {
            self.remove_intermediate();
            self.progress += delta_t;
            if self.progress >= 1.0 {
                self.queue.pop_front();
                self.progress = 0.0;
            }
        }
    }
}

/// The following rotations are applied to the whole puzzle in order before
/// rendering:
///
/// 1. `queued_delta`
/// 2. `current`
/// 3. `view_prefs.view_angle` (from `ViewPreferences`)
#[derive(Debug, Default, Clone)]
struct ViewAngleAnimState {
    /// View angle offset compared to the latest puzzle state.
    current: Rotor,
    /// Target view angle offset to animate toward.
    target: Rotor,
    /// Whether the user is currently dragging the view.
    dragging: bool,
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

/// Which parts of the puzzle to twist.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Grip {
    pub axes: HashSet<TwistAxis>,
    pub layers: Option<LayerMask>,
}
impl BitOr<&Grip> for Grip {
    type Output = Self;

    fn bitor(mut self, rhs: &Grip) -> Self::Output {
        self |= rhs;
        self
    }
}
impl BitOrAssign<&Grip> for Grip {
    fn bitor_assign(&mut self, rhs: &Self) {
        self.axes.extend(&rhs.axes);
        self.layers = match (self.layers, rhs.layers) {
            (None, None) => None,
            (None, Some(l)) | (Some(l), None) => Some(l),
            (Some(l1), Some(l2)) => Some(l1 | l2),
        }
    }
}
impl Grip {
    pub fn with_axis(axis: TwistAxis) -> Self {
        Self {
            axes: HashSet::from_iter([axis]),
            ..Self::default()
        }
    }
    pub fn with_layers(layers: LayerMask) -> Self {
        Self {
            layers: Some(layers),
            ..Self::default()
        }
    }

    pub fn toggle_axis(&mut self, axis: TwistAxis, exclusive: bool) {
        if self.axes.contains(&axis) {
            if exclusive {
                self.axes = HashSet::new();
            } else {
                self.axes.remove(&axis);
            }
        } else if exclusive {
            self.axes = HashSet::from_iter([axis]);
        } else {
            self.axes.insert(axis);
        }
    }
    pub fn toggle_layer(&mut self, layer: u8, exclusive: bool) {
        let l = self.layers.get_or_insert(LayerMask::default());
        *l ^= LayerMask(1 << layer);
        if exclusive {
            *l &= LayerMask(1 << layer);
        }
        if *l == LayerMask::default() {
            self.layers = None;
        }
    }

    /// Returns whether the twist selection includes a particular piece.
    pub fn has_piece(&self, puzzle: &dyn PuzzleState, piece: Piece) -> Option<bool> {
        if self.axes.is_empty() {
            None
        } else {
            let layer_mask = self.layers.unwrap_or_default();
            Some(
                self.axes
                    .iter()
                    .map(|&twist_axis| puzzle.layer_from_twist_axis(twist_axis, piece))
                    .all(|layer| layer_mask[layer]),
            )
        }
    }
}

/// Boolean piece state, such as whether a piece is hidden.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LogicalPieceState {
    pub hidden: bool,
    pub preview_hidden: Option<bool>,
}

/// Floating-point piece state, such as whether a piece is hidden.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct VisualPieceState {
    pub gripped: f32,
    pub ungripped: f32,
    pub hidden: f32,
    pub selected: f32,
    pub hovered: f32,

    hidden_opacity_override: Option<f32>,
}
impl VisualPieceState {
    pub fn outline_color(self, prefs: &Preferences, is_sticker_selected: bool) -> egui::Rgba {
        let pr = &prefs.outlines;

        let hidden_or_ungripped = f32::max(self.hidden, self.ungripped);

        let mut ret = egui::Rgba::from(pr.default_color);
        // In order from lowest to highest priority:
        ret = util::mix(ret, egui::Rgba::from(pr.hidden_color), hidden_or_ungripped);
        ret = util::mix(ret, egui::Rgba::from(pr.hovered_color), self.hovered);
        ret = util::mix(
            ret,
            egui::Rgba::from(if is_sticker_selected {
                pr.selected_sticker_color
            } else {
                pr.selected_piece_color
            }),
            self.selected,
        );
        ret
    }
    pub fn outline_size(self, prefs: &Preferences) -> f32 {
        let pr = &prefs.outlines;

        let hidden_or_ungripped = f32::max(self.hidden, self.ungripped);

        let mut ret = pr.default_size;
        // In order from lowest to highest priority:
        ret = util::mix(ret, pr.hidden_size, hidden_or_ungripped);
        ret = util::mix(ret, pr.selected_size, self.selected);
        ret = util::mix(ret, pr.hovered_size, self.hovered);
        ret
    }
    pub fn opacity(self, prefs: &Preferences) -> f32 {
        let pr = &prefs.opacity;

        let full_opacity = f32::max(
            self.hovered,
            self.gripped
                * if pr.unhide_grip {
                    1.0
                } else {
                    1.0 - self.hidden
                },
        );
        let hidden_opacity = self.hidden_opacity_override.unwrap_or(pr.hidden);

        let mut ret = 1.0;
        // In order from lowest to highest priority:
        ret = util::mix(ret, hidden_opacity, self.hidden);
        ret *= pr.base;
        ret = util::mix(ret, pr.selected, self.selected);
        ret = util::mix(ret, 1.0, full_opacity);
        if pr.base * pr.ungripped < ret {
            ret = util::mix(ret, pr.base * pr.ungripped, self.ungripped);
        }
        ret
    }
}
