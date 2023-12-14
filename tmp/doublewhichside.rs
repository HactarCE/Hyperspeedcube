/// Location of two (N-1)-dimensional manifolds relative to each other.
pub enum DoubleWhichSide {
    /// The manifolds are the same, potentially with a sign difference.
    Same { sign: Sign },
    /// The manifolds intersect.
    Intersect,
    /// The manifolds do not intersect, except perhaps a single point of
    /// tangency.
    NonIntersecting {
        a_contains_b: bool,
        b_contains_a: bool,
        is_tangent: bool,
    },
}

/// Returns the location of two (N-1)-dimensional manifolds in N-dimensional
/// space relative to each other.
fn which_sides_have_manifolds(
    &self,
    space: ManifoldRef,
    a: ManifoldRef,
    b: ManifoldRef,
) -> Result<DoubleWhichSide> {
    let n = self.ndim_of(space);
    ensure!(self.ndim_of(a) + 1 == n);
    ensure!(self.ndim_of(b) + 1 == n);

    let a_contains_b;
    let is_tangent;
    match self.which_side_has_manifold(space, a, b.id)? {
        WhichSide::Flush => {
            ensure!(a.id == b.id, "flush manifolds must have same ID");
            return Ok(DoubleWhichSide::Same {
                sign: a.sign * b.sign,
            });
        }
        WhichSide::Inside { is_touching } => {
            a_contains_b = true;
            is_tangent = is_touching;
        }
        WhichSide::Outside { is_touching } => {
            a_contains_b = false;
            is_tangent = is_touching;
        }
        WhichSide::Split => return Ok(DoubleWhichSide::Intersect),
    }

    let b_contains_a;
    match self.which_side_has_manifold(space, b, a.id)? {
        WhichSide::Flush | WhichSide::Split => bail!("asymmetric manifold location"),
        WhichSide::Inside { .. } => b_contains_a = true,
        WhichSide::Outside { .. } => b_contains_a = false,
    }

    Ok(DoubleWhichSide::NonIntersecting {
        a_contains_b,
        b_contains_a,
        is_tangent,
    })
}
