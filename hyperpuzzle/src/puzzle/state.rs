use std::{collections::HashMap, sync::Arc};

use hypermath::{collections::GenericVec, idx_struct, prelude::*};
use hypershape::VertexId;
use itertools::Itertools;
use parking_lot::Mutex;

use crate::{Axis, LayerMask, LayeredTwist, PerPiece, Piece, PieceMask, Puzzle};

use super::{AxisInfo, Layer, LayerInfo, PerAxis, PerLayer};

type PerCachedTransform<T> = GenericVec<CachedTransform, T>;
idx_struct! {
    struct CachedTransform(usize);
}

#[derive(Debug)]
struct CachedTransformData {
    pub motor: pga::Motor,
    pub rev_motor: pga::Motor,
    pub transformed_cuts: PerAxis<Option<PerLayer<Option<LayerInfo>>>>,
}
impl CachedTransformData {
    fn new(motor: pga::Motor, axes: &PerAxis<AxisInfo>) -> Self {
        let transformed_cuts = axes.map_ref(|_, _| None);
        let rev_motor = motor.reverse();
        Self {
            motor,
            rev_motor,
            transformed_cuts,
        }
    }
    fn reverse_transform_layer(
        &mut self,
        axis: Axis,
        layer: Layer,
        axes: &PerAxis<AxisInfo>,
    ) -> &LayerInfo {
        self.transformed_cuts[axis].get_or_insert_with(|| axes[axis].layers.map_ref(|_, _| None))
            [layer]
            .get_or_insert_with(|| self.rev_motor.transform(&axes[axis].layers[layer]))
    }
}

/// Instance of a puzzle with a particular state.
#[derive(Debug, Clone)]
pub struct PuzzleState {
    /// Immutable puzzle type info.
    puzzle_type: Arc<Puzzle>,
    /// Attitude (position & rotation) of each piece.
    piece_transforms: PerPiece<CachedTransform>,
    /// Cached set of possible attitudes of pieces.
    cached_transforms: Arc<Mutex<PerCachedTransform<CachedTransformData>>>,
    cached_transform_by_motor: Arc<Mutex<ApproxHashMap<pga::Motor, CachedTransform>>>,
    cached_which_side_results:
        Arc<Mutex<PerAxis<PerLayer<(HashMap<VertexId, WhichSide>, HashMap<VertexId, WhichSide>)>>>>,
}
impl PuzzleState {
    /// Constructs a new instance of a puzzle.
    pub fn new(puzzle_type: Arc<Puzzle>) -> Self {
        let ident = pga::Motor::ident(puzzle_type.ndim());
        let piece_transforms = puzzle_type.pieces.map_ref(|_, _| CachedTransform(0));

        let cached_transforms = Arc::new(Mutex::new(PerCachedTransform::from_iter([
            CachedTransformData::new(ident.clone(), &puzzle_type.axes),
        ])));

        let mut by_motor = ApproxHashMap::new();
        by_motor.insert(ident, CachedTransform(0));
        let cached_transform_by_motor = Arc::new(Mutex::new(by_motor));

        let cached_which_side_results =
            Arc::new(Mutex::new(puzzle_type.axes.map_ref(|_, axis_info| {
                axis_info
                    .layers
                    .map_ref(|_, _| (HashMap::new(), HashMap::new()))
            })));

        PuzzleState {
            puzzle_type,
            piece_transforms,
            cached_transforms,
            cached_transform_by_motor,
            cached_which_side_results,
        }
    }
    /// Returns the puzzle type
    pub fn ty(&self) -> &Arc<Puzzle> {
        &self.puzzle_type
    }
    /// Returns the position and rotation of each piece.
    pub fn piece_transforms(&self) -> PerPiece<pga::Motor> {
        let cached = self.cached_transforms.lock();
        self.piece_transforms
            .map_ref(|_, &i| cached[i].motor.clone())
    }
    /// Returns the position and rotation of each piece during an arbitrary
    /// animation affecting a subset of pieces.
    pub fn partial_piece_transforms(
        &self,
        grip: &PieceMask,
        transform: &pga::Motor,
    ) -> PerPiece<pga::Motor> {
        self.piece_transforms()
            .map(|piece, static_transform| match grip.contains(piece) {
                true => transform * static_transform,
                _ => static_transform.clone(),
            })
    }

    /// Does a twist, or returns an error containing the set of pieces that
    /// prevented the twist.
    pub fn do_twist(&self, twist: LayeredTwist) -> Result<Self, Vec<Piece>> {
        let twist_info = &self.puzzle_type.twists[twist.transform];
        let grip = self.compute_grip(twist_info.axis, twist.layers);

        // Check for split pieces, which prevent the turn.
        let split_pieces = grip
            .iter_filter(|_piece, &which_side| which_side == WhichSide::Split)
            .collect_vec();
        if !split_pieces.is_empty() {
            return Err(split_pieces);
        }

        let mut cached_transforms = self.cached_transforms.lock();
        let mut cached_transform_by_motor = self.cached_transform_by_motor.lock();

        let piece_transforms = self.piece_transforms.map_ref(|piece, &piece_transform| {
            if grip[piece] == WhichSide::Inside {
                let current_motor = &cached_transforms[piece_transform].motor;
                let new_motor = &twist_info.transform * current_motor;
                *cached_transform_by_motor
                    .entry(new_motor.clone())
                    .or_insert_with(|| {
                        cached_transforms
                            .push(CachedTransformData::new(new_motor, &self.puzzle_type.axes))
                            .expect("out of memory")
                    })
            } else {
                piece_transform
            }
        });

        Ok(Self {
            puzzle_type: Arc::clone(&self.puzzle_type),
            piece_transforms,
            cached_transforms: Arc::clone(&self.cached_transforms),
            cached_transform_by_motor: Arc::clone(&self.cached_transform_by_motor),
            cached_which_side_results: Arc::clone(&self.cached_which_side_results),
        })
    }

