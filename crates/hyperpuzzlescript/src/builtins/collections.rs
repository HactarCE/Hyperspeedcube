//! Operators and functions for operating on lists and maps.

use std::sync::Arc;

use itertools::Itertools;

use crate::{Builtins, FnValue, List, Map, Result, Str, Value, ValueData};

/// Adds the built-in operators and functions.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_fns(hps_fns![
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
        // Functional programming
        (
            "map",
            |ctx, list: List, (f, f_span): Arc<FnValue>| -> List {
                list.into_iter()
                    .map(|v| f.call(f_span, ctx, vec![v], Map::new()))
                    .try_collect()?
            }
        ),
        (
            "map",
            |ctx, map: Arc<Map>, (f, f_span): Arc<FnValue>| -> Map {
                map.iter()
                    .map(|(k, v)| {
                        let k_str = ValueData::Str(k.as_str().into()).at(ctx.caller_span);
                        let new_v = f.call(f_span, ctx, vec![k_str, v.clone()], Map::new())?;
                        Ok((k.clone(), new_v))
                    })
                    .try_collect()?
            }
        ),
        (
            "reduce",
            |ctx, list: List, (f, f_span): Arc<FnValue>| -> Value {
                list.into_iter()
                    .map(Ok)
                    .reduce(|a, b| f.call(f_span, ctx, vec![a?, b?], Map::new()))
                    .unwrap_or(Ok(Value::NULL))?
            }
        ),
        (
            "filter",
            |ctx, list: List, (f, f_span): Arc<FnValue>| -> List {
                let mut ret = vec![];
                for value in list {
                    if f.call(f_span, ctx, vec![value.clone()], Map::new())?.to()? {
                        ret.push(value);
                    }
                }
                ret
            }
        ),
        // Other operations
        ("rev", |_, l: List| -> List {
            let mut l = l;
            l.reverse();
            l
        }),
    ])
}
