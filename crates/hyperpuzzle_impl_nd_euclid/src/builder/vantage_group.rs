use std::sync::Arc;

use eyre::{OptionExt, Result, bail};
use hypermath::{ApproxHashMap, Vector};
use hyperpuzzle_core::{
    Axis, BoxDynVantageGroup, NameSpecBiMap, NameSpecBiMapBuilder, PerTwist, PerVantage, Twist,
};
use hypershape::{Group, IsometryGroup};
use itertools::Itertools;
use smallvec::SmallVec;

use crate::{
    NdEuclidTwistSystemEngineData, NdEuclidVantageGroup, PerReferenceVector, ReferenceVector,
};

/// Vantage group during puzzle construction.
#[derive(Debug, Default)]
pub struct VantageGroupBuilder {
    /// Symmetry group.
    pub symmetry: IsometryGroup,
    /// Reference vectors, which are used to serialize/deserialize vantages.
    pub reference_vectors: PerReferenceVector<Vector>,
    /// Names for reference vectors.
    pub reference_vector_names: NameSpecBiMapBuilder<ReferenceVector>,
    /// List of reference vectors to prefer when serializing vantages.
    ///
    /// This list MUST contain enough reference vectors to uniquely identify
    /// every vantage. It SHOULD contain exactly enough.
    pub preferred_reference_vectors: Vec<ReferenceVector>,
}
impl VantageGroupBuilder {
    /// Validates and constructs a vantage group.
    pub fn build(
        &self,
        axis_names: Arc<NameSpecBiMap<Axis>>,
        twist_names: Arc<NameSpecBiMap<Twist>>,
        twist_axes: Arc<PerTwist<Axis>>,
        twist_system_engine_data: NdEuclidTwistSystemEngineData,
    ) -> Result<NdEuclidVantageGroup> {
        let reference_vectors_by_vector = ApproxHashMap::<Vector, ReferenceVector>::from_iter(
            self.reference_vectors
                .iter()
                .map(|(reference_vector, vector)| (vector.clone(), reference_vector)),
        );

        let reference_vector_names = self
            .reference_vector_names
            .clone()
            .build(self.reference_vectors.len())
            .ok_or_eyre("missing reference vector names")?;

        // This algorithm could be modified to support missing references. This
        // isn't necessary for N-dimensional Euclidean, but is for puzzles in
        // infinite spaces.
        let vantage_names: PerVantage<_> = self
            .symmetry
            .elements()
            .filter(|&e| !self.symmetry[e].is_reflection())
            .map(|e| {
                self.preferred_reference_vectors
                    .iter()
                    .map(|&ref_vec| {
                        let transformed_ref_vec = *reference_vectors_by_vector
                            .get(&self.symmetry[e].transform(&self.reference_vectors[ref_vec]))
                            .ok_or_eyre("reference frame is not valid in some vantages")?;
                        Ok((ref_vec, transformed_ref_vec))
                    })
                    .collect::<Result<SmallVec<_>>>()
            })
            .try_collect()?;
        if let Some(pairs) = vantage_names.iter_values().duplicates().next() {
            let name = hyperpuzzle_core::util::vantage_name(
                pairs
                    .iter()
                    .map(|&(a, b)| (&reference_vector_names[a], &reference_vector_names[b])),
            );
            bail!("duplicate vantage name {name:?}");
        }

        Ok(NdEuclidVantageGroup {
            symmetry: self.symmetry.clone(),

            reference_vectors: self.reference_vectors.clone(),
            reference_vector_names,
            preferred_reference_vectors: self.preferred_reference_vectors.clone(),

            vantage_names,

            axis_names,
            twist_names,

            twist_axes,
            twist_system_engine_data,
        })
    }

    /// "Unbuilds" a vantage group.
    pub fn unbuild(vantage_group: &BoxDynVantageGroup) -> Result<Self> {
        let NdEuclidVantageGroup {
            symmetry,
            reference_vectors,
            reference_vector_names,
            preferred_reference_vectors,
            ..
        } = vantage_group
            .downcast_ref()
            .ok_or_eyre("expected NdEuclid vantage group")?;

        Ok(Self {
            symmetry: symmetry.clone(),
            reference_vectors: reference_vectors.clone(),
            reference_vector_names: reference_vector_names.clone().into(),
            preferred_reference_vectors: preferred_reference_vectors.clone(),
        })
    }
}
