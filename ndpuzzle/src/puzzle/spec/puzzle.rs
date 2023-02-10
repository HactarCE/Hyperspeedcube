use ahash::AHashMap;
use anyhow::{bail, ensure, Context, Result};
use approx::{abs_diff_eq, AbsDiffEq};
use itertools::Itertools;
use regex::Regex;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::sync::Arc;

use super::{MathExpr, PieceTypesSpec, ShapeSpec, TwistsSpec};
use crate::math::*;
use crate::polytope::*;
use crate::puzzle::PuzzleType;

/// Specification for a puzzle.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PuzzleSpec {
    /// Spec version.
    pub version: usize,

    /// Human-friendly name of the puzzle.
    pub name: String,

    /// Constants used throughout the puzzle definition.
    #[serde(default)]
    pub constants: AHashMap<String, MathExpr>,

    /// Puzzle shape specification.
    pub shape: ShapeSpec,

    /// Puzzle twists specifications.
    #[serde(default)]
    pub twists: TwistsSpec,

    /// Piece type specifications.
    #[serde(default)]
    pub piece_types: Option<PieceTypesSpec>,
}

impl PuzzleSpec {
    /// Constructs a puzzle type from its spec.
    pub fn build(&self, warnings: &mut Vec<String>) -> Result<Arc<PuzzleType>> {
        todo!()

        /*

        // Build the base shape.
        let (shape, mut polytopes) = self.shape.build(warnings)?;
        let twists = self.twists.build(warnings)?;
        let ndim = shape.ndim;

        // Slice for each layer of each twist axis.
        for axis in &twists.axes {
            for cut in &axis.cuts {
                let TwistCut::Planar { radius } = cut;
                polytopes.slice_internal(&Hyperplane {
                    normal: axis.normal.clone(),
                    distance: *radius,
                })?;
            }
        }

        let mut piece_infos = vec![];
        let mut sticker_infos = vec![];
        for piece in polytopes.roots() {
            let shrink_vectors = piece.shrink_vectors(ShrinkStrategy::default())?;

            let i = sticker_infos.len() as u16;
            for sticker in piece.children()? {
                let color = match sticker.facet_set()?.iter().next() {
                    Some(facet) => facet,
                    None if NO_INTERNAL => continue,
                    None => Facet(0), // TODO: make facet optional
                };

                let point_polytopes = sticker.descendent_points()?.collect_vec();

                // Map from the IDs used by `PolytopeArena` to the IDs within
                // this sticker.
                let point_id_map: AHashMap<PolytopeRef<'_>, u16> = point_polytopes
                    .iter()
                    .enumerate()
                    .map(|(i, &vert)| (vert, i as u16))
                    .collect();

                let points = point_polytopes
                    .iter()
                    .map(|point| point.point().cloned())
                    .try_collect()?;

                let shrink_vectors = point_polytopes
                    .iter()
                    .map(|&point| {
                        shrink_vectors
                            .get(sticker, point)
                            .cloned()
                            .context("missing shrink vector")
                    })
                    .try_collect()?;

                let polygons = sticker
                    .descendents_with_rank_at_least(2)?
                    .into_iter()
                    .filter(|p| p.rank() == 2)
                    .map(|polygon| {
                        polygon
                            .polygon_verts()?
                            .map(|point| point_id_map.get(&point).copied().context("missing point"))
                            .collect()
                    })
                    .try_collect()?;

                sticker_infos.push(StickerInfo {
                    piece: Piece(piece_infos.len() as u16),
                    color,

                    points,
                    shrink_vectors,
                    polygons,
                });
            }
            let j = sticker_infos.len() as u16;

            piece_infos.push(PieceInfo {
                stickers: (i..j).map(Sticker).collect(),
                piece_type: PieceType(0),

                points: piece
                    .descendent_points()?
                    .map(|point| point.point().cloned())
                    .try_collect()?,
            })
        }

        let piece_count = piece_infos.len();

        Ok(Arc::new_cyclic(|this| PuzzleType {
            this: this.clone(),
            name: self.name.clone(),
            shape: Arc::new(shape),
            twists: Arc::new(twists),
            family_name: "Fun".to_string(),
            projection_type: match ndim {
                0..=3 => ProjectionType::_3D,
                _ => ProjectionType::_4D,
            },
            layer_count: 9,
            pieces: piece_infos,
            stickers: sticker_infos,
            piece_types: vec![PieceTypeInfo {
                name: "Piece".to_string(),
            }],
            scramble_moves_count: 100,
            notation: NotationScheme {
                axis_names: vec![],
                direction_names: vec![],
                block_suffix: None,
                aliases: vec![],
            },
            new: Box::new(move |ty| {
                Box::new(JumblingPuzzle {
                    ty,
                    piece_states: vec![Rotoreflector::ident(); piece_count],
                })
            }),
        }))

        */
    }
}
