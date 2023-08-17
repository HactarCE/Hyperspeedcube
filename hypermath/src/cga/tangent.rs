use float_ord::FloatOrd;

use crate::*;

/// Tangent space for a manifold at a point.
#[derive(Debug, Clone)]
pub struct TangentSpace(TangentSpaceInner);
impl From<&Blade> for TangentSpace {
    fn from(blade: &Blade) -> Self {
        if blade.opns_is_flat() {
            match flat_spanning_basis(blade) {
                Some(basis) => TangentSpace(TangentSpaceInner::Flat(basis)),
                None => TangentSpace(TangentSpaceInner::Error),
            }
        } else {
            let space_ndim = blade.ndim();
            let ipns_manifold = blade.opns_to_ipns(space_ndim);
            TangentSpace(TangentSpaceInner::Curved {
                space_ndim,
                ipns_manifold,
            })
        }
    }
}
impl TangentSpace {
    /// Returns a minimal set of vectors that spans the tangent space of a
    /// manifold at `point`, or `None` if the manifold is degenerate. The point
    /// is assumed to be on the manifold.
    pub fn at(&self, point: impl ToConformalPoint) -> Option<Vec<Vector>> {
        match &self.0 {
            TangentSpaceInner::Error => None,
            TangentSpaceInner::Flat(result) => Some(result.clone()),
            TangentSpaceInner::Curved {
                space_ndim,
                ipns_manifold,
            } => {
                // (self & !p) ^ ni
                let perpendicular_bundle = ipns_manifold ^ Blade::point(point);
                let parallel_bundle = perpendicular_bundle.ipns_to_opns(*space_ndim);
                let tangent_manifold = parallel_bundle ^ Blade::NI;
                flat_spanning_basis(&tangent_manifold)
            }
        }
    }
}

#[derive(Debug, Clone)]
enum TangentSpaceInner {
    Error,
    Flat(Vec<Vector>),
    Curved {
        space_ndim: u8,
        ipns_manifold: Blade,
    },
}

/// Returns a set of vectors spanning the tangent space of a flat manifold
/// (line, plane, etc.) represented by an OPNS blade, or `None` if the blade is
/// degenerate.
fn flat_spanning_basis(blade: &Blade) -> Option<Vec<Vector>> {
    let space_ndim = blade.ndim();
    let mut dual_space = blade.opns_to_ipns(space_ndim);
    let mut spanning_vectors = vec![];

    while dual_space.grade() < space_ndim {
        // Take a unit vector along each axis. Wedge it with `dual_space` to see
        // what would happen if we added it to our spanning set. Take the one
        // that gives the maximum value (i.e., is most perpendicular to the
        // existing spanning set within `blade`)
        let new_dual_space = (0..space_ndim)
            .map(|axis| &dual_space ^ Blade::vector(Vector::unit(axis)))
            .max_by_key(|m| FloatOrd(m.dot(m).abs()))?;
        let old_dual_space_inv = dual_space.inverse()?;
        let new_tangent_vector = (old_dual_space_inv << &new_dual_space)
            .to_vector()
            .normalize()?;
        spanning_vectors.push(new_tangent_vector);
        dual_space = new_dual_space
    }
    if blade.grade() - 2 != spanning_vectors.len() as u8 {
        log::error!("dimension of tangent space does not match manifold dimension");
        return None;
    }
    Some(spanning_vectors)
}
