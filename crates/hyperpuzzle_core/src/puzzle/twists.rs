use std::sync::Arc;

use indexmap::IndexMap;

use super::*;
use crate::{
    BoxDynRelativeAxis, BoxDynRelativeTwist, BoxDynTwistSystemEngineData, BoxDynVantageGroup,
    BoxDynVantageGroupElement, NameSpecBiMap, VantageGroup, VantageGroupElement,
};

/// System of axes, twists, and vantages for a puzzle.
#[derive(Debug)]
pub struct TwistSystem {
    /// Twist system ID.
    pub id: String,
    /// Human-friendly name for the twist system.
    pub name: String,

    /// Axis system.
    pub axes: Arc<AxisSystem>,

    /// Twist names.
    pub names: NameSpecBiMap<Twist>,
    /// List of twists, indexed by ID.
    pub twists: PerTwist<TwistInfo>,
    /// Twist directions accessible in all vantage sets.
    pub directions: Vec<(String, PerAxis<Option<Twist>>)>,

    /// Vantage group.
    pub vantage_group: BoxDynVantageGroup,
    /// Built-in vantage sets.
    pub vantage_sets: IndexMap<String, VantageSetInfo>,

    /// Engine-specific data.
    pub engine_data: BoxDynTwistSystemEngineData,
}
impl TwistSystem {
    /// Returns an empty twist system.
    pub fn new_empty(axes: &Arc<AxisSystem>) -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            axes: Arc::clone(axes),
            names: NameSpecBiMap::new(),
            twists: PerTwist::new(),
            directions: vec![],
            vantage_group: ().into(),
            vantage_sets: IndexMap::new(),
            engine_data: ().into(),
        }
    }

    /// Returns whether the twist system has no twists.
    pub fn is_empty(&self) -> bool {
        self.twists.is_empty()
    }

    /// Returns the number of twists.
    pub fn len(&self) -> usize {
        self.twists.len()
    }
}

/// Vantage set.
#[derive(Debug)]
pub struct VantageSetInfo {
    /// User-friendly name for the vantage set.
    pub name: String,

    /// Map from name spec to transform.
    pub transform_map: Vec<(String, VantageTransformInfo)>,
    /// Map from name spec to relative axis.
    pub axis_map: Vec<(String, BoxDynRelativeAxis)>,
    /// Twist direction map.
    ///
    /// There should be at most one of these for each relative axis.
    pub direction_maps: Vec<AxisDirectionMap>,
}

/// Map from twist direction name to relative twist for a single relative axis.
#[derive(Debug)]
pub struct AxisDirectionMap {
    /// Axis for which this direction map applies.
    pub axis: BoxDynRelativeAxis,

    /// Map from name spec to twist.
    pub directions: Vec<(String, BoxDynRelativeTwist)>,
    /// Transform via which to inherit the direction map from another relative
    /// axis.
    pub inherit: Option<BoxDynVantageGroupElement>,
}

/// Rotation from one vantage to another.
#[derive(Debug)]
pub struct VantageTransformInfo {
    /// Transform to another vantage.
    pub transform: BoxDynVantageGroupElement,
    /// New vantage set to activate, if any.
    pub new_vantage_set: Option<String>,
}

impl VantageGroupElement for () {
    fn clone_dyn(&self) -> BoxDynVantageGroupElement {
        ().into()
    }
}

impl VantageGroup for () {
    fn vantage_count(&self) -> usize {
        1
    }

    fn compose(
        &self,
        _e1: BoxDynVantageGroupElement,
        _e2: BoxDynVantageGroupElement,
    ) -> Option<BoxDynVantageGroupElement> {
        Some(().into())
    }

    fn transform_vantage(
        &self,
        _elem: BoxDynVantageGroupElement,
        vantage: Vantage,
    ) -> Option<Vantage> {
        Some(vantage)
    }

    fn transform_axis(
        &self,
        _elem: BoxDynVantageGroupElement,
        axis: BoxDynRelativeAxis,
    ) -> Option<BoxDynRelativeAxis> {
        Some(axis)
    }

    fn transform_twist(
        &self,
        _elem: BoxDynVantageGroupElement,
        twist: BoxDynRelativeTwist,
    ) -> Option<BoxDynRelativeTwist> {
        Some(twist)
    }

    fn resolve_axis(&self, _vantage: Vantage, _axis: BoxDynRelativeAxis) -> Option<Axis> {
        None
    }

    fn resolve_twist(&self, _vantage: Vantage, _twist: BoxDynRelativeTwist) -> Option<Twist> {
        None
    }

    fn vantage_group_element_name(&self, _elem: BoxDynVantageGroupElement) -> String {
        "I".to_owned()
    }

    fn vantage_name(&self, _vantage: Vantage) -> String {
        "I".to_owned()
    }

    fn axis_name(&self, _axis: BoxDynRelativeAxis) -> String {
        String::new()
    }

    fn twist_name(&self, _twist: BoxDynRelativeTwist) -> String {
        String::new()
    }

    fn vantage_group_element_from_name(&self, _name: String) -> Option<BoxDynVantageGroupElement> {
        None
    }

    fn vantage_from_name(&self, _name: String) -> Option<Vantage> {
        None
    }

    fn axis_from_name(&self, _name: String) -> Option<BoxDynRelativeAxis> {
        None
    }

    fn twist_from_name(&self, _name: String) -> Option<BoxDynRelativeTwist> {
        None
    }
}
