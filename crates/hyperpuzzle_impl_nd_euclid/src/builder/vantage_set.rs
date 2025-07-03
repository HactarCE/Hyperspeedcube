use eyre::{OptionExt, Result, WrapErr, bail, eyre};
use hypermath::pga;
use hyperpuzzle_core::prelude::*;
use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    NdEuclidRelativeAxis, NdEuclidRelativeTwist, NdEuclidVantageGroup, NdEuclidVantageGroupElement,
    NdEuclidVantageSetEngineData,
};

/// Vantage set during puzzle construction.
#[derive(Debug)]
pub struct VantageSetBuilder {
    /// Human-friendly name.
    pub name: String,
    /// ID of the vantage group used by this vantage set.
    pub group: String,

    /// View offset.
    pub view_offset: pga::Motor,

    /// Relative rotations, indexed by human-friendly name.
    pub transforms: Vec<(String, pga::Motor)>,
    /// Relative axes, indexed by human-friendly name.
    pub axes: Vec<(String, RelativeAxisBuilder)>,
}
impl VantageSetBuilder {
    /// Validates and constructs a vantage set.
    pub fn build(&self, groups: &IndexMap<String, NdEuclidVantageGroup>) -> Result<VantageSet> {
        let group = groups
            .get(&self.group)
            .ok_or_else(|| eyre!("no vantage group with name {:?}", self.group))?;

        if !self.transforms.iter().map(|(name, _)| name).all_unique() {
            bail!("transform names are not all unique");
        }
        if !self.axes.iter().map(|(name, _)| name).all_unique() {
            bail!("axis names are not all unique");
        }

        let transform_map = self
            .transforms
            .iter()
            .sorted_by_cached_key(|(name, _)| NameSpec::sort_key(name.clone()))
            .map(|(name, rot)| {
                let transform = try_element_from_motor(group, rot)
                    .wrap_err_with(|| format!("building relative transform {name:?}"))?
                    .into();
                let vantage_transform = VantageTransformInfo {
                    transform,
                    new_vantage_set: None,
                };
                eyre::Ok((name.clone(), vantage_transform))
            })
            .try_collect()?;

        let axis_map = self
            .axes
            .iter()
            .sorted_by_cached_key(|(name, _)| NameSpec::sort_key(name.clone()))
            .map(|(name, axis)| {
                let (relative_axis, direction_map) = axis
                    .build(group)
                    .wrap_err_with(|| format!("building relative axis {name:?}"))?;
                eyre::Ok((name.clone(), relative_axis, direction_map))
            })
            .try_collect()?;

        let engine_data = NdEuclidVantageSetEngineData {
            view_offset: self.view_offset.clone(),
        };

        Ok(VantageSet {
            name: self.name.clone(),

            group: self.group.clone(),

            transform_map,
            axis_map,

            engine_data: engine_data.into(),
        })
    }

    /// "Unbuilds" a vantage set.
    pub fn unbuild(
        vantage_set: &VantageSet,
        groups: &IndexMap<String, NdEuclidVantageGroup>,
    ) -> Result<Self> {
        let VantageSet {
            name,
            group: group_name,
            transform_map,
            axis_map,
            engine_data,
        } = vantage_set;

        let group = groups
            .get(group_name)
            .ok_or_else(|| eyre!("no vantage group with name {group_name:?}"))?;

        let NdEuclidVantageSetEngineData { view_offset } = engine_data
            .downcast_ref()
            .ok_or_eyre("expected NdEuclid vantage set")?;

        let transforms = transform_map
            .iter()
            .map(|(name, transform)| {
                let motor = try_motor_from_element(group, &transform.transform)?;
                eyre::Ok((name.clone(), motor.clone()))
            })
            .try_collect()?;

        let axes = axis_map
            .iter()
            .map(|(name, axis, direction_map)| {
                let relative_axis_builder =
                    RelativeAxisBuilder::unbuild(axis, direction_map, group)?;
                eyre::Ok((name.clone(), relative_axis_builder))
            })
            .try_collect()?;

        Ok(Self {
            name: name.clone(),
            group: group_name.clone(),
            view_offset: view_offset.clone(),
            transforms,
            axes,
        })
    }
}

/// Twist directions for a relative axis during puzzle construction.
#[derive(Debug)]
pub struct AxisDirectionMapBuilder {
    /// Map from name spec to twist.
    pub directions: Vec<(String, RelativeTwistBuilder)>,
    /// Transform via which to inherit the direction map from another relative
    /// axis.
    pub inherit: Option<pga::Motor>,
}
impl AxisDirectionMapBuilder {
    /// Validates and constructs an axis direction map.
    pub fn build(&self, group: &NdEuclidVantageGroup) -> Result<AxisDirectionMap> {
        if !self.directions.iter().map(|(name, _)| name).all_unique() {
            bail!("direction names are not all unique per axis");
        }

        Ok(AxisDirectionMap {
            directions: self
                .directions
                .iter()
                .sorted_by_cached_key(|(name, _)| NameSpec::sort_key(name.clone()))
                .map(|(name, twist)| {
                    let relative_twist = twist
                        .build(group)
                        .wrap_err_with(|| format!("building twist direction {name:?}"))?;
                    eyre::Ok((name.clone(), relative_twist))
                })
                .try_collect()?,
            inherit: match &self.inherit {
                Some(motor) => Some(
                    try_element_from_motor(group, motor)
                        .map(BoxDynVantageGroupElement::new)
                        .wrap_err("building inheritance field")?,
                ),
                None => None,
            },
        })
    }

