#![allow(clippy::too_many_arguments, clippy::too_many_lines)]

use std::sync::{Arc, Weak};

use eyre::Result;
use hypermath::VecMap;
use hypershape::prelude::*;
use parking_lot::Mutex;

use super::{ShapeBuilder, TwistSystemBuilder};
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
    pub fn build(&self, mut warn_fn: impl FnMut(eyre::Error)) -> Result<Arc<Puzzle>> {
        // Build shape.
        let (mesh, pieces, stickers) = self.shape.build()?;

        // Build color system.
        let colors = self.shape.colors.build()?;

        // Build list of piece types.
        let piece_types = [PieceTypeInfo {
            name: "Piece".to_string(), // TODO piece types
        }]
        .into_iter()
        .collect();

        // Build twist system.
        let (axes, twists, twist_gizmos) = self.twists.build(&self.space(), &mut warn_fn)?;
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

            mesh,

            pieces,
            stickers,
            piece_types,
            colors,

            scramble_moves_count: 500, // TODO

            notation: Notation {},

            axes,
            axis_by_name,

            twists,
            twist_by_name,

            twist_gizmos,

            space: self.space(),
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
        })
    }
    /// Returns the color of a facet, or `Color::INTERNAL` if there is no
    /// color assigned.
    pub fn sticker_color(&self, sticker_id: FacetId) -> Color {
        *self.stickers.get(&sticker_id).unwrap_or(&Color::INTERNAL)
    }
}
