use anyhow::{anyhow, bail, ensure, Context, Result};
use std::fmt;

use super::{Manifold, ManifoldWhichSide};
use crate::math::{cga::*, *};

/// Manifold in Euclidean space, represented using a CGA blade.
#[derive(Debug, Clone, PartialEq)]
pub struct EuclideanCgaManifold {
    space_ndim: u8,
    manifold_ndim: u8,
    opns: Blade,
}

impl AbsDiffEq for EuclideanCgaManifold {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        Blade::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.space_ndim == other.space_ndim
            && self.manifold_ndim == other.manifold_ndim
            && self.opns.abs_diff_eq(&other.opns, epsilon)
    }
}

impl fmt::Display for EuclideanCgaManifold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut need_to_print_multivector = true;
        if self.manifold_ndim == 0 {
            if let Ok([p1, p2]) = self.to_point_pair() {
                write!(f, "point pair ")?;
                fmt::Display::fmt(&p1, f)?;
                write!(f, " .. ")?;
                fmt::Display::fmt(&p2, f)?;
                need_to_print_multivector = false;
            }
        } else if self.manifold_ndim == self.space_ndim - 1 {
            let ipns = self.ipns();
            let is_flat = ipns.ipns_is_flat();
            if is_flat {
                if let (Some(n), Some(d)) = (ipns.ipns_plane_normal(), ipns.ipns_plane_distance()) {
                    let n = n.pad(self.space_ndim);
                    write!(f, "hyperplane n={n} d={d}")?;
                    need_to_print_multivector = false;
                }
            } else if let (c, Some(r)) = (ipns.ipns_sphere_center(), ipns.ipns_radius()) {
                write!(f, "hypersphere c={c} r={r}")?;
                need_to_print_multivector = false;
            }
        } else if self.manifold_ndim == self.space_ndim {
            write!(f, "whole space ")?;
        }

        if need_to_print_multivector {
            fmt::Display::fmt(&self.opns, f)?;
        }

        Ok(())
    }
}

impl EuclideanCgaManifold {
    /// Constructs a manifold that is the whole space of an OPNS blade.
    pub fn whole_space(ndim: u8) -> Self {
        Self::from_opns(Blade::pseudoscalar(ndim), ndim).unwrap()
    }
    /// Constructs a manifold from an OPNS blade, or returns `None` if the
    /// object is a point or scalar.
    pub fn from_opns(opns: Blade, space_ndim: u8) -> Option<Self> {
        Some(Self {
            space_ndim,
            manifold_ndim: opns.grade().checked_sub(2)?,
            opns,
        })
    }
    /// Constructs a manifold from an IPNS blade, or returns `None` if it is
    /// degenerate.
    pub fn from_ipns(ipns: &Blade, space_ndim: u8) -> Option<Self> {
        Self::from_opns(ipns.ipns_to_opns(space_ndim), space_ndim)
    }

    /// Returns the OPNS blade representing the manifold.
    pub fn opns(&self) -> &Blade {
        &self.opns
    }
    /// Returns the IPNS blade representing the manifold.
    pub fn ipns(&self) -> Blade {
        self.opns.opns_to_ipns(self.space_ndim)
    }

    /// Returns the IPNS blade representing the manifold within a space
    /// containing it.
    pub fn ipns_in_space(&self, space: &Self) -> Blade {
        self.opns().opns_to_ipns_in_space(space.opns())
    }
}

/// Manifold represented by a blade in the conformal geometric algebra.
impl Manifold for EuclideanCgaManifold {
    type Point = Point;

    fn ndim(&self) -> Result<u8> {
        Ok(self.manifold_ndim)
    }

    fn new_point_pair(a: &Self::Point, b: &Self::Point, space: &Self) -> Result<Self> {
        EuclideanCgaManifold::from_opns(Blade::point(a) ^ Blade::point(b), space.space_ndim)
            .context("error splitting point pair")
    }

    fn to_point_pair(&self) -> Result<[Self::Point; 2]> {
        ensure!(self.ndim()? == 0, "expected point pair");
        self.opns()
            .point_pair_to_points()
            .context("unable to split point pair")
    }

    fn triple_orientation(&self, points: [&Self::Point; 3]) -> f32 {
        let [a, b, c] = points.map(Blade::point);
        self.opns().unchecked_scale_factor_to(&(a ^ b ^ c))
    }

    fn flip(&self) -> Result<Self> {
        Ok(Self {
            space_ndim: self.space_ndim,
            manifold_ndim: self.manifold_ndim,
            opns: -&self.opns,
        })
    }

