use std::collections::HashMap;

use eyre::{bail, Result};
use hypermath::collections::generic_vec::IndexOutOfRange;
use hypershape::prelude::*;

use super::{CustomOrdering, NamingScheme};
use crate::{Color, PerColor};

/// Sticker color during shape construction.
#[derive(Debug, Clone)]
pub struct ColorBuilder {
    manifolds: ManifoldSet,

    /// Default color string.
    pub default_color: Option<String>,
}
impl ColorBuilder {
    /// Returns the color's manifold set.
    pub fn manifolds(&self) -> &ManifoldSet {
        &self.manifolds
    }
}

/// Set of all sticker colors during shape construction.
#[derive(Debug, Default)]
pub struct ColorSystemBuilder {
    /// Color data (not including name and ordering).
    by_id: PerColor<ColorBuilder>,
    /// Map from manifold set to color ID.
    manifold_set_to_id: HashMap<ManifoldSet, Color>,
    /// Set of manifolds with colors assigned.
    used_manifolds: ManifoldSet,
    /// User-specified color names.
    pub names: NamingScheme<Color>,
    /// User-specified ordering of colors.
    pub ordering: CustomOrdering<Color>,
}
impl ColorSystemBuilder {
    /// Constructs a new empty color system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of colors in the color system.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Returns a deep copy of the color system in a new space.
    pub fn clone(&self, space_map: &mut SpaceMap<'_>) -> Self {
        Self {
            by_id: self
                .by_id
                .iter()
                .map(|(_id, color)| ColorBuilder {
                    manifolds: space_map.map_set(&color.manifolds),
                    default_color: color.default_color.clone(),
                })
                .collect(),
            manifold_set_to_id: self
                .manifold_set_to_id
                .iter()
                .map(|(manifolds, &color)| (space_map.map_set(manifolds), color))
                .collect(),
            used_manifolds: space_map.map_set(&self.used_manifolds),
            names: self.names.clone(),
            ordering: self.ordering.clone(),
        }
    }

    /// Adds a new color.
    pub fn add(&mut self, manifolds: ManifoldSet) -> Result<Color> {
        // Check that the manifolds aren't already taken.
        for m in manifolds.iter() {
            if self.used_manifolds.contains(m) {
                bail!("manifold is already taken");
            }
        }

        let id = self.by_id.push(ColorBuilder {
            manifolds: manifolds.clone(),
            default_color: None,
        })?;
        self.ordering.add(id)?;
        self.manifold_set_to_id.insert(manifolds.clone(), id);
        self.used_manifolds.extend(manifolds);

        Ok(id)
    }

    /// Returns a reference to a color by ID, or an error if the ID is out of
    /// range.
    pub fn get(&self, id: Color) -> Result<&ColorBuilder, IndexOutOfRange> {
        self.by_id.get(id)
    }
    /// Returns a mutable reference to a color by ID, or an error if the ID is
    /// out of range.
    pub fn get_mut(&mut self, id: Color) -> Result<&mut ColorBuilder, IndexOutOfRange> {
        self.by_id.get_mut(id)
    }

    /// Returns a map from manifold set to color ID.
    pub fn manifold_set_to_id(&self) -> &HashMap<ManifoldSet, Color> {
        &self.manifold_set_to_id
    }

    /// Returns an iterator over all the colors, in the canonical ordering.
    pub fn iter(&self) -> impl Iterator<Item = (Color, &ColorBuilder)> {
        self.ordering
            .ids_in_order()
            .iter()
            .map(|&id| (id, &self.by_id[id]))
    }
}
