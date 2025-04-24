use std::any::Any;

use eyre::OptionExt;

use crate::{Axis, NameSpecBiMap, Twist, Vantage};

/// [`VantageGroup`] that uses [`SimpleRelativeAxis`] and
/// [`SimpleRelativeTwist`]. Except where stated otherwise, they are always
/// expected to be simplified.
///
/// This is slightly easier to implement than [`VantageGroup`]
pub trait SimpleVantageGroup: Any + Send + Sync {
    /// Concrete vantage group element type.
    type Element: VantageGroupElement + Clone;

    /// Returns the number of vantages.
    fn vantage_count_concrete(&self) -> usize;

    /// Returns the identity element of the vantage group.
    fn identity_concrete(&self) -> Self::Element;

    /// Composes two vantage group elements.
    fn compose_concrete(&self, e1: Self::Element, e2: Self::Element) -> Option<Self::Element>;

    /// Transforms a vantage by a group element.
    fn transform_vantage_concrete(&self, elem: Self::Element, vantage: Vantage) -> Option<Vantage>;

    /// Transforms a vantage by a group element.
    fn transform_axis_concrete(
        &self,
        elem: Self::Element,
        axis: SimpleRelativeAxis<Self>,
    ) -> Option<SimpleRelativeAxis<Self>> {
        let new_relative_axis = SimpleRelativeAxis {
            absolute_axis: axis.absolute_axis,
            transform: self.compose_concrete(elem, axis.transform)?,
        };

        // Try to use identity transform if possible.
        match self.resolve_axis_concrete(Vantage::INITIAL, new_relative_axis.clone()) {
            Some(absolute_axis) => Some(SimpleRelativeAxis {
                absolute_axis,
                transform: self.identity_concrete(),
            }),
            None => Some(new_relative_axis),
        }
    }
    /// Transforms a twist by a group element.
    fn transform_twist_concrete(
        &self,
        elem: Self::Element,
        twist: SimpleRelativeTwist<Self>,
    ) -> Option<SimpleRelativeTwist<Self>> {
        let new_relative_twist = SimpleRelativeTwist {
            absolute_twist: twist.absolute_twist,
            transform: self.compose_concrete(elem, twist.transform)?,
        };

        // Try to use identity transform if possible.
        match self.resolve_twist_concrete(Vantage::INITIAL, new_relative_twist.clone()) {
            Some(absolute_twist) => Some(SimpleRelativeTwist {
                absolute_twist,
                transform: self.identity_concrete(),
            }),
            None => Some(new_relative_twist),
        }
    }

    /// Resolves a relative axis to an absolute one.
    ///
    /// **`axis` might not be simplified!**
    fn resolve_axis_concrete(
        &self,
        vantage: Vantage,
        axis: SimpleRelativeAxis<Self>,
    ) -> Option<Axis>;
    /// Resolves a relative twist to an absolute one.
    ///
    /// **`twist` might not be simplified!**
    fn resolve_twist_concrete(
        &self,
        vantage: Vantage,
        twist: SimpleRelativeTwist<Self>,
    ) -> Option<Twist>;

    /// Returns the name of a vantage group element, for saving in user
    /// preferences and log files.
    fn vantage_group_element_name_concrete(&self, elem: Self::Element) -> eyre::Result<String>;
    /// Returns the name of a vantage, for saving in log files.
    fn vantage_name_concrete(&self, vantage: Vantage) -> eyre::Result<String>;
    /// Returns the name of a relative axis.
    fn axis_name_concrete(&self, axis: SimpleRelativeAxis<Self>) -> eyre::Result<String> {
        let axis_name = &self.axis_names()[axis.absolute_axis];
        if self.is_identity(&axis.transform) {
            Ok(axis_name.to_string())
        } else {
            let transform_name = self.vantage_group_element_name_concrete(axis.transform)?;
            Ok(format!("{axis_name}@{transform_name}"))
        }
    }
    /// Returns the name of a relative twist.
    fn twist_name_concrete(&self, twist: SimpleRelativeTwist<Self>) -> eyre::Result<String> {
        let twist_name = &self.twist_names()[twist.absolute_twist];
        if self.is_identity(&twist.transform) {
            Ok(twist_name.to_string())
        } else {
            let transform_name = self.vantage_group_element_name_concrete(twist.transform)?;
            Ok(format!("{twist_name}@{transform_name}"))
        }
    }

