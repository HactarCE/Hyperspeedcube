use ahash::AHashMap;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::math::*;
use crate::polytope::PolytopeArena;
use crate::puzzle::common::*;

use super::{CutSpec, FlattenedCutSpec, MathExpr, NameSetSpec, SymmetrySpec};

/// Specification for a puzzle shape, which has no internal cuts.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ShapeSpec {
    /// Human-friendly name of the shape.
    pub name: Option<String>,
    /// Number of dimensions.
    pub ndim: u8,
    /// Default symmetry.
    #[serde(default)]
    pub symmetry: SymmetrySpec,

    /// Construction.
    pub construction: Vec<ShapeConstructOperation>,

    /// Facet order override.
    pub facet_order: Option<Vec<String>>,
    /// Default facet colors.
    pub facet_colors: Option<AHashMap<String, String>>,
}
impl ShapeSpec {
    /// Constructs a shape from its spec.
    pub fn build(&self, warnings: &mut Vec<String>) -> Result<(PuzzleShape, PolytopeArena)> {
        todo!()

        /*

        let ndim = self.ndim;

        // Build a list of facets.
        let mut facets = vec![];
        let mut facet_namer = Namer {
            type_of_thing: "facet",
            prefix_iter: crate::util::letters_upper(),
            by_name: AHashMap::new(),
        };
        for seed in &self.facets {
            // If no symmetry is specified for this facet seed, use the default
            // symmetry.
            let symmetry = seed.symmetry.as_ref().unwrap_or(&self.symmetry);

            // Expand one seed facet into multiple facets.
            let poles = symmetry.generate([seed.pole.clone()], |r, t| r * t)?;

            let facet_ids = facets.len()..facets.len() + poles.len();
            let new_facet_names =
                facet_namer.assign_names(&seed.names, facet_ids.map(|i| Facet(i as _)));

            for (name, (_transform, pole)) in new_facet_names.zip(poles) {
                let name = name?;
                let default_color = self
                    .facet_colors
                    .and_then(|colors| colors.get(&name))
                    .cloned();

                // Add the new facet.
                facets.push(FacetInfo {
                    name,
                    pole,
                    default_color,
                });
            }
        }

        if let Some(colors) = self.facet_colors {
            // Warn if any invalid facet names were given. Duplicates are
            // already handled by the deserializer.
            if let Some(unused) = colors
                .keys()
                .find(|&name| !facet_namer.by_name.contains_key(name))
            {
                warnings.push(format!("invalid facet_colors: no facet named {unused:?}"));
            }

            // Leaving off facet names is ok; not all colors need to be
            // specified.
        }

        let mut facets_by_name = facet_namer.by_name;

        // Reorder facets.
        if let Some(order) = self.facet_order {
            let new_facets = vec![];
            let new_facets_by_name = AHashMap::new();
            let old_facets_by_name = facets_by_name;
            for (new_id, name) in order.iter().enumerate() {
                // Warn if any invalid facet names were given. Duplicates are
                // already handled by the deserializer.
                let Some(old_id) = facets_by_name.get(name) else {
                    warnings.push(format!("invalid facet_order; no facet named {name:?}"));
                    continue;
                };
                new_facets.push(facets[old_id.0 as usize].clone());
                new_facets_by_name.insert(name.clone(), Facet(new_id as _));
            }

            // If any facet names were missed, add those and emit an error.
            for facet_info in facets {
                let name = &facet_info.name;
                new_facets_by_name.entry(name.clone()).or_insert_with(|| {
                    warnings.push(format!("invalid facet_order; missing facet {name:?}"));
                    let new_id = Facet(new_facets.len() as _);
                    new_facets.push(facet_info);
                    new_id
                });
            }

            facets = new_facets;
            facets_by_name = new_facets_by_name;
        }

        // Estimate maximum puzzle radius.
        let initial_radius = facets
            .iter()
            .map(|f| f.pole.mag())
            .reduce(Float::max)
            .context("no base facets")?
            * ndim as Float
            * 2.0;

        // Construct a polytope arena.
        let mut polytope = PolytopeArena::new_cube(ndim, initial_radius)?;
        // Carve the polytope.
        for (facet, facet_info) in (0..).map(Facet).zip(facets) {
            let plane =
                Hyperplane::from_pole(&facet_info.pole).context("facet cannot intersect origin")?;
            polytope.carve(&plane, facet)?;
        }

        // Get the distance of the furthest vertex from the origin, or 1.0,
        // whichever is bigger.
        let radius = Float::max(1.0, polytope.radius());

        Ok((
            PuzzleShape {
                name: self.name.clone(),
                ndim,
                facets,
                radius,

                facets_by_name,
            },
            polytope,
        ))

        */
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ShapeConstructOperation {
    /// Symmetry for the construction operation.
    pub symmetry: Option<SymmetrySpec>,

    /// Cut determined by a mathematical expression.
    pub cut: Option<CutSpec>,
    /// Center of a (hyper)spherical cut.
    pub center: Option<MathExpr>,
    /// Radius of a (hyper)spherical cut, or multiple radii.
    pub radius: Option<MathExpr>,
    /// Normal vector to a (hyper)planar cut (may not be normalized).
    pub normal: Option<MathExpr>,
    /// Distance of a (hyper)planar cut from the origin, or multiple distances.
    pub distance: Option<MathExpr>,
    /// Vector from the origin to the nearest point on the (hyper)planar cut, which is
    /// always perpendicular to the (hyper)plane.
    pub pole: Option<MathExpr>,
    /// Cuts to intersect.
    pub intersect: Option<Vec<CutSpec>>,

    /// Whether to remove pieces carved out by the cuts.
    pub remove: Option<bool>,
    /// Whether to generate facets from this operation.
    pub facet: Option<bool>,

    /// Optional prefix before each name.
    pub prefix: Option<String>,
    /// Name to give each facet.
    pub names: Option<Vec<String>>,
}
impl ShapeConstructOperation {
    pub fn cut_spec(&self) -> Result<CutSpec> {
        FlattenedCutSpec {
            cut: &self.cut,
            center: &self.center,
            radius: &self.radius,
            normal: &self.normal,
            distance: &self.distance,
            pole: &self.pole,
            intersect: &self.intersect,
        }
        .try_into()
    }
}
