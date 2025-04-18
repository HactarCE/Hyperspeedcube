use std::any::Any;

use crate::{Axis, Twist, Vantage};

/// Vantage group, which defines angles from which to view and interact with a
/// puzzle.
pub trait VantageGroup: Any + Send + Sync {
    /// Returns the number of vantages.
    fn vantage_count(&self) -> usize;

    /// Composes two vantage group elements.
    fn compose(
        &self,
        e1: BoxDynVantageGroupElement,
        e2: BoxDynVantageGroupElement,
    ) -> Option<BoxDynVantageGroupElement>;

    /// Transforms a vantage by a group element.
    fn transform_vantage(
        &self,
        elem: BoxDynVantageGroupElement,
        vantage: Vantage,
    ) -> Option<Vantage>;

    /// Transforms a vantage by a group element.
    fn transform_axis(
        &self,
        elem: BoxDynVantageGroupElement,
        axis: BoxDynRelativeAxis,
    ) -> Option<BoxDynRelativeAxis>;
    /// Transforms a twist by a group element.
    fn transform_twist(
        &self,
        elem: BoxDynVantageGroupElement,
        twist: BoxDynRelativeTwist,
    ) -> Option<BoxDynRelativeTwist>;

    /// Resolves a relative axis to an absolute one.
    fn resolve_axis(&self, vantage: Vantage, axis: BoxDynRelativeAxis) -> Option<Axis>;
    /// Resolves a relative twist to an absolute one.
    fn resolve_twist(&self, vantage: Vantage, twist: BoxDynRelativeTwist) -> Option<Twist>;

    /// Returns the name of a vantage group element, for saving in user
    /// preferences and log files.
    fn vantage_group_element_name(&self, elem: BoxDynVantageGroupElement) -> String;
    /// Returns the name of a vantage, for saving in log files.
    fn vantage_name(&self, vantage: Vantage) -> String;
    /// Returns the name of a relative axis.
    fn axis_name(&self, axis: BoxDynRelativeAxis) -> String;
    /// Returns the name of a relative twist.
    fn twist_name(&self, twist: BoxDynRelativeTwist) -> String;

    /// Returns the vantage group element with the given name.
    fn vantage_group_element_from_name(&self, name: String) -> Option<BoxDynVantageGroupElement>;
    /// Returns the vantage with the given name.
    fn vantage_from_name(&self, name: String) -> Option<Vantage>;
    /// Returns the relative axis with the given name.
    fn axis_from_name(&self, name: String) -> Option<BoxDynRelativeAxis>;
    /// Returns the relative twist with the given name.
    fn twist_from_name(&self, name: String) -> Option<BoxDynRelativeTwist>;
}
box_dyn_wrapper_struct! {
    /// Wrapper around `Arc<dyn VantageGroup>` that can be downcast to a
    /// concrete vantage group type.
    pub struct BoxDynVantageGroup(Box<dyn VantageGroup>);
}

/// Element of a vantage group.
pub trait VantageGroupElement: Any + Send + Sync {
    /// Returns a copy of the data.
    fn clone_dyn(&self) -> BoxDynVantageGroupElement;
}
box_dyn_wrapper_struct! {
    /// Wrapper around `Arc<dyn VantageGroupElement>` that can be downcast to a
    /// concrete vantage group element type. This type also implements [`Clone`]
    /// for conveninence.
    pub struct BoxDynVantageGroupElement(Box<dyn VantageGroupElement>);
}
impl_dyn_clone!(for BoxDynVantageGroupElement);

/// Relative location of an axis, which can be resolved at a given vantage.
pub trait RelativeAxis: Any + Send + Sync {
    /// Returns a copy of the data.
    fn clone_dyn(&self) -> BoxDynRelativeAxis;
    /// Returns whether two relative axes are equal.
    fn eq(&self, other: &BoxDynRelativeAxis) -> bool;
}
box_dyn_wrapper_struct! {
    /// Wrapper around `Arc<dyn RelativeAxis>` that can be downcast to a
    /// concrete relative axis type. This type also implements [`Clone`] for
    /// conveninence.
    pub struct BoxDynRelativeAxis(Box<dyn RelativeAxis>);
}
impl_dyn_clone!(for BoxDynRelativeAxis);

/// Relative location of an twist, which can be resolved at a given vantage.
pub trait RelativeTwist: Any + Send + Sync {
    /// Returns a copy of the data.
    fn clone_dyn(&self) -> BoxDynRelativeTwist;
}
box_dyn_wrapper_struct! {
    /// Wrapper around `Arc<dyn RelativeTwist>` that can be downcast to a
    /// concrete relative twist type. This type also implements [`Clone`] for
    /// conveninence.
    pub struct BoxDynRelativeTwist(Box<dyn RelativeTwist>);
}
impl_dyn_clone!(for BoxDynRelativeTwist);
