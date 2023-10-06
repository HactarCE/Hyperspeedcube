use hypermath::collections::approx_hashmap::{self, ApproxHashMapKey};

use super::*;

/// Planar or spherical manifold represented using a blade.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifoldData {
    /// Number of dimensions of the manifold.
    pub(super) ndim: u8,
    /// OPNS blade representing the manifold.
    pub blade: Blade,
}

impl ApproxHashMapKey for ManifoldData {
    type Hash = <Blade as ApproxHashMapKey>::Hash;

    fn approx_hash(
        &self,
        float_hash_fn: impl FnMut(Float) -> approx_hashmap::FloatHash,
    ) -> Self::Hash {
        self.blade.approx_hash(float_hash_fn)
    }
}

impl fmt::Display for ManifoldData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ndim {
            0 => {
                let [a, b] = self.blade.point_pair_to_points().expect("bad point pair");
                write!(f, "point pair {a}..{b}")?;
            }
            1 => write!(f, "line {}", self.blade)?,
            2 => write!(f, "plane {}", self.blade)?,
            _ => write!(f, "{}", self.blade)?,
        }
        Ok(())
    }
}

impl ManifoldData {
    /// Constructs a manifold from a blade.
    pub fn new(blade: Blade) -> Result<Self> {
        let ndim = blade
            .grade()
            .checked_sub(2)
            .ok_or_else(|| eyre!("blade has too low of a grade"))?;
        Ok(ManifoldData { ndim, blade })
    }

    /// Returns the number of dimensions of the manifold.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }
}

impl Mul<Sign> for ManifoldId {
    type Output = ManifoldRef;

    fn mul(self, rhs: Sign) -> Self::Output {
        ManifoldRef {
            id: self,
            sign: rhs,
        }
    }
}