    fn relative_orientation(&self, other: &Self) -> Option<Sign> {
        let factor = self.opns.scale_factor_to(&other.opns)?;
        if factor.is_sign_negative() {
            Some(Sign::Neg)
        } else {
            Some(Sign::Pos)
        }
    }

    fn intersect(&self, cut: &Self, space: &Self) -> Result<Option<Self>> {
        ensure!(cut.ndim()? + 1 == space.ndim()?);
        ensure!(self.ndim()? <= space.ndim()?);

        if self.ndim()? == space.ndim()? {
            // `self` is the whole space, so the intersection is just `cut`. Be
            // sure to get the sign right though.
            let result = match self.relative_orientation(space) {
                None => bail!(
                    "cannot intersect two manifolds because \
                     {self} is not contained within {space}"
                ),
                Some(Sign::Pos) => cut.clone(),
                Some(Sign::Neg) => cut.flip()?,
            };
            return Ok(Some(result));
        }

        let cut_ipns = cut.opns().opns_to_ipns_in_space(space.opns());
        let self_ipns = self.opns().opns_to_ipns_in_space(space.opns());
        let intersection = (cut_ipns ^ self_ipns).ipns_to_opns_in_space(space.opns());
        let intersection_manifold = if intersection.opns_is_real() {
            EuclideanCgaManifold::from_opns(intersection, self.space_ndim)
        } else {
            None
        };

        if let Some(intersection) = &intersection_manifold {
            ensure!(intersection.ndim()? + 1 == self.ndim()?);
        }

        Ok(intersection_manifold)
    }

    fn which_side(&self, cut: &Self, space: &Self) -> Result<ManifoldWhichSide> {
        ensure!(cut.ndim()? + 1 == space.ndim()?);
        ensure!(self.ndim()? <= space.ndim()?);

        if self.ndim()? == space.ndim()? {
            return Ok(ManifoldWhichSide {
                is_any_inside: true,
                is_any_outside: true,
            });
        }

        // Get the IPNS (inner product null space) representation of the
        // hypersphere that is perpendicular to `space` and tangent to `cut`.
        let cut_ipns = cut.ipns_in_space(space);
        // ... and the one tangent to `cut`.
        let self_ipns = self.ipns_in_space(space);

        // Find two points on `self` such that they straddle `cut` if `self`
        // intersects `cut`. If `self` is entirely on one side of `cut`, then
        // these points will give both be on the same side.
        let pair_on_self_across_cut = if self.ndim()? == 0 {
            // `self` is a point pair. Just query each of those points.
            self.opns().clone()
        } else {
            // This algorithm took WEEKS of work to figure out. Huge thanks to
            // Luna Harran for helping!
            //
            // Here's a geometric algebra expression for what we're about to do:
            // `c1 & !(c1 & c2 & !p7)`
            //
            // See `cga_euclidean_demo.js` for an interactive ganja.js demo.

            // 1. Compute the dual of the intersection of `self` and `cut`. This
            //    is sort of like an amalgamation of every possible manifold
            //    that is perpendicular to `self` and `cut`.
            let perpendicularity = &self_ipns ^ &cut_ipns;

            if perpendicularity.is_zero() {
                // `self` and `cut` are the same object, so it's not on either
                // side.
                return Ok(ManifoldWhichSide::neither_side());
            }

            // 2. Wedge with an arbitrary point to select one of those possible
            //    perpendicular manifolds. The only restriction here is that we
            //    don't want the wedge product to be zero.
            let perpendicular_manifold = nonzero_wedge_with_arbitrary_point(&perpendicularity)?;

            // 3. Intersect that perpendicular manifold with `self` to get two
            //    points on `self`.
            (self_ipns ^ perpendicular_manifold.opns_to_ipns_in_space(space.opns()))
                .ipns_to_opns_in_space(space.opns())

            // There exists some conformal transformation `C` that turns
            // `perpendicular_manifold` into a flat line/plane/hyperplane and
            // make `self` and `cut` both circles/spheres/hyperspheres
            // perpendicular to it.
            //
            // `pair_on_self_across_cut` is the intersection of `self` and
            // `perpendicular_manifold`.
            //
            // After applying `C`, `pair_on_self_across_cut` consists of the two
            // points on `self` that are closest and farthest from
            // `perpendicular_manifold`. If any point on `self` is inside `cut`,
            // then the closest point will also be inside `cut`. And if any
            // point on `self` is outside `cut`, then the farthest point will
            // also be outside `cut`.
        };

        // Extract those two points.
        let Some([a, b]) = pair_on_self_across_cut.point_pair_to_points() else {
            bail!(
                "unable to query points \
                 on manifold: {self}\n\
                 within space: {space}\n\
                 relative to cut: {cut}\n\
                 using point pair: {pair_on_self_across_cut}"
            );
        };

        // Query whether each one is inside or outside of `cut`.
        Ok(query_point(&a, &cut_ipns)? | query_point(&b, &cut_ipns)?)
    }