    /// Returns the vantage group element with the given name.
    fn vantage_group_element_from_name_concrete(&self, name: &str) -> Option<Self::Element>;
    /// Returns the vantage with the given name.
    fn vantage_from_name_concrete(&self, name: &str) -> Option<Vantage>;
    /// Returns the relative axis with the given name.
    fn axis_from_name_concrete(&self, name: &str) -> Option<SimpleRelativeAxis<Self>> {
        match name.split_once('@') {
            Some((axis_name, elem_name)) => Some(SimpleRelativeAxis {
                absolute_axis: self.axis_names().id_from_name(axis_name)?,
                transform: self.vantage_group_element_from_name_concrete(elem_name)?,
            }),
            None => Some(SimpleRelativeAxis {
                absolute_axis: self.axis_names().id_from_name(name)?,
                transform: self.identity_concrete(),
            }),
        }
    }
    /// Returns the relative twist with the given name.
    fn twist_from_name_concrete(&self, name: &str) -> Option<SimpleRelativeTwist<Self>> {
        match name.split_once('@') {
            Some((twist_name, elem_name)) => Some(SimpleRelativeTwist {
                absolute_twist: self.twist_names().id_from_name(twist_name)?,
                transform: self.vantage_group_element_from_name_concrete(elem_name)?,
            }),
            None => Some(SimpleRelativeTwist {
                absolute_twist: self.twist_names().id_from_name(name)?,
                transform: self.identity_concrete(),
            }),
        }
    }

    /// Returns whether `elem` is equivalent to the identity vantage group
    /// element.
    fn is_identity(&self, elem: &Self::Element) -> bool;
    /// Returns the map of axis names.
    fn axis_names(&self) -> &NameSpecBiMap<Axis>;
    /// Returns the map of twist names.
    fn twist_names(&self) -> &NameSpecBiMap<Twist>;
}
impl<G: SimpleVantageGroup> VantageGroup for G {
    fn vantage_count(&self) -> usize {
        self.vantage_count_concrete()
    }

    fn identity(&self) -> BoxDynVantageGroupElement {
        self.identity_concrete().into()
    }

    fn compose(
        &self,
        e1: BoxDynVantageGroupElement,
        e2: BoxDynVantageGroupElement,
    ) -> Option<BoxDynVantageGroupElement> {
        self.compose_concrete(*e1.downcast()?, *e2.downcast()?)
            .map(BoxDynVantageGroupElement::new)
    }

    fn transform_vantage(
        &self,
        elem: BoxDynVantageGroupElement,
        vantage: Vantage,
    ) -> Option<Vantage> {
        self.transform_vantage_concrete(*elem.downcast()?, vantage)
    }

    fn transform_axis(
        &self,
        elem: BoxDynVantageGroupElement,
        axis: BoxDynRelativeAxis,
    ) -> Option<BoxDynRelativeAxis> {
        self.transform_axis_concrete(*elem.downcast()?, *axis.downcast()?)
            .map(BoxDynRelativeAxis::new)
    }

    fn transform_twist(
        &self,
        elem: BoxDynVantageGroupElement,
        twist: BoxDynRelativeTwist,
    ) -> Option<BoxDynRelativeTwist> {
        self.transform_twist_concrete(*elem.downcast()?, *twist.downcast()?)
            .map(BoxDynRelativeTwist::new)
    }

    fn resolve_axis(&self, vantage: Vantage, axis: BoxDynRelativeAxis) -> Option<Axis> {
        self.resolve_axis_concrete(vantage, *axis.downcast()?)
    }

    fn resolve_twist(&self, vantage: Vantage, twist: BoxDynRelativeTwist) -> Option<Twist> {
        self.resolve_twist_concrete(vantage, *twist.downcast()?)
    }

    fn vantage_group_element_name(&self, elem: BoxDynVantageGroupElement) -> eyre::Result<String> {
        self.vantage_group_element_name_concrete(
            *elem.downcast().ok_or_eyre("bad type for relative twist")?,
        )
    }

    fn vantage_name(&self, vantage: Vantage) -> eyre::Result<String> {
        self.vantage_name_concrete(vantage)
    }

    fn axis_name(&self, axis: BoxDynRelativeAxis) -> eyre::Result<String> {
        self.axis_name_concrete(*axis.downcast().ok_or_eyre("bad type for relative axis")?)
    }

    fn twist_name(&self, twist: BoxDynRelativeTwist) -> eyre::Result<String> {
        self.twist_name_concrete(*twist.downcast().ok_or_eyre("bad type for relative twist")?)
    }

    fn vantage_group_element_from_name(&self, name: &str) -> Option<BoxDynVantageGroupElement> {
        self.vantage_group_element_from_name_concrete(name)
            .map(BoxDynVantageGroupElement::new)
    }

