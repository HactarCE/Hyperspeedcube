use anyhow::{Context, Result};
use approx::abs_diff_eq;
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
        self.symmetry
            .generate(self.seeds.clone(), |r, t| r * t)?
            .into_iter()
            .map(|(_transform, pole)| Ok(pole))
            .collect()
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
                pole: pole.clone(),
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

    pub fn generate<T>(
        &self,
        seeds: impl IntoIterator<Item = T>,
        transform: impl Fn(&Rotoreflector, &T) -> T,
    ) -> Result<Vec<(Rotoreflector, T)>>
    where
        T: approx::AbsDiffEq,
    {
        let generators = self.generators()?;
        let mut ret = seeds
            .into_iter()
            .map(|seed| (Rotoreflector::ident(), seed))
            .collect_vec();
        let mut unprocessed_idx = 0;
        while unprocessed_idx < ret.len() {
            for gen in &generators {
                let old = &ret[unprocessed_idx];
                let new = transform(gen, &old.1);
                if !ret.iter().any(|old| abs_diff_eq!(old.1, new)) {
                    ret.push((gen * &old.0, new));
                }
            }
            unprocessed_idx += 1;
        }
        Ok(ret)
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
            Self::Schlafli(string) => Ok(SchlafliSymbol::from_string(string).generators()),
        }
    }
}

const AXIS_NAMES: &str = "XYZWUVRS";

pub fn parse_transform(string: &str) -> Option<Rotoreflector> {
    string
        .split("->")
        .map(|v| parse_vector(v)?.normalize())
        .tuple_windows()
        .map(|(v1, v2)| Rotor::from_vec_to_vec(v1.as_ref()?, v2.as_ref()?))
        .try_fold(Rotoreflector::ident(), |r1, r2| {
            Some(r1 * Rotoreflector::from(r2?))
        })
}

pub fn parse_vector(string: &str) -> Option<Vector> {
    if string.contains(',') {
        Some(Vector(
            string
                .split(',')
                .map(|x| x.trim().parse::<f32>())
                .try_collect()
                .ok()?,
        ))
    } else if AXIS_NAMES.contains(string.trim().trim_start_matches('-')) {
        if let Some(s) = string.trim().strip_prefix('-') {
            Some(-Vector::unit(AXIS_NAMES.find(s)? as u8))
        } else {
            Some(Vector::unit(AXIS_NAMES.find(string.trim())? as u8))
        }
    } else {
        None
    }
}
