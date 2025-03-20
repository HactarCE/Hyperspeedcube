use std::sync::Arc;

use hypermath::pga::Motor;
use hyperpuzzle_core::{
    Axis, BoxDynPuzzleState, BoxDynPuzzleStateRenderData, LayerMask, NdEuclidPuzzleAnimation,
    NdEuclidPuzzleGeometry, NdEuclidPuzzleUiData, PieceMask, Puzzle,
};

/// Extra state for a simulation of an N-dimensional Euclidean puzzle.
#[derive(Debug, Clone)]
pub struct NdEuclidSimState {
    pub geom: Arc<NdEuclidPuzzleGeometry>,
    pub partial_twist_drag_state: Option<PartialTwistDragState>,
}
impl NdEuclidSimState {
    /// Constructs a fresh state.
    ///
    /// Returns `None` if `puzzle` is not an N-dimensional Euclidean puzzle.
    pub fn new(puzzle: &Arc<Puzzle>) -> Option<Self> {
        let geom = Arc::clone(puzzle.ui_data.downcast_ref::<NdEuclidPuzzleUiData>()?);

        Some(Self {
            geom,
            partial_twist_drag_state: None,
        })
    }

    /// Returns a render state for a partial twist drag, or `None` if there is
    /// none in progress.
    pub fn partial_twist_drag_render_state(
        &self,
        latest_state: &BoxDynPuzzleState,
    ) -> Option<BoxDynPuzzleStateRenderData> {
        let partial = self.partial_twist_drag_state.as_ref()?;
        let anim = NdEuclidPuzzleAnimation {
            pieces: partial.grip.clone(),
            initial_transform: partial.transform.clone(),
            final_transform: partial.transform.clone(),
        };
        let t = 0.0;
        Some(latest_state.animated_render_data(&anim.into(), t))
    }
}

#[derive(Debug, Clone)]
pub struct PartialTwistDragState {
    pub axis: Axis,
    pub layers: LayerMask,
    pub grip: PieceMask,
    pub transform: Motor,
}