    fn which_side_has_point(&self, p: &Self::Point, space: &Self) -> Result<ManifoldWhichSide> {
        query_point(p, &self.ipns_in_space(space))
    }
}

/// Selects an arbitrary point that is not on an object and wedges the object
/// with that point.
///
/// Returns an error if there is no such point, which should only happen if the
/// object is already zero.
fn nonzero_wedge_with_arbitrary_point(opns_obj: &Blade) -> Result<Blade> {
    let ndim = opns_obj.ndim() + 1;
    let candidates = (0..ndim)
        .map(|i| Blade::point(Vector::unit(i)))
        .chain([Blade::NO, Blade::NI]);
    candidates
        .map(|p| opns_obj ^ p)
        .find(|obj| !obj.is_zero())
        .ok_or_else(|| anyhow!("unable to find point not on object {opns_obj}"))
}

fn query_point(point: &Point, cut_ipns: &Blade) -> Result<ManifoldWhichSide> {
    let result = cut_ipns.ipns_query_point(point);
    Ok(ManifoldWhichSide {
        is_any_inside: result == PointQueryResult::Inside,
        is_any_outside: result == PointQueryResult::Outside,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cga_euclidean_intersection_orientation() {
        let expected_result = ManifoldWhichSide {
            is_any_inside: true,
            is_any_outside: false,
        };

        let obj_ipns = Blade::ipns_sphere(vector![], 1.0); // r=1 sphere
        let divider_ipns = Blade::ipns_plane(vector![1.0], 1.0); // x=1 plane

        for ndim in 1..=8 {
            println!("In {ndim}D ...");

            let space = EuclideanCgaManifold::whole_space(ndim);
            let obj = EuclideanCgaManifold::from_ipns(&obj_ipns, ndim).unwrap();
            let divider = EuclideanCgaManifold::from_ipns(&divider_ipns, ndim).unwrap();

            let result = obj.which_side(&divider, &space).unwrap();
            println!("result = {result:?}");
            assert_eq!(expected_result, result);

            for subspace_ndim in 1..ndim {
                let subspace =
                    EuclideanCgaManifold::from_opns(Blade::pseudoscalar(subspace_ndim), ndim)
                        .unwrap();
                let obj_in_subspace = subspace.intersect(&obj, &space).unwrap().unwrap();
                let divider_in_subspace = subspace.intersect(&divider, &space).unwrap().unwrap();

                let result = obj_in_subspace
                    .which_side(&divider_in_subspace, &subspace)
                    .unwrap();
                println!("  result in {subspace_ndim}D subspace: {result:?}");
                assert_eq!(expected_result, result);

                for subsubspace_ndim in 1..subspace_ndim {
                    let subsubspace = EuclideanCgaManifold::from_opns(
                        Blade::pseudoscalar(subsubspace_ndim),
                        ndim,
                    )
                    .unwrap();

                    // Let `obj` take a detour through `subspace`.
                    {
                        let obj_in_subsubspace = subsubspace
                            .intersect(&obj_in_subspace, &subspace)
                            .unwrap()
                            .unwrap();
                        let divider_in_subsubspace =
                            subsubspace.intersect(&divider, &space).unwrap().unwrap();

                        let result = obj_in_subsubspace
                            .which_side(&divider_in_subsubspace, &subsubspace)
                            .unwrap();
                        println!("    result in {subspace_ndim}D subsubspace: {result:?}");
                        assert_eq!(expected_result, result);
                    }

                    // Let `divider` take a detour through `subspace`.
                    {
                        let obj_in_subsubspace =
                            subsubspace.intersect(&obj, &space).unwrap().unwrap();
                        let divider_in_subsubspace = subsubspace
                            .intersect(&divider_in_subspace, &subspace)
                            .unwrap()
                            .unwrap();

                        let result = obj_in_subsubspace
                            .which_side(&divider_in_subsubspace, &subsubspace)
                            .unwrap();
                        println!("    result in {subspace_ndim}D subsubspace: {result:?}");
                        assert_eq!(expected_result, result);
                    }
                }
            }

            println!();
        }
    }
}