use std::sync::Arc;

use eyre::Result;
use hypermath::collections::GenericVec;
use hypermath::{Vector, approx_eq, pga};
use hyperpuzzle_core::prelude::*;
use hypershape::{Group, GroupElementId, IsometryGroup};
use itertools::Itertools;
use smallvec::SmallVec;

use crate::{NdEuclidTwistSystemEngineData, TwistKey};

hypermath::idx_struct! {
    /// ID of a reference vector in a vantage group.
    pub struct ReferenceVector(u16);
}

/// List containing a value per reference vector.
pub type PerReferenceVector<T> = GenericVec<ReferenceVector, T>;

/// Vantage group for an N-dimensional Euclidean puzzle that is based on a
/// finite isometry group of N-dimensional Euclidean space.
#[derive(Debug, Clone)]
pub struct NdEuclidVantageGroup {
    pub(crate) symmetry: IsometryGroup,

    pub(crate) reference_vectors: PerReferenceVector<Vector>,
    pub(crate) reference_vector_names: NameSpecBiMap<ReferenceVector>,
    pub(crate) preferred_reference_vectors: Vec<ReferenceVector>,

    pub(crate) vantage_names: PerVantage<SmallVec<[(ReferenceVector, ReferenceVector); 4]>>,

    pub(crate) axis_names: Arc<NameSpecBiMap<Axis>>,
    pub(crate) twist_names: Arc<NameSpecBiMap<Twist>>,

    /// Map from twist to twist axis.
    ///
    /// This information exists in the twist system too, but we don't really
    /// want to hold a reference to the twist system because that would be a
    /// circular reference which is slightly awkward to construct.
    pub(crate) twist_axes: Arc<PerTwist<Axis>>,
    pub(crate) twist_system_engine_data: NdEuclidTwistSystemEngineData,
}

impl NdEuclidVantageGroup {
    pub fn vantage_to_group_element(&self, vantage: Vantage) -> Option<GroupElementId> {
        Some(GroupElementId(u16::try_from(vantage.0).ok()?))
    }
    pub fn group_element_to_vantage(&self, element: GroupElementId) -> Vantage {
        Vantage(u32::from(element.0))
    }
    pub fn group_element_to_vantage_concrete(
        &self,
        element: NdEuclidVantageGroupElement,
    ) -> Vantage {
        self.group_element_to_vantage(element.0)
    }
    pub fn vantage_motor(&self, vantage: Vantage) -> Option<&pga::Motor> {
        Some(&self.symmetry[self.vantage_to_group_element(vantage)?])
    }
    pub fn group_element_motor(&self, element: NdEuclidVantageGroupElement) -> &pga::Motor {
        &self.symmetry[element.0]
    }
}

pub type NdEuclidRelativeAxis = SimpleRelativeAxis<NdEuclidVantageGroup>;
pub type NdEuclidRelativeTwist = SimpleRelativeTwist<NdEuclidVantageGroup>;

impl SimpleVantageGroup for NdEuclidVantageGroup {
    type Element = NdEuclidVantageGroupElement;

    fn vantage_count_concrete(&self) -> usize {
        self.symmetry.element_count()
    }

    fn identity_concrete(&self) -> NdEuclidVantageGroupElement {
        NdEuclidVantageGroupElement::IDENTITY
    }

    fn compose_concrete(
        &self,
        e1: NdEuclidVantageGroupElement,
        e2: NdEuclidVantageGroupElement,
    ) -> Option<NdEuclidVantageGroupElement> {
        Some(NdEuclidVantageGroupElement(
            self.symmetry.compose(e1.0, e2.0),
        ))
    }

    fn transform_vantage_concrete(
        &self,
        elem: NdEuclidVantageGroupElement,
        vantage: Vantage,
    ) -> Option<Vantage> {
        let vantage_elem = self.vantage_to_group_element(vantage)?;
        let new_elem = self.symmetry.compose(elem.0, vantage_elem);
        Some(self.group_element_to_vantage(new_elem))
    }

