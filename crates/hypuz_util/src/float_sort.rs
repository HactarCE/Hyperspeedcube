//! Extension traits for comparing iterator items by floating-point values.

use std::cmp::Ordering;

use itertools::Itertools;

/// Extension trait generalizing [`Iterator::max()`] and [`Iterator::min()`] to
/// [`f32`] and [`f64`] using float total comparison functions.
pub trait FloatMinMaxIteratorExt: Iterator + Sized
where
    Self::Item: TotalCmp,
{
    /// Same as [`Iterator::max()`], but using
    /// [`f32::total_cmp()`]/[`f64::total_cmp()`] to compare items.
    fn max_float(self) -> Option<Self::Item> {
        self.max_by(|a, b| a.hypuz_util_total_cmp(b))
    }

    /// Same as [`Iterator::min()`], but using
    /// [`f32::total_cmp()`]/[`f64::total_cmp()`] to compare items.
    fn min_float(self) -> Option<Self::Item> {
        self.min_by(|a, b| a.hypuz_util_total_cmp(b))
    }

    /// Same as [`Itertools::minmax()`], but using
    /// [`f32::total_cmp()`]/[`f64::total_cmp()`] to compare items.
    fn minmax_float(self) -> itertools::MinMaxResult<Self::Item> {
        self.minmax_by(|a, b| a.hypuz_util_total_cmp(b))
    }

    /// Same as [`Iterator::is_sorted()`], but using
    /// [`f32::total_cmp()`]/[`f64::total_cmp()`] to compare items.
    fn is_float_sorted(self) -> bool {
        self.is_sorted_by(|a, b| a.hypuz_util_total_cmp(b).is_le())
    }
}

impl<I: Iterator> FloatMinMaxIteratorExt for I where I::Item: TotalCmp {}

/// Extension trait generalizing [`Iterator::max_by_key()`] and
/// [`Iterator::min_by_key()`] to [`f32`] and [`f64`] using float total
/// comparison functions.
pub trait FloatMinMaxByIteratorExt: Iterator + Sized {
    /// Same as [`Iterator::max_by_key()`], but using
    /// [`f32::total_cmp()`]/[`f64::total_cmp()`] to compare the keys.
    fn max_by_float_key<B, F>(self, mut f: F) -> Option<Self::Item>
    where
        B: TotalCmp,
        F: FnMut(&Self::Item) -> B,
    {
        self.max_by(|a, b| f(a).hypuz_util_total_cmp(&f(b)))
    }

    /// Same as [`Iterator::min_by_key()`], but using
    /// [`f32::total_cmp()`]/[`f64::total_cmp()`] to compare the keys.
    fn min_by_float_key<B, F>(self, mut f: F) -> Option<Self::Item>
    where
        B: TotalCmp,
        F: FnMut(&Self::Item) -> B,
    {
        self.min_by(|a, b| f(a).hypuz_util_total_cmp(&f(b)))
    }

    /// Same as [`Iterator::is_sorted_by_key()`], but using
    /// [`f32::total_cmp()`]/[`f64::total_cmp()`] to compare items.
    fn is_float_sorted<B, F>(self, mut f: F) -> bool
    where
        B: TotalCmp,
        F: FnMut(&Self::Item) -> B,
    {
        self.is_sorted_by(|a, b| f(a).hypuz_util_total_cmp(&f(b)).is_le())
    }
}

impl<I: Iterator> FloatMinMaxByIteratorExt for I {}

/// Trait unifying [`f32::total_cmp()`] and [`f64::total_cmp()`].
pub trait TotalCmp {
    /// See [`f32::total_cmp()`] and [`f64::total_cmp()`].
    fn hypuz_util_total_cmp(&self, other: &Self) -> Ordering;
}

impl TotalCmp for f32 {
    fn hypuz_util_total_cmp(&self, other: &Self) -> Ordering {
        f32::total_cmp(self, other)
    }
}

impl TotalCmp for f64 {
    fn hypuz_util_total_cmp(&self, other: &Self) -> Ordering {
        f64::total_cmp(self, other)
    }
}
