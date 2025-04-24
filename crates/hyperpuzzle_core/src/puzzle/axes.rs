use std::sync::Arc;

use super::*;
use crate::NameSpecBiMap;

/// System of axes for a puzzle.
#[derive(Debug)]
pub struct AxisSystem {
    /// Axis names.
    pub names: Arc<NameSpecBiMap<Axis>>,

    /// Orbits used to generate axes.
    pub orbits: Vec<Orbit<Axis>>,
}
impl AxisSystem {
    /// Returns an empty axis system.
    pub fn new_empty() -> Self {
        Self {
            names: Arc::new(NameSpecBiMap::new()),
            orbits: vec![],
        }
    }

    /// Returns whether there are no axes in the axis system.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
    /// Returns the number of axes in the axis system.
    pub fn len(&self) -> usize {
        self.names.len()
    }
}
