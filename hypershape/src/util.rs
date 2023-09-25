use itertools::Itertools;
use std::fmt;

/// Returns a human-friendly string listing comma-separated values.
pub fn display_list<T: fmt::Display>(list: impl IntoIterator<Item = T>) -> String {
    format!("[{}]", list.into_iter().join(", "))
}
