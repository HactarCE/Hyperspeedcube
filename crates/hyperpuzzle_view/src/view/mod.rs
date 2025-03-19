use std::ops::Range;
use std::sync::Arc;

use cgmath::{InnerSpace, SquareMatrix};
use eyre::bail;
use float_ord::FloatOrd;
use hyperdraw::image;
use hyperdraw::{GfxEffectParams, GraphicsState, NdEuclidCamera, NdEuclidPuzzleRenderer};
use hypermath::pga::*;
use hypermath::prelude::*;
use hyperprefs::{
    AnimationPreferences, ColorScheme, FilterPreset, FilterPresetName, FilterPresetRef, FilterRule,
    FilterSeqPreset, InterpolateFn, ModifiedPreset, Preferences, PresetRef,
    PuzzleFilterPreferences,
};
use hyperpuzzle_core::{
    Axis, GizmoFace, LayerMask, LayeredTwist, NdEuclidPuzzleGeometry,
    NdEuclidPuzzleStateRenderData, PerPiece, Piece, PieceMask, Puzzle, PuzzleViewPreferencesSet,
    Sticker,
};
use parking_lot::Mutex;
use smallvec::smallvec;

mod nd_euclid;

use super::ReplayEvent;
use super::simulation::PuzzleSimulation;
use super::styles::*;
pub use nd_euclid::{DragState, NdEuclidViewState, PartialTwistDragState};

/// View into a puzzle simulation, which has its own piece filters.
#[derive(Debug)]
pub struct PuzzleView {
    /// Puzzle state. This is wrapped in an `Arc<Mutex<T>>` so that multiple
    /// puzzle views can access the same state.
    pub sim: Arc<Mutex<PuzzleSimulation>>,

    /// Extra state if this is an N-dimensional Euclidean puzzle.
    nd_euclid: Option<Box<NdEuclidViewState>>,

    /// Current color scheme.
    pub colors: ModifiedPreset<ColorScheme>,
    /// Color scheme to apply for only the current frame.
    ///
    /// This is used to preview a change to a color scheme (particularly when
    /// hovering over UI elements that change the sticker colors when clicked).
    pub temp_colors: Option<ColorScheme>,
    /// Computed piece styles based on the filters state.
    pub styles: PuzzleStyleStates,
    /// Piece filters state.
    pub filters: PuzzleFiltersState,

    /// Whether to show the piece being hovered. This is updated every frame.
    pub show_puzzle_hover: bool,
    /// Whether to show the twist gizmo facet being hovered. This is updated
    /// every frame.
    pub show_gizmo_hover: bool,
}
impl PuzzleView {
    /// Constructs a new puzzle view with an existing simulation.
    pub fn new(
        gfx: &Arc<GraphicsState>,
        sim: &Arc<Mutex<PuzzleSimulation>>,
        prefs: &mut Preferences,
    ) -> Self {
        use hyperprefs::DEFAULT_PRESET_NAME;

        let simulation = sim.lock();
        let puzzle = simulation.puzzle_type();

        let colors = prefs
            .color_schemes
            .get_mut(&puzzle.colors)
            .schemes
            .load_last_loaded(DEFAULT_PRESET_NAME);

        Self {
            sim: Arc::clone(sim),

            nd_euclid: NdEuclidViewState::new(gfx, prefs, puzzle).map(Box::new),

            colors,
            temp_colors: None,
            styles: PuzzleStyleStates::new(puzzle.pieces.len()),
            filters: PuzzleFiltersState::new(prefs.first_custom_style()),

            show_puzzle_hover: false,
            show_gizmo_hover: false,
        }
    }

    /// Returns the puzzle type.
    pub fn puzzle(&self) -> Arc<Puzzle> {
        Arc::clone(self.sim.lock().puzzle_type())
    }

    /// Returns N-dimensional Euclidean view state, if applicable.
    pub fn nd_euclid(&self) -> Option<&NdEuclidViewState> {
        self.nd_euclid.as_deref()
    }
    /// Returns N-dimensional Euclidean view state, if applicable.
    pub fn nd_euclid_mut(&mut self) -> Option<&mut NdEuclidViewState> {
        self.nd_euclid.as_deref_mut()
    }
    /// Sets the temporary gizmo highlight for one frame.
    pub fn set_temp_gizmo_highlight(&mut self, axis: Axis) {
        if let Some(euclid) = &mut self.nd_euclid {
            euclid.temp_gizmo_highlight = Some(axis);
        }
    }