    fn vantage_from_name(&self, name: &str) -> Option<Vantage> {
        self.vantage_from_name_concrete(name)
    }

    fn axis_from_name(&self, name: &str) -> Option<BoxDynRelativeAxis> {
        self.axis_from_name_concrete(name)
            .map(BoxDynRelativeAxis::new)
    }

    fn twist_from_name(&self, name: &str) -> Option<BoxDynRelativeTwist> {
        self.twist_from_name_concrete(name)
            .map(BoxDynRelativeTwist::new)
    }
}

/// Vantage group, which defines angles from which to view and interact with a
/// puzzle.
pub trait VantageGroup: Any + Send + Sync {
    /// Returns the number of vantages.
    fn vantage_count(&self) -> usize;

    /// Returns the identity element of the vantage group.
    fn identity(&self) -> BoxDynVantageGroupElement;

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
    fn vantage_group_element_name(&self, elem: BoxDynVantageGroupElement) -> eyre::Result<String>;
    /// Returns the name of a vantage, for saving in log files.
    fn vantage_name(&self, vantage: Vantage) -> eyre::Result<String>;
    /// Returns the name of a relative axis.
    fn axis_name(&self, axis: BoxDynRelativeAxis) -> eyre::Result<String>;
    /// Returns the name of a relative twist.
    fn twist_name(&self, twist: BoxDynRelativeTwist) -> eyre::Result<String>;

    /// Returns the vantage group element with the given name.
    fn vantage_group_element_from_name(&self, name: &str) -> Option<BoxDynVantageGroupElement>;
    /// Returns the vantage with the given name.
    fn vantage_from_name(&self, name: &str) -> Option<Vantage>;
    /// Returns the relative axis with the given name.
    fn axis_from_name(&self, name: &str) -> Option<BoxDynRelativeAxis>;
    /// Returns the relative twist with the given name.
    fn twist_from_name(&self, name: &str) -> Option<BoxDynRelativeTwist>;
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

/// [`RelativeAxis`] consisting of an absolute axis and a vantage group element
/// `E`. For most vantage groups, this is sufficient to specify a relative axis.
///
/// This is insufficient for some puzzles on strange geometries, such as
/// orbifolds.
#[derive(Debug)]
pub struct SimpleRelativeAxis<G: SimpleVantageGroup + ?Sized> {
    /// Absolute axis that this relative axis is similar to.
    pub absolute_axis: Axis,
    /// Vantage group element to apply to `absolute_axis`.
    pub transform: G::Element,
}
impl<G: SimpleVantageGroup + ?Sized> Clone for SimpleRelativeAxis<G> {
    fn clone(&self) -> Self {
        Self {
            absolute_axis: self.absolute_axis,
            transform: self.transform.clone(),
        }
    }
}
impl<G: SimpleVantageGroup + ?Sized> Copy for SimpleRelativeAxis<G> where G::Element: Copy {}
impl<G: SimpleVantageGroup + ?Sized> RelativeAxis for SimpleRelativeAxis<G>
where
    G::Element: Clone,
{
    fn clone_dyn(&self) -> BoxDynRelativeAxis {
        self.clone().into()
    }
}

/// [`RelativeTwist`] consisting of an absolute twist and a vantage group
/// element `E`. For most vantage groups, this is sufficient to specify a
/// relative twist.
///
/// This is insufficient for some puzzles on strange geometries, such as
/// orbifolds.
#[derive(Debug)]
pub struct SimpleRelativeTwist<G: SimpleVantageGroup + ?Sized> {
    /// Absolute twist that this relative twist is similar to.
    pub absolute_twist: Twist,
    /// Vantage group element to apply to `absolute_twist`.
    pub transform: G::Element,
}
impl<G: SimpleVantageGroup + ?Sized> Clone for SimpleRelativeTwist<G> {
    fn clone(&self) -> Self {
        Self {
            absolute_twist: self.absolute_twist,
            transform: self.transform.clone(),
        }
    }
}
impl<G: SimpleVantageGroup + ?Sized> Copy for SimpleRelativeTwist<G> where G::Element: Copy {}
impl<G: SimpleVantageGroup + ?Sized> RelativeTwist for SimpleRelativeTwist<G>
where
    G::Element: Clone,
{
    fn clone_dyn(&self) -> BoxDynRelativeTwist {
        self.clone().into()
    }
}

/// Relative location of an axis, which can be resolved at a given vantage.
pub trait RelativeAxis: Any + Send + Sync {
    /// Returns a copy of the data.
    fn clone_dyn(&self) -> BoxDynRelativeAxis;
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