    fn resolve_axis_concrete(&self, vantage: Vantage, axis: NdEuclidRelativeAxis) -> Option<Axis> {
        if self.is_identity(&axis.transform) {
            Some(axis.absolute_axis) // optimization
        } else {
            let axis_vector = self
                .twist_system_engine_data
                .axis_vectors
                .get(axis.absolute_axis)
                .ok()?;
            let new_axis_vector = self.vantage_motor(vantage)?.transform(axis_vector);
            self.twist_system_engine_data
                .axis_from_vector
                .get(&new_axis_vector)
                .copied()
        }
    }

    fn resolve_twist_concrete(
        &self,
        vantage: Vantage,
        twist: NdEuclidRelativeTwist,
    ) -> Option<Twist> {
        if self.is_identity(&twist.transform) {
            Some(twist.absolute_twist) // optimization
        } else {
            let twist_axis = NdEuclidRelativeAxis {
                absolute_axis: self.twist_axes[twist.absolute_twist],
                transform: twist.transform,
            };
            let new_twist_axis = self.resolve_axis_concrete(vantage, twist_axis)?;

            let twist_transform = self
                .twist_system_engine_data
                .twist_transforms
                .get(twist.absolute_twist)
                .ok()?;
            let new_twist_transform = self.vantage_motor(vantage)?.transform(twist_transform);

            self.twist_system_engine_data
                .twist_from_transform
                .get(&TwistKey::new(new_twist_axis, &new_twist_transform)?)
                .copied()
        }
    }

    fn vantage_group_element_name_concrete(
        &self,
        elem: NdEuclidVantageGroupElement,
    ) -> eyre::Result<String> {
        self.vantage_name_concrete(self.group_element_to_vantage(elem.0))
    }

    fn vantage_name_concrete(&self, vantage: Vantage) -> eyre::Result<String> {
        Ok(hyperpuzzle_core::util::vantage_name(
            self.vantage_names[vantage].iter().map(|&(r1, r2)| {
                (
                    &self.reference_vector_names[r1],
                    &self.reference_vector_names[r2],
                )
            }),
        ))
    }

    fn vantage_group_element_from_name_concrete(
        &self,
        name: &str,
    ) -> Option<NdEuclidVantageGroupElement> {
        Some(NdEuclidVantageGroupElement(self.vantage_to_group_element(
            self.vantage_from_name_concrete(name)?,
        )?))
    }

    fn vantage_from_name_concrete(&self, name: &str) -> Option<Vantage> {
        let reference_vector_mapping = hyperpuzzle_core::util::parse_vantage_name(name)?
            .into_iter()
            .map(|(r1, r2)| {
                Some((
                    &self.reference_vectors[self.reference_vector_names.id_from_name(r1)?],
                    &self.reference_vectors[self.reference_vector_names.id_from_name(r2)?],
                ))
            })
            .collect::<Option<Vec<(&Vector, &Vector)>>>()?;

        // This is O(n) with respect to the number of vantages (very slow!) but
        // that's probably ok. We could probably do something fancy with linear
        // independence but that sounds annoying and complicated.
        self.symmetry
            .elements()
            .filter(|&elem| {
                reference_vector_mapping
                    .iter()
                    .all(|&(r1, r2)| approx_eq(&self.symmetry[elem].transform(r1), r2))
            })
            .map(|elem| self.group_element_to_vantage(elem))
            .exactly_one()
            .ok()
    }

    fn is_identity(&self, elem: &NdEuclidVantageGroupElement) -> bool {
        elem.0 == GroupElementId::IDENTITY
    }

    fn axis_names(&self) -> &NameSpecBiMap<Axis> {
        &self.axis_names
    }

    fn twist_names(&self) -> &NameSpecBiMap<Twist> {
        &self.twist_names
    }
}

#[derive(Debug, Copy, Clone)]
pub struct NdEuclidVantageGroupElement(pub(crate) GroupElementId);
impl VantageGroupElement for NdEuclidVantageGroupElement {
    fn clone_dyn(&self) -> BoxDynVantageGroupElement {
        (*self).into()
    }
}
impl NdEuclidVantageGroupElement {
    pub const IDENTITY: Self = Self(GroupElementId::IDENTITY);
}
