//! Hyperpuzzlescript interface for the symmetric puzzle engine.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use eyre::{Context, eyre};
use hypergroup::{AbbrGenSeq, GenSeq};
use hypermath::Vector;
use hypermath::pga::Motor;
use hyperpuzzle_core::CatalogBuilder;
use hyperpuzzle_core::catalog::{Menu, MenuContent};
use hyperpuzzle_impl_nd_euclid::hps::{ElementNames, HpsOrbitNames};
use hyperpuzzlescript::util::{expect_end_of_map, pop_map_key};
use hyperpuzzlescript::{
    BUILTIN_SPAN, Builtins, ErrorExt, EvalCtx, FnValue, HpsEngine, ListOf, Map, Runtime, Spanned,
    Str, Value, ValueData, hps_fns,
};
use parking_lot::Mutex;

mod puzzle_engine;
mod twist_system_engine;

use puzzle_engine::SymmetricPuzzleEngine;
use twist_system_engine::SymmetricTwistSystemEngine;

use crate::{SimpleOrbitMemberSpec, SimpleOrbitSpec};

/// ID for the symmetric puzzle [`Menu`].
pub const MENU_ID: &'static str = "symmetric";

pub fn register_hps_engines(rt: &mut Runtime) {
    rt.register_puzzle_engine("symmetric", Arc::new(SymmetricPuzzleEngine));
    rt.register_twist_system_engine("symmetric", Arc::new(SymmetricTwistSystemEngine));
}

/// Adds the built-ins.
pub fn define_in(
    builtins: &mut Builtins<'_>,
    catalog: &CatalogBuilder,
) -> hyperpuzzlescript::Result<()> {
    let cat = catalog.clone();

    cat.add_menu(MENU_ID, "Symmetric Puzzles".to_string())
        .at(BUILTIN_SPAN)?;

    builtins.set_fns(hps_fns![
        #[kwargs(
            path: Str,
            priority: Option<i64>,
            default: bool = false,
            next_column: Option<Str>,
            next_inline: Option<Str>,
            section: Option<bool>,
            (id, id_span): Option<Str>,
        )]
        fn add_menu_entry(ctx: EvalCtx) -> () {
            let next = match (next_column, next_inline, section.unwrap_or(false), id) {
                (Some(title), None, false, None) => MenuContent::Column {
                    title: title.into(),
                },
                (None, Some(label), false, None) => MenuContent::Inline {
                    label: label.into(),
                },
                (None, None, true, None) => MenuContent::Section,
                (None, None, false, Some(id)) => MenuContent::End {
                    id: id
                        .parse()
                        .map_err(|e| eyre!("error parsing puzzle ID: {e}"))
                        .at(id_span)?,
                },
                _ => return Err(
                    "`next_column`, `next_inline`, `section`, and `id` are all mutually exclusive"
                        .to_string()
                        .at(ctx.caller_span),
                ),
            };

            cat.add_menu_node(MENU_ID, path.into(), next, priority.unwrap_or(0), default)
                .at(ctx.caller_span)?;
        }

        fn add_colors_override(ctx: EvalCtx, id_pattern: Str, f: Arc<FnValue>) -> () {
            ctx.warn(format!("adding color override for {id_pattern:?}"));
        }
    ])?;

    Ok(())
}

fn named_orbit_from_value(
    ctx: &mut EvalCtx<'_>,
    generators: &[(GenSeq, Motor)],
    value: Value,
) -> hyperpuzzlescript::Result<SimpleOrbitSpec> {
    let mut map = value.as_ref::<Map>()?.clone();
    let init_vector: Vector = pop_map_key(&mut map, value.span, "vector")?;
    let ElementNames(orbit_names) = pop_map_key(&mut map, value.span, "names")?;
    expect_end_of_map(map, value.span);

    let mut vectors = vec![];
    let mut gen_seqs = vec![];
    let mut transforms = vec![];
    for (gen_seq, motor, v) in hypergroup::orbit_geometric_with_gen_seq(generators, init_vector) {
        vectors.push(v);
        gen_seqs.push(gen_seq);
        transforms.push(motor);
    }
    let names = orbit_names.to_strings(ctx, &transforms)?;

    let orbit_members = itertools::izip!(vectors, names, gen_seqs)
        .map(|(vector, name, abbr_gen_seq)| SimpleOrbitMemberSpec {
            vector,
            name,
            abbr_gen_seq,
        })
        .collect();
    Ok(SimpleOrbitSpec { orbit_members })
}

fn new_hps_list() -> Value {
    ValueData::List(Arc::new(vec![])).at(BUILTIN_SPAN)
}
fn new_hps_map() -> Value {
    ValueData::Map(Arc::new(Map::new())).at(BUILTIN_SPAN)
}
