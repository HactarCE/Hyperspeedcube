use ahash::AHashMap;
use anyhow::{bail, ensure, Context, Result};
use itertools::Itertools;
use regex::Regex;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::sync::Arc;

use super::{ControlsSpec, NotationSpec};
use super::{CutSpec, FlattenedCutSpec, MathExpr, NameSetSpec, SymmetrySpec};
use crate::math::*;
use crate::polytope::*;
use crate::puzzle::PuzzleTwists;

const MAX_TWIST_PERIOD: usize = 10;

/// Specification for a set of twists.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TwistsSpec {
    /// Human-friendly name of the twists set.
    pub name: Option<String>,
    /// Symmetry of the axis set, and default symmetry used for twist
    /// generators.
    #[serde(default)]
    pub symmetry: SymmetrySpec,

    /// Twist axis specifications.
    #[serde(default)]
    pub axes: Vec<AxisSpec>,
    /// Canonical ordering of twist axes.
    #[serde(default)]
    pub axis_order: Option<Vec<String>>,

    /// Twist transforms.
    #[serde(default)]
    pub transforms: Vec<TwistTransformSpec>,

    /// Twist notation.
    #[serde(default)]
    pub notation: NotationSpec,
    /// Twist controls.
    #[serde(default)]
    pub controls: ControlsSpec,
}
impl Default for TwistsSpec {
    fn default() -> Self {
        Self {
            name: Some("none".to_string()),
            symmetry: SymmetrySpec::default(),

            axes: vec![],
            axis_order: None,

            transforms: vec![],

            notation: NotationSpec::default(),
            controls: ControlsSpec::default(),
        }
    }
}
impl TwistsSpec {
    /// Constructs a twist set from its spec.
    pub fn build(&self, warnings: &mut Vec<String>) -> Result<PuzzleTwists> {
        todo!()

        /*

        // Build a list of twist axes.
        let mut axes = vec![];
        let mut axis_namer = Namer {
            type_of_thing: "twist axis",
            prefix_iter: crate::util::letters_upper(),
            by_name: AHashMap::new(),
        };
        for axis in &self.axes {
            for seed in &axis.seeds {
                ensure!(
                    seed.are_cut_depths_sorted(),
                    "cuts must be sorted by depth from largest to smallest: {:?}",
                    seed.cuts,
                );

                let seed_normal = seed
                    .normal
                    .normalize()
                    .context("axis normal must not be zero")?;
                let normals = axis.symmetry.generate([seed_normal], |r, v| r * v)?;
                let axis_ids = axes.len()..axes.len() + normals.len();
                let names =
                    axis_namer.assign_names(&seed.names, axis_ids.map(|i| TwistAxis(i as _)));

                for (name, (reference_frame, normal)) in names.zip(normals) {
                    axes.push(TwistAxisInfo {
                        name: name?,

                        normal,
                        cuts: seed
                            .cuts
                            .iter()
                            .map(|&radius| TwistCut::Planar { radius })
                            .collect(),

                        transforms: vec![],

                        opposite: None,
                    });
                }

                let reverse_base_frame = base_frame.reverse();

                let generators = axis.symmetry.generators()?;
                let mut periodic_twists = axis
                    .twist_generators
                    .iter()
                    .map(|s| {
                        PeriodicTwist::new(
                            parse_transform(s)
                                .with_context(|| format!("invalid transform: {s:?}"))?,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?;
                let mut unprocessed_idx = 0;
                while unprocessed_idx < periodic_twists.len() {
                    for gen in &generators {
                        let old = &periodic_twists[unprocessed_idx];
                        let mut new = old.transform_by(gen);
                        if gen.is_reflection() {
                            new = new.reverse();
                        }
                        if !periodic_twists.iter().any(|old| approx_eq(old, &new)) {
                            periodic_twists.push(new);
                        }
                    }
                    unprocessed_idx += 1;
                }

                for periodic_twist in periodic_twists {
                    let transforms = periodic_twist
                        .transforms
                        .iter()
                        .map(|t| reverse_base_frame.transform_rotoreflector_uninverted(t))
                        .collect_vec();

                    let first = &transforms[0];
                    if !approx_eq(&first.matrix().col(0).to_vector(), &Vector::unit(0)) {
                        continue; // does not preserve X axis
                    }

                    let i = directions.len();

                    let transform_count = transforms.len();

                    directions.extend(
                        transforms
                            .into_iter()
                            .enumerate()
                            .zip((0..transform_count).rev())
                            .map(|((idx, transform), rev_idx)| TwistDirectionInfo {
                                symbol: (i + idx).to_string(),
                                name: (i + idx).to_string(),
                                qtm: 1,
                                rev: TwistDirection((i + rev_idx) as u8),

                                transform,
                            }),
                    );
                }
            }
        }

        let mut directions = vec![];

        Ok(PuzzleTwists {
            name: self.name.clone().unwrap_or("unnamed twist set".to_string()),

            axes,
            directions,

            orientations: vec![Rotor::ident()],

            notation: NotationNew {},

            axes_by_name: axis_namer.by_name,
        })

        */
    }
}

/// Specification for a set of twist axes.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct AxisSpec {
    /// Symmetry for the set of twist axes.
    pub symmetry: Option<SymmetrySpec>,

    /// Core around which to rotate.
    pub core: Option<MathExpr>,

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

    /// Optional prefix before each name.
    pub prefix: Option<String>,
    /// Name to give each twist axis.
    pub names: Option<Vec<String>>,
}
impl AxisSpec {
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
    pub fn name_set_spec(&self) -> NameSetSpec {
        NameSetSpec {
            prefix: self.prefix.clone(),
            names: self.names.clone(),
        }
    }
}

/// Specification for a twist transform.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TwistTransformSpec {
    /// Twist name.
    pub name: Option<String>,
    /// Name of axis to twist.
    pub axis: String,
    /// Transformation to apply
    pub transform: String,
    /// Multiplicity of the twist.
    pub multiplicity: Option<Vec<i32>>,
}
