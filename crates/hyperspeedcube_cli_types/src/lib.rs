//! Types for interacting with Hyperspeedcube's command-line interface.

pub mod catalog_id;
pub mod puzzle_info;
pub mod verification;

/// Compares IDs of objects in a [`Catalog`].
///
/// Currently this uses [`numeric_sort`], a string comparison algorithm that
/// handles numbers in a human-friendly way.
pub fn compare_ids(a: &str, b: &str) -> std::cmp::Ordering {
    numeric_sort::cmp(a, b)
}
