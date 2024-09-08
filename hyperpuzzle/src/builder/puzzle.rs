use std::sync::{Arc, Weak};

use eyre::Result;
use hypermath::{vector, VecMap, Vector};
use hypershape::prelude::*;
use parking_lot::Mutex;

use super::{shape::ShapeBuildOutput, ShapeBuilder, TwistSystemBuilder};
use crate::puzzle::*;

/// Puzzle being constructed.
#[derive(Debug)]
pub struct PuzzleBuilder {
    /// Reference-counted pointer to this struct.
    pub this: Weak<Mutex<Self>>,

    /// Puzzle ID.
    pub id: String,
    /// Name of the puzzle.
    pub name: String,

    /// Shape of the puzzle.
    pub shape: ShapeBuilder,
    /// Twist system of the puzzle.
    pub twists: TwistSystemBuilder,
}
impl PuzzleBuilder {
    /// Constructs a new puzzle builder with a primordial cube.
    pub fn new(id: String, name: String, ndim: u8) -> Result<Arc<Mutex<Self>>> {
        let shape = ShapeBuilder::new_with_primordial_cube(Space::new(ndim))?;
        let twists = TwistSystemBuilder::new();
        Ok(Arc::new_cyclic(|this| {
            Mutex::new(Self {
                this: this.clone(),

                id,
                name,

                shape,
                twists,
            })
        }))
    }

    /// Returns an `Arc` reference to the puzzle builder.
    pub fn arc(&self) -> Arc<Mutex<Self>> {
        self.this
            .upgrade()
            .expect("`PuzzleBuilder` removed from `Arc`")
    }

    /// Returns the nubmer of dimensions of the underlying space the puzzle is
    /// built in. Equivalent to `self.shape.lock().space.ndim()`.
    pub fn ndim(&self) -> u8 {
        self.shape.space.ndim()
    }
    /// Returns the underlying space the puzzle is built in. Equivalent to
    /// `self.shape.lock().space`
    pub fn space(&self) -> Arc<Space> {
        Arc::clone(&self.shape.space)
    }

    /// Performs the final steps of building a puzzle, generating the mesh and
    /// assigning IDs to pieces, stickers, etc.
    pub fn build(&self, warn_fn: impl Copy + Fn(eyre::Error)) -> Result<Arc<Puzzle>> {
        let mut dev_data = PuzzleDevData::new();

        // Build shape.
        let ShapeBuildOutput {
            mut mesh,
            pieces,
            stickers,
            piece_types,
            piece_type_hierarchy,
            piece_type_masks,
        } = self.shape.build(warn_fn)?;

        // Build color system. TODO: cache this across puzzles?
        let colors = Arc::new(self.shape.colors.build(&self.id, &mut dev_data, warn_fn)?);

        // Build twist system.
        let (axes, twists, gizmo_twists) =
            self.twists
                .build(&self.space(), &mut mesh, &mut dev_data, warn_fn)?;
        let axis_by_name = axes
            .iter()
            .map(|(id, info)| (info.name.clone(), id))
            .collect();
        let twist_by_name = twists
            .iter()
            .map(|(id, info)| (info.name.clone(), id))
            .collect();

        Ok(Arc::new_cyclic(|this| Puzzle {
            this: Weak::clone(this),
            name: self.name.clone(),
            id: self.id.clone(),

            space: self.space(),
            mesh,

            pieces,
            stickers,
            piece_types,
            piece_type_hierarchy,
            piece_type_masks,

            colors,

            scramble_moves_count: 500, // TODO

            notation: Notation {},

            axes,
            axis_by_name,

            twists,
            twist_by_name,

            gizmo_twists,

            dev_data,
        }))
    }
}

/// Piece of a puzzle during puzzle construction.
#[derive(Debug, Clone)]
pub struct PieceBuilder {
    /// Polytope of the piece.
    pub polytope: PolytopeId,
    /// If the piece is defunct because it was cut, these are the pieces it was
    /// cut up into.
    pub cut_result: PieceSet,
    /// Colored stickers of the piece.
    pub stickers: VecMap<FacetId, Color>,
    /// Type of piece, if assigned.
    pub piece_type: Option<PieceType>,

    /// Cached arbitrary point inside the polytope.
    cached_interior_point: Option<Vector>,
}
impl PieceBuilder {
    pub(super) fn new(
        polytope: SpaceRef<'_, impl ToElementId>,
        stickers: VecMap<FacetId, Color>,
    ) -> Result<Self> {
        Ok(Self {
            polytope: polytope.as_element().as_polytope()?.id(),
            cut_result: PieceSet::new(),
            stickers,
            piece_type: None,

            cached_interior_point: None,
        })
    }
    /// Returns the color of a facet, or `Color::INTERNAL` if there is no
    /// color assigned.
    pub fn sticker_color(&self, sticker_id: FacetId) -> Color {
        *self.stickers.get(&sticker_id).unwrap_or(&Color::INTERNAL)
    }

    pub(super) fn interior_point(&mut self, space: &Space) -> &Vector {
        // Average the vertices to get a point that is inside the polytope. For
        // polytopes with many vertices, this could perhaps be improved by using
        // blades.
        self.cached_interior_point.get_or_insert_with(|| {
            let mut count = 0;
            let mut sum = vector![];
            for v in space.get(self.polytope).vertex_set() {
                count += 1;
                sum += v.pos();
            }
            sum / count as _
        })
    }
}

/// Piece type of a puzzle during puzzle construction.
#[derive(Debug, Clone)]
pub struct PieceTypeBuilder {
    /// Name for the piece type. (e.g., "center/oblique_1_2/left")
    pub name: String,
}