    /// Returns what the cursor was hovering over.
    // TODO: remove this method probably
    pub fn puzzle_hover_state(&self) -> Option<PuzzleHoverState> {
        self.nd_euclid().and_then(|e| e.puzzle_hover_state())
    }

    /// Returns the hovered twist gizmo element.
    // TODO: remove this method probably
    pub fn gizmo_hover_state(&self) -> Option<GizmoHoverState> {
        self.nd_euclid().and_then(|e| e.gizmo_hover_state())
    }

    /// Sets the mouse drag state.
    // TODO: make this more generic
    pub fn set_drag_state(&mut self, new_drag_state: DragState) {
        if let Some(nd_euclid) = &mut self.nd_euclid {
            nd_euclid.drag_state = Some(new_drag_state);
        }
    }
    /// Returns the mouse drag state.
    // TODO: make this more generic
    pub fn drag_state(&self) -> Option<DragState> {
        self.nd_euclid().and_then(|nd_euclid| nd_euclid.drag_state)
    }
    /// Completes a mouse drag.
    pub fn confirm_drag(&mut self) {
        if let Some(nd_euclid) = &mut self.nd_euclid {
            nd_euclid.confirm_drag(&self.sim)
        }
    }
    /// Cancels a mouse drag.
    pub fn cancel_drag(&mut self) {
        if let Some(nd_euclid) = &mut self.nd_euclid {
            nd_euclid.cancel_drag(&self.sim);
        }
    }

    /// Updates the current piece filters.
    fn update_filters(&mut self) {
        let all_pieces = PieceMask::new_full(self.puzzle().pieces.len());

        let fallback_style = self.filters.fallback_style().clone();
        self.styles.set_base_styles(&all_pieces, fallback_style);

        let main_rules = self.filters.iter_active_rules();
        let fallback_rules = self
            .filters
            .combined_fallback_preset
            .iter()
            .flat_map(|f| &f.rules);
        let rules = itertools::chain(main_rules, fallback_rules);

        // Iterate in an order such that later rules override earlier ones.
        for rule in rules.rev() {
            let pieces = rule.set.eval(&self.puzzle());
            self.styles.set_base_styles(&pieces, rule.style.clone());
        }

        for rule in self.filters.iter_active_rules().rev() {
            let pieces = rule.set.eval(&self.puzzle());
            self.styles.set_base_styles(&pieces, rule.style.clone());
        }
    }

    /// Updates the current piece styles based on interaction.
    fn update_styles(&mut self, animation_prefs: &AnimationPreferences) {
        let hovered_piece = None.or_else(|| Some(self.nd_euclid()?.puzzle_hover_state()?.piece));

        // Update hovered piece.
        self.styles
            .set_hovered_piece(hovered_piece.filter(|_| self.show_puzzle_hover));

        // Update blocking state.

        let puzzle = self.puzzle();
        let sim = self.sim.lock();
        let anim = sim.blocking_pieces_anim();
        let amt = anim.blocking_amount(animation_prefs);
        let pieces = PieceMask::from_iter(puzzle.pieces.len(), anim.pieces().iter().copied());
        self.styles.set_blocking_pieces(pieces, amt);
    }

    /// Updates the puzzle view for a frame. This method is idempotent.
    pub fn update(
        &mut self,
        input: PuzzleViewInput,
        prefs: &Preferences,
        animation_prefs: &AnimationPreferences,
    ) {
        if self.filters.changed {
            self.filters.changed = false;
            self.update_filters();
        }

        self.show_puzzle_hover = input.hover_mode == Some(HoverMode::Piece)
            && self.drag_state().is_none()
            && !self.sim.lock().has_twist_anim_queued();
        self.show_gizmo_hover =
            input.hover_mode == Some(HoverMode::TwistGizmo) && self.drag_state().is_none();

        self.update_styles(animation_prefs);

        if let Some(nd_euclid) = &mut self.nd_euclid {
            nd_euclid.update(input, prefs, animation_prefs, &self.sim, &self.styles);
        }
    }

