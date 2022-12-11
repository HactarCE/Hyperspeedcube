//! Puzzle specification structures.

use ahash::AHashMap;
use anyhow::{ensure, Context, Result};
use approx::abs_diff_eq;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use super::common::*;
use crate::math::*;
use crate::polytope::PolytopeArena;
use crate::schlafli::SchlafliSymbol;

/// Specification for a puzzle shape, which has no internal cuts.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ShapeSpec {
    /// Human-friendly name of the shape.
    pub name: Option<String>,
    /// Number of dimensions.
    pub ndim: u8,
    /// Facet specifications.
    pub facets: Vec<FacetsSpec>,
}
impl ShapeSpec {
    /// Constructs a shape from its spec.
    pub fn build(&self) -> Result<(PuzzleShape, PolytopeArena)> {
        let name = self.name.clone();
        let ndim = self.ndim;

        // Estimate maximum puzzle radius.
        let radius = self
            .facets
            .iter()
            .flat_map(|facet_spec| &facet_spec.seeds)
            .map(|seed| seed.pole.mag())
            .reduce(f32::max)
            .context("no base facets")?;
        let initial_radius = radius * 2.0 * ndim as f32;

        // Construct a polytope arena.
        let mut polytope = PolytopeArena::new_cube(ndim, initial_radius)?;

        // Carve the polygon and record metadata for each facet.
        let mut facets = vec![];
        let mut facet_namer = Namer {
            type_of_thing: "facet",
            prefix_iter: crate::util::letters_upper(),
            by_name: AHashMap::new(),
        };
        for facet_set in &self.facets {
            for seed in &facet_set.seeds {
                // Expand one seed into multiple facets.
                let poles = facet_set
                    .symmetry
                    .generate([seed.pole.clone()], |r, t| r * t)?;

                let facet_ids = facets.len()..facets.len() + poles.len();
                let named_facets =
                    facet_namer.with_names(&seed.names, facet_ids.map(|i| Facet(i as _)))?;

                for ((name, facet), (_transform, pole)) in named_facets.into_iter().zip(poles) {
                    // Carve the polytope.
                    let plane =
                        Hyperplane::from_pole(&pole).context("facet cannot intersect origin")?;
                    polytope.carve(&plane, facet)?;

                    // Add the new facet.
                    facets.push(FacetInfo {
                        name,
                        pole,

                        default_color: None,
                    });
                }
            }
        }

        // Get the distance of the furthest vertex from the origin, or 1.0,
        // whichever is bigger.
        let radius = f32::max(1.0, polytope.radius());

        Ok((
            PuzzleShape {
                name,
                ndim,
                facets,
                radius,

                facets_by_name: facet_namer.by_name,
            },
            polytope,
        ))
    }
}

/// Specification for a symmetric set of puzzle facets.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FacetsSpec {
    /// Symmetry for the set of facets.
    #[serde(default)]
    pub symmetry: SymmetrySpecList,
    /// Seeds to generate the facet set.
    pub seeds: Vec<FacetSeedSpec>,
}

/// Specification for a set of facets derived from one pole.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FacetSeedSpec {
    /// Vector from the origin to the facet plane and perpendicular to the
    /// facet.
    pub pole: Vector,
    /// Facet names.
    #[serde(flatten)]
    pub names: NameSetSpec,
}

/// Specification for a set of names.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct NameSetSpec {
    /// Optional prefix before each name.
    pub prefix: Option<String>,
    /// Name to give each member.
    pub names: Option<Vec<String>>,
}

/// Specification for a set of symmetries.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(transparent)]
pub struct SymmetrySpecList(pub Vec<SymmetrySpec>);
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

    /// Multiplies each element of the symmetry group by each seed and returns
    /// the list of unique results.
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

/// Specification for a single symmetry.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SymmetrySpec {
    /// Schlafli symbol describing a symmetry.
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

/// Helper struct to give names to things.
#[derive(Debug)]
pub(super) struct Namer<P, T> {
    pub(super) prefix_iter: P,
    pub(super) by_name: AHashMap<String, T>,
    pub(super) type_of_thing: &'static str,
}
impl<P: Iterator<Item = String>, T: Copy> Namer<P, T> {
    /// Returns the names for a set of things.
    pub fn with_names(
        &mut self,
        name_set: &NameSetSpec,
        elements: impl IntoIterator<Item = T>,
    ) -> Result<Vec<(String, T)>> {
        let prefix = if let Some(prefix) = &name_set.prefix {
            prefix.clone()
        } else if name_set.names.is_some() {
            "".to_string()
        } else {
            self.prefix_iter.next().unwrap()
        };

        let user_specified_names = name_set.names.iter().flatten().map(Cow::Borrowed);
        let unprefixed_names =
            user_specified_names.chain(crate::util::letters_lower().map(Cow::Owned));

        unprefixed_names
            .map(|s| format!("{prefix}{s}"))
            .zip(elements)
            .map(|(name, thing)| {
                // Ensure the name is unique.
                let is_name_unique = self.by_name.insert(name.clone(), thing).is_none();
                ensure!(
                    is_name_unique,
                    "{} names must be unique; multiple have name {name:?}",
                    self.type_of_thing,
                );
                Ok((name, thing))
            })
            .collect()
    }
}

const AXIS_NAMES: &str = "XYZWUVRS";

/// Parses a user-specified transform.
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

/// Parses a user-specified vector.
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
