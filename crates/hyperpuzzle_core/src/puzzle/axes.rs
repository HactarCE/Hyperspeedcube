use std::sync::Arc;

use hypuz_util::ti::TypedIndexIter;

use super::*;
use crate::ComponentList;

/// System of axes for a puzzle.
#[derive(Debug)]
pub struct AxisSystem {
    /// Axis names.
    pub names: Arc<Names<Axis>>,
    /// Orbits used to generate axes.
    pub orbits: Vec<Orbit<Axis>>,
    /// Extra components.
    pub components: ComponentList<Self>,
}

impl AxisSystem {
    /// Returns an empty axis system.
    pub fn new_empty() -> Self {
        Self {
            names: Arc::new(Names::new_empty()),
            orbits: vec![],
            components: ComponentList::new(),
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

    /// Returns an iterator over all the axes.
    pub fn iter(&self) -> TypedIndexIter<Axis> {
        self.names.list().iter_keys()
    }
}
