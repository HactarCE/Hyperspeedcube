use eyre::{Result, bail};
use hypermath::{APPROX, Float, RangeMap, collections::NanError};
use hyperpuzzle_core::PerLayer;
use hypuz_notation::Layer;
use itertools::Itertools;
use smallvec::SmallVec;

/// Cut distances for an axis.
///
/// Distances are measured from the origin and must be sorted from outermost
/// (greatest) to innermost (least).
#[derive(Debug, Default, Clone)]
pub struct CutDistances(Vec<Float>);

impl CutDistances {
    /// Validates cut distances.
    pub fn new(mut cut_distances: Vec<Float>) -> Result<Self> {
        if let Some(bad) = cut_distances.iter().find(|f| !f.is_finite()) {
            bail!("bad cut distance {bad}");
        }
        cut_distances.sort_by(|a, b| a.total_cmp(b).reverse());
        cut_distances.dedup_by(|a, b| APPROX.eq(*a, *b));
        Ok(Self(cut_distances))
    }

    /// Returns the cut distances.
    pub fn distances(&self) -> &[Float] {
        &self.0
    }

    /// Returns a layer map, assuming each cut range is assigned one layer.
    ///
    /// If there are _n_ cuts, then there will be _n+1_ layers.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hypermath::prelude::*;
    /// # use hyperpuzzle_impl_symmetric::CutDistances;
    /// # use hypuz_notation::Layer;
    /// let cuts = CutDistances::new(vec![0.5, 0.0, -0.5]).unwrap();
    ///
    /// let mut expected = RangeMap::new(None);
    /// expected.set_range(0.5..Float::INFINITY, Layer::new(1));
    /// expected.set_range(0.0..0.5, Layer::new(2));
    /// expected.set_range(-0.5..0.0, Layer::new(3));
    /// expected.set_range(Float::NEG_INFINITY..-0.5, Layer::new(4));
    ///
    /// assert_eq!(expected, cuts.implied_layers());
    /// ```
    pub fn implied_layers(&self, is_full_cut: bool) -> Result<RangeMap<Option<Layer>>, NanError> {
        let mut ret = RangeMap::new(None);
        for (i, (&hi, &lo)) in itertools::chain!(
            &[Float::INFINITY],
            &self.0,
            is_full_cut.then_some(&Float::NEG_INFINITY)
        )
        .tuple_windows()
        .enumerate()
        {
            ret.set_range(lo..hi, Layer::from_index(i))?;
        }
        Ok(ret)
    }
}

/// Layer distances along an axis.
///
/// Each layer has a list of ranges. The ranges and layers are not necessarily
/// in order, but they must be non-overlapping.
pub struct LayerDistanceRanges(PerLayer<SmallVec<[[Float; 2]; 1]>>);

pub struct DistanceRange {}

impl LayerDistanceRanges {
    // pub fn get(&self) -> &PerLayer<SmallVec<[[Float;2]; 1]>>
}