    /// "Unbuilds" an axis direction map.
    pub fn unbuild(direction_map: &AxisDirectionMap, group: &NdEuclidVantageGroup) -> Result<Self> {
        Ok(AxisDirectionMapBuilder {
            directions: direction_map
                .directions
                .iter()
                .map(|(name, relative_twist)| {
                    let relative_twist_builder =
                        RelativeTwistBuilder::unbuild(relative_twist, group)?;
                    eyre::Ok((name.clone(), relative_twist_builder))
                })
                .try_collect()?,
            inherit: match &direction_map.inherit {
                Some(inherit) => Some(try_motor_from_element(group, inherit)?.clone()),
                None => None,
            },
        })
    }
}

/// Relative axis during puzzle construction.
///
/// A relative axis is defined in terms of an absolute axis at a particular
/// vantage.
#[derive(Debug)]
pub struct RelativeAxisBuilder {
    /// Axis from one vantage.
    pub absolute_axis: Axis,
    /// Transform representing the vantage from which this relative axis
    /// resolves to `absolute_axis`.
    pub transform: pga::Motor,
    /// Twist directions for the relative axis.
    pub direction_map: AxisDirectionMapBuilder,
}
impl RelativeAxisBuilder {
    /// Validates and constructs a relative axis.
    fn build(
        &self,
        group: &NdEuclidVantageGroup,
    ) -> Result<(BoxDynRelativeAxis, AxisDirectionMap)> {
        let elem = try_element_from_motor(group, &self.transform)
            .wrap_err("constructing relative axis")?;
        let initial_relative_axis = NdEuclidRelativeAxis {
            absolute_axis: self.absolute_axis,
            transform: NdEuclidVantageGroupElement::IDENTITY,
        };
        let relative_axis = group
            .transform_axis_concrete(elem, initial_relative_axis)
            .ok_or_eyre("error constructing relative axis")
            .map(BoxDynRelativeAxis::new)?;
        let direction_map = self.direction_map.build(group)?;
        Ok((relative_axis, direction_map))
    }

    /// "Unbuilds" a relative axis.
    fn unbuild(
        axis: &BoxDynRelativeAxis,
        direction_map: &AxisDirectionMap,
        group: &NdEuclidVantageGroup,
    ) -> Result<Self> {
        let NdEuclidRelativeAxis {
            absolute_axis,
            transform,
        } = *axis
            .downcast_ref()
            .ok_or_eyre("expected NdEuclid relative axis")?;

        Ok(Self {
            absolute_axis,
            transform: group.group_element_motor(transform).clone(),
            direction_map: AxisDirectionMapBuilder::unbuild(direction_map, group)?,
        })
    }
}

/// Relative twist during puzzle construction.
///
/// A relative twist is defined in terms of an absolute twist at a particular
/// vantage.
#[derive(Debug)]
pub struct RelativeTwistBuilder {
    /// Twist from one vantage.
    pub absolute_twist: Twist,
    /// Transform representing the vantage from which this relative twist
    /// resolves to `absolute_twist`.
    pub transform: pga::Motor,
}
impl RelativeTwistBuilder {
    /// Validates and constructs a relative twist.
    fn build(&self, group: &NdEuclidVantageGroup) -> Result<BoxDynRelativeTwist> {
        let elem = try_element_from_motor(group, &self.transform)
            .wrap_err("constructing relative twist")?;
        let initial_relative_twist = NdEuclidRelativeTwist {
            absolute_twist: self.absolute_twist,
            transform: NdEuclidVantageGroupElement::IDENTITY,
        };
        group
            .transform_twist_concrete(elem, initial_relative_twist)
            .ok_or_eyre("error constructing relative twist")
            .map(BoxDynRelativeTwist::new)
    }

    /// "Unbuilds" a relative twist.
    fn unbuild(twist: &BoxDynRelativeTwist, group: &NdEuclidVantageGroup) -> Result<Self> {
        let NdEuclidRelativeTwist {
            absolute_twist,
            transform,
        } = *twist
            .downcast_ref()
            .ok_or_eyre("expected NdEuclid relative twist")?;

        Ok(Self {
            absolute_twist,
            transform: group.group_element_motor(transform).clone(),
        })
    }
}

fn try_element_from_motor(
    group: &NdEuclidVantageGroup,
    motor: &pga::Motor,
) -> Result<NdEuclidVantageGroupElement> {
    group
        .symmetry
        .element_from_motor(motor)
        .ok_or_eyre("no matching group element")
        .map(NdEuclidVantageGroupElement)
}

fn try_motor_from_element<'a>(
    group: &'a NdEuclidVantageGroup,
    element: &BoxDynVantageGroupElement,
) -> Result<&'a pga::Motor> {
    Ok(group.group_element_motor(
        *element
            .downcast_ref()
            .ok_or_eyre("expected NdEuclid vantage group element")?,
    ))
}
