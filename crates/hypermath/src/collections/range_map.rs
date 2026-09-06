use std::{collections::BTreeMap, fmt, range::Range};

use crate::{APPROX, Float, FloatPool};
use itertools::Itertools;

/// Map from float ranges to values.
///
/// Floats are compared using [`Float::total_cmp()`]. NaNs produce errors.
///
/// The entire range of non-NaN floats from `-∞` to `+∞` is covered.
#[derive(Clone)]
pub struct RangeMap<T> {
    /// Map from float to value beneath it.
    ///
    /// This map always contains an entry at [`Float::INFINITY`].
    value_below: BTreeMap<OrdFloat, T>,
    /// Pool for interning floats.
    pool: FloatPool,
}

impl<T: fmt::Debug> fmt::Debug for RangeMap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RangeMap")
            .field("value_below", &self.value_below)
            .finish_non_exhaustive()
    }
}

impl<T: Default + fmt::Debug> Default for RangeMap<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: PartialEq> PartialEq for RangeMap<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value_below == other.value_below
    }
}

impl<T: fmt::Debug> RangeMap<T> {
    /// Constructs a range map with a default value.
    pub fn new(init: T) -> Self {
        Self {
            value_below: BTreeMap::from_iter([(OrdFloat(Float::INFINITY), init)]),
            pool: FloatPool::new(APPROX),
        }
    }

    /// Sets a range to a value.
    pub fn set_range(&mut self, range: impl Into<Range<Float>>, value: T) -> Result<(), NanError>
    where
        T: Clone + PartialEq,
    {
        let range = range.into();
        if range.start.is_nan() || range.end.is_nan() {
            return Err(NanError);
        }
        if range.is_empty() {
            return Ok(());
        }
        let [min, max] = [range.start, range.end].map(|x| OrdFloat(self.pool.intern(x)));
        let value_below = self.get(min).clone();

        // A hypothetical `BTreeMap::remove_range()` would be best here; alas
        let keys_to_remove = self
            .value_below
            .range(min..max)
            .map(|(k, _v)| *k)
            .collect_vec();
        for k in keys_to_remove {
            self.value_below.remove(&k);
        }

        self.value_below.insert(max, value);
        if min > OrdFloat(Float::NEG_INFINITY) {
            self.value_below.insert(min, value_below);
            self.remove_if_not_needed(min);
        }
        self.remove_if_not_needed(max);
        Ok(())
    }

    /// Removes an entry at a key if that entry is redundant.
    fn remove_if_not_needed(&mut self, k: OrdFloat)
    where
        T: PartialEq,
    {
        let mut iter = self.value_below.range(k..).map(|(_k, v)| v);
        if iter.next() == iter.next() {
            self.value_below.remove(&k);
        }
    }

    #[cfg(test)]
    fn redundant_keys(&self) -> impl Iterator<Item = OrdFloat>
    where
        T: PartialEq,
    {
        self.value_below
            .iter()
            .tuple_windows()
            .filter(|((_, v1), (_, v2))| v1 == v2)
            .map(|((k1, _), (_k2, _))| *k1)
    }

    /// Returns the value at a point exactly.
    ///
    /// Note that this is sensitive to floating-point imprecision, and so should
    /// be used with care.
    fn get(&self, k: OrdFloat) -> &T {
        self.value_below.range(k..).next().expect("missing max").1
    }

    /// Returns all values that are properly intersected by a range.
    pub fn get_range(
        &self,
        range: impl Into<Range<Float>>,
    ) -> Result<impl Iterator<Item = &T>, NanError> {
        let range = range.into();
        if range.start.is_nan() || range.end.is_nan() {
            return Err(NanError);
        }
        Ok(self
            .value_below
            .range(OrdFloat(range.start)..)
            .filter(move |_| range.start < range.end)
            .skip_while(move |(k, _)| APPROX.lt_eq(k.0, range.start))
            .take_while_inclusive(move |(k, _)| APPROX.lt(k.0, range.end))
            .map(|(_, v)| v))
    }

    /// Returns an iterator over ranges in the map, from least to greatest.
    ///
    /// These ranges **must** be used with [`Float::total_cmp()`], because they
    /// **always** include NaNs at either end.
    pub fn iter(&self) -> impl Iterator<Item = (Range<Float>, &T)> {
        std::iter::zip(
            std::iter::chain([Float::NEG_INFINITY], self.value_below.keys().map(|f| f.0)),
            &self.value_below,
        )
        .map(|(lower_bound, (upper_bound, value))| (Range::from(lower_bound..upper_bound.0), value))
    }
}

impl<T: Default + PartialEq + Clone + fmt::Debug> FromIterator<(Range<Float>, T)> for RangeMap<T> {
    fn from_iter<I: IntoIterator<Item = (Range<Float>, T)>>(iter: I) -> Self {
        let mut ret = Self::default();
        for (range, value) in iter {
            let _ = ret.set_range(range, value); // ignore NaN
        }
        ret
    }
}

/// Wrapper around [`Float`] that compares using [`Float::total_cmp()`].
#[derive(Debug, Copy, Clone)]
struct OrdFloat(Float);

impl Eq for OrdFloat {}

impl PartialEq for OrdFloat {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Ord for OrdFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for OrdFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NanError;

impl fmt::Display for NanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unexpected NaN")
    }
}

impl std::error::Error for NanError {}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn proptest_range_map_iter(insertion_ranges: Vec<([Float; 2], u8)>) {
            let expected = RangeMap::from_iter(
                insertion_ranges
                    .into_iter()
                    .map(|([lo, hi], value)| (Range::from(lo..hi), value)),
            );
            assert!(expected.redundant_keys().next().is_none());
            let reconstructed =
                RangeMap::from_iter(expected.iter().map(|(range, value)| (range, value.clone())));
            assert!(reconstructed.redundant_keys().next().is_none());
            assert_eq!(expected, reconstructed);
        }

        #[test]
        fn proptest_range_map_query(insertion_ranges: Vec<[u8; 2]>, query_ranges: Vec<[u8; 2]>) {
            test_range_map_with_ranges(insertion_ranges, query_ranges).unwrap();
        }
    }

    fn test_range_map_with_ranges(
        insertion_ranges: Vec<[u8; 2]>,
        query_ranges: Vec<[u8; 2]>,
    ) -> Result<(), NanError> {
        let mut range_map = RangeMap::new(None);
        let mut vec = vec![None; 256];
        for (i, &[a, b]) in insertion_ranges.iter().enumerate() {
            range_map.set_range((a as _)..(b as _), Some(i))?;
            if a < b {
                vec[a as usize..b as usize].fill(Some(i));
            }
        }

        assert!(range_map.redundant_keys().next().is_none());

        for [a, b] in query_ranges {
            let float_range = (a as Float)..(b as Float);
            let expected = vec[if a < b { a as usize..b as usize } else { 0..0 }]
                .iter()
                .copied()
                .dedup();
            itertools::assert_equal(expected.clone(), range_map.get_range(float_range)?.copied());

            // query with ±epsilon
            const DELTA: Float = Float::EPSILON * 128.0;
            let higher_range = (a as Float + DELTA)..(b as Float + DELTA);
            itertools::assert_equal(
                expected.clone(),
                range_map.get_range(higher_range)?.copied(),
            );
            let lower_range = (a as Float - DELTA)..(b as Float - DELTA);
            itertools::assert_equal(expected, range_map.get_range(lower_range)?.copied());
        }

        Ok(())
    }
}
