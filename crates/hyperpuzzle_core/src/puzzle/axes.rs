use super::*;
use crate::NameSpecBiMap;

/// System of axes for a puzzle.
#[derive(Debug)]
pub struct AxisSystem {
    /// Axis names.
    pub names: NameSpecBiMap<Axis>,
    /// For each axis, its opposite axis if there is one.
    ///
    /// This is important for Slice Turn Metric calculations.
    pub opposites: PerAxis<Option<Axis>>,
}
impl AxisSystem {
    /// Returns an empty axis system.
    pub fn new_empty() -> Self {
        Self {
            names: NameSpecBiMap::new(),
            opposites: PerAxis::new(),
        }
    }

    /// Returns the number of axes.
    pub fn len(&self) -> usize {
        self.opposites.len()
    }
}
