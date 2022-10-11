use std::ops::Mul;

use anyhow::{Context, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::common::*;
use crate::math::*;
use crate::polytope::PolytopeArena;
use crate::schlafli::SchlafliSymbol;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ShapeSpec {
    pub name: Option<String>,
    pub ndim: u8,
    pub facets: Vec<ShapeFacetsSpec>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ShapeFacetsSpec {
    #[serde(default)]
    pub symmetry: SymmetrySpecList,
    pub seeds: Vec<Vector>,
}
impl ShapeFacetsSpec {
    pub fn expand_poles(&self) -> Result<Vec<Vector>> {
        self.symmetry.generate(self.seeds.clone())
    }
}

impl ShapeSpec {
    pub fn build(&self) -> Result<(PuzzleShape, PolytopeArena)> {
        let name = self.name.clone();
        let ndim = self.ndim;

        // Estimate maximum puzzle radius.
        let radius = self
            .facets
            .iter()
            .flat_map(|facet_spec| &facet_spec.seeds)
            .map(|pole| pole.mag())
            .reduce(f32::max)
            .context("no base facets")?;
        let initial_radius = radius * 2.0 * ndim as f32;

        // Construct a polytope arena.
        let mut polytope = PolytopeArena::new_cube(ndim, initial_radius);

        // Construct a list of poles.
        let poles = self
            .facets
            .iter()
            .map(ShapeFacetsSpec::expand_poles)
            .flatten_ok()
            .map_ok(|pole| pole.resize(ndim))
            .collect::<Result<Vec<_>>>()?;

        // Carve the polygon and record metadata for each facet.
        let mut facets = vec![];
        for (i, pole) in poles.iter().enumerate() {
            let plane = Hyperplane::from_pole(pole).context("facet cannot intersect origin")?;
            polytope.carve(&plane, Facet(i as _))?;
            facets.push(FacetInfo {
                name: pole.to_string(),
            });
        }

        // Get the distance of the furthest vertex from the origin.
        let radius = polytope.radius();

        Ok((
            PuzzleShape {
                name,
                ndim,
                facets,
                radius,
            },
            polytope,
        ))
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(transparent)]
pub struct SymmetrySpecList(Vec<SymmetrySpec>);
impl SymmetrySpecList {
    /// Returns a list of generators for the symmetry. This set may not be
    /// minimal.
    pub fn generators(&self) -> Result<Vec<Rotoreflector>> {
        self.0
            .iter()
            .map(|sym| sym.generators())
            .flatten_ok()
            .collect()
    }

    pub fn generate<T>(&self, seeds: Vec<T>) -> Result<Vec<T>>
    where
        for<'a> &'a Rotoreflector: Mul<&'a T, Output = T>,
        T: approx::AbsDiffEq,
    {
        Ok(Group::generate(&self.generators()?, seeds))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SymmetrySpec {
    Schlafli(String),
}
impl SymmetrySpec {
    /// Returns a list of generators for the symmetry, which may include
    /// duplicates.
    pub fn generators(&self) -> Result<Vec<Rotoreflector>> {
        match self {
            Self::Schlafli(string) => Ok(SchlafliSymbol::from_string(&string).generators()),
        }
    }
}