    /// Resets the camera.
    pub fn reset_camera(&mut self) {
        if let Some(nd_euclid) = &mut self.nd_euclid {
            nd_euclid.reset_camera();
        }
    }

    /// Applies a twist to the puzzle based on the current mouse position.
    pub fn do_click_twist(&self, layers: LayerMask, direction: Sign) {
        if let Some(nd_euclid) = &self.nd_euclid {
            nd_euclid.do_click_twist(&mut *self.sim.lock(), layers, direction);
        }
    }

    /// Returns the color value for a given puzzle color, ignoring temporary
    /// per-frame overrides.
    pub fn get_rgb_color(
        &self,
        color: hyperpuzzle_core::Color,
        prefs: &Preferences,
    ) -> Option<hyperpuzzle_core::Rgb> {
        let default_color = self.colors.value.get_index(color.0 as usize)?.1;
        prefs.color_palette.get(default_color)
    }

    /// Renders a screenshot of the puzzle view.
    pub fn screenshot(
        &mut self,
        width: u32,
        height: u32,
    ) -> eyre::Result<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> {
        if let Some(nd_euclid) = &mut self.nd_euclid {
            nd_euclid.renderer.screenshot(width, height)
        } else {
            bail!("puzzle backend does not screenshots")
        }
    }

    /// Returns a mutable reference to the N-dimensional Euclidean camera, if
    /// there is one.
    pub fn nd_euclid_camera_mut(&mut self) -> Option<&mut NdEuclidCamera> {
        Some(&mut self.nd_euclid.as_mut()?.camera)
    }

    /// Returns the downscale rate for the puzzle renderer.
    ///
    /// This is typically 1.
    pub fn downscale_rate(&self) -> u32 {
        match &self.nd_euclid {
            Some(nd_euclid) => nd_euclid.camera.prefs().downscale_rate,
            None => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PuzzleHoverState {
    /// Screen-space cursor coordinates within the puzzle view.
    pub cursor_pos: cgmath::Point2<f32>,
    /// Screen-space Z coordinate.
    z: f32,

    /// Piece being hovered.
    pub piece: Piece,
    /// Sticker being hovered. If this is `None`, then an internal facet of the
    /// piece is being hovered.
    pub sticker: Option<Sticker>,

    /// IDs of the vertices of the hovered triangle.
    vertex_ids: [u32; 3],
    /// Barycentric coordinates on the hovered triangle.
    barycentric_coords: [f32; 3],
    /// Whether the backface of the surface is being hovered (as opposed to the
    /// frontface). This primarily matters in 3D, where stickers are oriented.
    pub backface: bool,

    /// Exact hovered location on the surface of the puzzle, in puzzle space,
    /// after undoing geometry modifications such as sticker shrink and piece
    /// explode.
    pub position: Vector,
    /// First tangent vector of the hovered surface, in puzzle space.
    pub u_tangent: Vector,
    /// Second tangent vector of the hovered surface, in puzzle space.
    pub v_tangent: Vector,
}
impl PuzzleHoverState {
    /// Returns the normal vector to the hovered surface, which is only valid in
    /// 3D.
    pub fn normal_3d(&self) -> Vector {
        self.u_tangent.cross_product_3d(&self.v_tangent)
    }
}

/// Hovered twist gizmo element.
#[derive(Debug, Clone, PartialEq)]
pub struct GizmoHoverState {
    /// Screen-space Z coordinate.
    pub z: f32,

    /// Gizmo face being hovered.
    pub gizmo_face: GizmoFace,

    /// Whether the backface of the gizmo is being hovered (as opposed to the
    /// frontface).
    ///
    /// TODO: check that this is correct -- I'm not sure the gizmo mesh
    ///       construction checks face orientation
    pub backface: bool,
}

/// Input data for a puzzle view for one frame.
pub struct PuzzleViewInput {
    /// Position of the cursor on the puzzle view, in the range -1 to 1.
    pub ndc_cursor_pos: Option<[f32; 2]>,
    /// Size of the target to draw to.
    pub target_size: [u32; 2],
    /// Whether the cursor has been dragged enough to begin a drag twist, if
    /// that's the type of drag happening.
    pub exceeded_twist_drag_threshold: bool,
    /// What the mouse can hover over.
    pub hover_mode: Option<HoverMode>,
}

/// Which kind of objects the user may interact with by hovering with the mouse.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum HoverMode {
    /// Pieces of the puzzle.
    #[default]
    Piece,
    /// Twist gizmos.
    TwistGizmo,
}

/// Piece filters state for a puzzle view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PuzzleFiltersState {
    /// Reference to the saved filter preset, if any.
    pub base: Option<FilterPresetRef>,
    /// Filter preset data.
    pub current: FilterSeqPreset,
    /// Combination of all fallback rules to apply to pieces not specified by
    /// the current preset.
    pub combined_fallback_preset: Option<FilterPreset>,
    /// For each rule: whether it is active. Inactive rules are ignored when
    /// displaying the puzzle.
    pub active_rules: Vec<bool>,

    /// Whether the piece filters have changed since the last frame.
    changed: bool,
}
impl PuzzleFiltersState {
    /// Returns a new empty filters state with no rules and no fallback style.
    pub fn new_empty() -> Self {
        Self {
            base: None,
            current: FilterSeqPreset::new_empty(),
            combined_fallback_preset: None,
            active_rules: vec![],
            changed: true,
        }
    }

