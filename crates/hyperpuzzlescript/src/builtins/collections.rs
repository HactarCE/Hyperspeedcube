//! Operators and functions for operating on lists and maps.

use std::sync::Arc;

use crate::{List, Map, Result, Scope, Str, ValueData};

/// Adds the built-in operators and functions to the scope.
pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions(hps_fns![
        // Operators
        ("++", |_, l1: List, l2: List| -> List {
            itertools::chain(l1, l2).collect()
        }),
        ("concat", |_, lists: Vec<List>| -> List {
            lists.into_iter().flatten().collect()
        }),
        // Getters
        ("get", |_, l: Arc<List>, i: i64| -> Option<ValueData> {
            i.try_into()
                .ok()
                .and_then(|i: usize| Some(l.get(i)?.data.clone()))
        }),
        ("get", |_, m: Arc<Map>, k: Str| -> Option<ValueData> {
            m.get(k.as_str()).map(|v| v.data.clone())
        }),
        ("get", |_, s: Str, i: i64| -> Option<char> {
            i.try_into().ok().and_then(|i| s.chars().nth(i))
        }),
        (
            "get_cyclic",
            |_, l: Arc<List>, i: i64| -> Option<ValueData> {
                i.rem_euclid(l.len() as i64)
                    .try_into()
                    .ok()
                    .and_then(|i: usize| Some(l.get(i)?.data.clone()))
            }
        ),
        // Length getters
        ("len", |_, l: Arc<List>| -> usize { l.len() }),
        ("len", |_, m: Arc<Map>| -> usize { m.len() }),
        ("len", |_, s: Str| -> usize { s.len() }),
        ("is_empty", |_, l: Arc<List>| -> bool { l.is_empty() }),
        ("is_empty", |_, m: Arc<Map>| -> bool { m.is_empty() }),
        ("is_empty", |_, s: Str| -> bool { s.is_empty() }),
        // Other operations
        ("rev", |_, l: List| -> List {
            let mut l = l;
            l.reverse();
            l
        }),
    ])
}