    /// Returns the set of pieces on the inside of a grip (axis + layer mask).
    /// This considers blocking pieces to be outside the grip; use
    /// `compute_grip()` to see which pieces are blocking a twist.
    pub fn compute_gripped_pieces(&self, axis: Axis, layers: LayerMask) -> PieceMask {
        PieceMask::from_iter(
            self.puzzle_type.pieces.len(),
            self.compute_grip(axis, layers)
                .iter_filter(|_, &status| status == WhichSide::Inside),
        )
    }

    /// Returns each piece's location with respect to a grip (axis + layer
    /// mask). A piece may be inside the grip, outside the grip, or blocking the
    /// grip. [`WhichSide::Flush`] is not used.
    pub fn compute_grip(&self, axis: Axis, layers: LayerMask) -> PerPiece<WhichSide> {
        let Ok(axis_info) = self.puzzle_type.axes.get(axis) else {
            log::error!("bad axis ID");
            return self.puzzle_type.pieces.map_ref(|_, _| WhichSide::Split);
        };

        let grip_layers = layers
            .iter()
            .filter_map(|layer| Some((layer, axis_info.layers.get(layer).ok()?)))
            .collect_vec();

        let mut segments: Vec<(Layer, Option<Layer>)> = vec![];
        for (layer, layer_info) in grip_layers {
            if let Some((_, Some(prev_top))) = segments.last_mut() {
                let prev_layer_info = &axis_info.layers[*prev_top];
                if prev_layer_info.top == Some(layer_info.bottom.flip()) {
                    *prev_top = layer;
                    continue;
                }
            }
            segments.push((layer, Some(layer)));
        }

        let space = &self.puzzle_type.space;

        let mut cached_transforms = self.cached_transforms.lock();

        self.puzzle_type.pieces.map_ref(|piece, piece_info| {
            // IIFE to mimic try_block
            (|| {
                let polytope = piece_info.polytope;
                let piece_transform = &mut cached_transforms[self.piece_transforms[piece]];
                let mut is_inside_any = false;
                'per_segment: for &(bottom, top) in &segments {
                    // bottom
                    {
                        let transformed_cut = &piece_transform
                            .reverse_transform_layer(axis, bottom, &self.puzzle_type.axes)
                            .bottom;
                        match space.get(polytope).is_on_which_side_of(transformed_cut) {
                            WhichSide::Outside => continue 'per_segment, // not in this segment; continue to next segment
                            WhichSide::Split => return Ok(WhichSide::Split), // split by one segment; cannot turn!
                            _ => (),
                        }
                    }

                    // top
                    if let Some(transformed_cut) = top.and_then(|top| {
                        piece_transform
                            .reverse_transform_layer(axis, top, &self.puzzle_type.axes)
                            .top
                            .as_ref()
                    }) {
                        match space.get(polytope).is_on_which_side_of(transformed_cut) {
                            WhichSide::Outside => continue 'per_segment, // not in this segment; continue to next segment
                            WhichSide::Split => return Ok(WhichSide::Split), // split by one segment; cannot turn!
                            _ => (),
                        }
                    }

                    // This piece wasn't excluded by either the bottom or the
                    // top, so it should be good!
                    is_inside_any = true;
                }
                match is_inside_any {
                    true => Ok(WhichSide::Inside),
                    false => Ok(WhichSide::Outside),
                }
            })()
            .unwrap_or_else(|e: eyre::Report| {
                log::error!("{e}");
                WhichSide::Split
            })
        })
    }

    /// Returns the smallest layer mask on `axis` that contains `piece`.
    pub fn min_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        let space = &self.puzzle_type.space;

        let cached_transforms = self.cached_transforms.lock();
        let piece_transform = &cached_transforms[self.piece_transforms[piece]].motor;

        // TODO: This assumes the piece only spans one layer. It does not
        //       account for bandaging.
        self.ty().axes[axis]
            .layers
            .find(|_layer, layer_info| {
                space
                    .get(self.ty().pieces[piece].polytope)
                    .vertex_set()
                    .all(|v| {
                        let p = piece_transform.transform_point(v.pos());
                        layer_info.bottom.location_of_point(&p) != PointWhichSide::Outside && {
                            match &layer_info.top {
                                Some(top) => top.location_of_point(&p) != PointWhichSide::Outside,
                                None => true,
                            }
                        }
                    })
            })
            .map(LayerMask::from)
    }
}
