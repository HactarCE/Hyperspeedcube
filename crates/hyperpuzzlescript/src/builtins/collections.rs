use std::sync::Arc;

use itertools::Itertools;

use crate::{Result, Scope, ValueData};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        // Operators
        hps_fn!("++", |l1: List, l2: List| -> List {
            itertools::chain(&**l1, &**l2).cloned().collect_vec()
        }),
        hps_fn!("concat", |lists: List(List)| -> List {
            lists
                .into_iter()
                .flat_map(Arc::unwrap_or_clone)
                .collect_vec()
        }),
        // Getters (TODO: nullable return type for all of these)
        hps_fn!("get", |l: List, i: Int| -> Any {
            i.try_into()
                .ok()
                .and_then(|i: usize| Some(l.get(i)?.data.clone()))
                .unwrap_or(ValueData::Null)
        }),
        hps_fn!("get", |m: Map, k: Str| -> Any {
            m.get(k.as_str())
                .map(|v| v.data.clone())
                .unwrap_or(ValueData::Null)
        }),
        hps_fn!("get", |s: Str, i: Int| -> Any {
            i.try_into()
                .ok()
                .and_then(|i| s.chars().nth(i))
                .map(|c| ValueData::Str(c.into()))
                .unwrap_or(ValueData::Null)
        }),
        hps_fn!("get_cyclic", |l: List, i: Int| -> Any {
            i.rem_euclid(l.len() as i64)
                .try_into()
                .ok()
                .and_then(|i: usize| Some(l.get(i)?.data.clone()))
                .unwrap_or(ValueData::Null)
        }),
        // Length getters
        hps_fn!("len", |l: List| -> Bool { l.len() }),
        hps_fn!("len", |m: Map| -> Bool { m.len() }),
        hps_fn!("len", |s: Str| -> Bool { s.len() }),
        hps_fn!("is_empty", |l: List| -> Bool { l.is_empty() }),
        hps_fn!("is_empty", |m: Map| -> Bool { m.is_empty() }),
        hps_fn!("is_empty", |s: Str| -> Bool { s.is_empty() }),
    ])
}
