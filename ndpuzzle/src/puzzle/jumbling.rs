use anyhow::Result;
use std::sync::Arc;

use super::*;
use crate::spec::BasicPuzzleSpec;

pub fn build(spec: BasicPuzzleSpec) -> Result<Arc<PuzzleType>> {
    let shape = todo!();
    let twists = todo!();

    Ok(Arc::new_cyclic(|this| PuzzleType {
        this: this.clone(),
        name: spec.name,
        ndim: spec.ndim,
        shape,
        twists,

        family_name: todo!(),
        projection_type: todo!(),
        radius: todo!(),
        layer_count: todo!(),

        pieces: todo!(),
        stickers: todo!(),
        piece_types: todo!(),

        scramble_moves_count: todo!(),

        notation: todo!(),

        new: todo!(),
    }))
}

// pub struct JumblingPuzzleInfo {
//     piece_verts:
// }

// pub struct JumblingPuzzle {
//     piece_states: Vec<Rotor>,
// }

// struct JumblingPiece {
//     verts: Vector,
//     stickers: StickerId,
// }

// struct StickerPolygons {
//     polygons: Vec<SmallVec<[u16; 8]>>,
// }
