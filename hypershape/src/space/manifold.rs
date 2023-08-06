use super::*;

/// Data for a manifold in a space.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifoldData {
    /// Number of dimensions of the manifold.
    pub ndim: u8,
    /// OPNS blade representing the manifold.
    pub blade: Blade,
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
            .context("blade has too low of a grade")?;
        Ok(ManifoldData { ndim, blade })
    }
}