    /// Returns a new default filters state with a single rule (to show all
    /// pieces in default state) and an optional fallback style.
    pub fn new(fallback_style: Option<PresetRef>) -> Self {
        Self {
            base: None,
            current: FilterSeqPreset::new_with_single_rule(fallback_style),
            combined_fallback_preset: None,
            active_rules: vec![],
            changed: true,
        }
    }

    /// Iterates over active rules, skipping inactive ones.
    pub fn iter_active_rules(&self) -> impl DoubleEndedIterator<Item = &FilterRule> {
        self.current
            .inner
            .rules
            .iter()
            .enumerate()
            .filter(|(i, _rule)| *self.active_rules.get(*i).unwrap_or(&true))
            .map(|(_i, rule)| rule)
    }

    /// Loads a filter preset, overwriting the current state completely.
    pub fn load_preset(
        &mut self,
        filter_prefs: &PuzzleFilterPreferences,
        name: Option<&FilterPresetName>,
    ) {
        // IIFE to mimic try_block
        match (|| Some((name?, filter_prefs.get(name?)?)))() {
            Some((name, current)) => {
                let preset_ref = filter_prefs.new_ref(name);
                let fallback = filter_prefs.combined_fallback_preset(&preset_ref.name());

                *self = Self {
                    base: Some(preset_ref),
                    current,
                    combined_fallback_preset: fallback,
                    active_rules: vec![],
                    changed: true,
                };
            }
            None => {
                *self = Self {
                    base: None,
                    current: std::mem::take(&mut self.current),
                    combined_fallback_preset: None,
                    active_rules: std::mem::take(&mut self.active_rules),
                    changed: true,
                }
            }
        }
    }
    /// Reloads the current filter preset, overwriting the current state
    /// completely.
    pub fn reload(&mut self, filter_prefs: &PuzzleFilterPreferences) {
        let name = self.base.as_ref().map(|r| r.name());
        self.load_preset(filter_prefs, name.as_ref());
    }

    /// Updates the combined fallback preset.
    pub fn update_combined_fallback_preset(&mut self, filter_prefs: &PuzzleFilterPreferences) {
        if let Some(base) = &self.base {
            let new_fallback = filter_prefs.combined_fallback_preset(&base.name());
            if new_fallback != self.combined_fallback_preset {
                self.combined_fallback_preset = new_fallback;
                self.changed = true;
            }
        } else if self.combined_fallback_preset.is_some() {
            self.combined_fallback_preset = None;
            self.changed = true;
        }
    }

    /// Returns the ultimate fallback style.
    fn fallback_style(&self) -> &Option<PresetRef> {
        match &self.combined_fallback_preset {
            Some(p) => &p.fallback_style,
            None => &self.current.inner.fallback_style,
        }
    }

    /// Marks the filters as having changed, indicating that the puzzle view
    /// should recompute piece styles on the next frame.
    pub fn mark_changed(&mut self) {
        self.changed = true;
    }
}
