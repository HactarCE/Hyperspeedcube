//! Hyperpuzzlescript interface for the symmetric puzzle engine.

use std::sync::Arc;

use eyre::eyre;
use hypergroup::GenSeq;
use hypermath::pga::Motor;
use hypermath::{
    ApproxEq, ApproxHash, ApproxInternable, Ndim, Point, Precision, TransformByMotor, Vector,
};
use hyperpuzzle_core::CatalogBuilder;
use hyperpuzzle_core::catalog::MenuContent;
use hyperpuzzlescript::util::{expect_end_of_map, pop_map_key};
use hyperpuzzlescript::{
    BUILTIN_SPAN, Builtins, ErrorExt, EvalCtx, FnValue, Map, Runtime, Spanned, Str, Type, Value,
    ValueData, hps_fns,
};

mod orbit_names;
mod puzzle_engine;
mod symmetry;
mod twist_system_engine;

pub use orbit_names::{ElementNames, HpsOrbitNames, HpsOrbitNamesComponent};
pub use symmetry::HpsSymmetry;

use puzzle_engine::SymmetricPuzzleEngine;
use twist_system_engine::SymmetricTwistSystemEngine;

use crate::{NamedPointOrbitSpec, NamedPointSpec, SimpleOrbitSpec};

/// ID for the symmetric puzzle [`Menu`].
pub const MENU_ID: &str = "symmetric";

pub fn register_hps_engines(rt: &mut Runtime) {
    rt.register_puzzle_engine("symmetric", Arc::new(SymmetricPuzzleEngine));
    rt.register_twist_system_engine("symmetric", Arc::new(SymmetricTwistSystemEngine));
}

/// Adds the built-ins.
pub fn define_in(
    builtins: &mut Builtins<'_>,
    catalog: &CatalogBuilder,
) -> hyperpuzzlescript::Result<()> {
    orbit_names::define_in(builtins)?;
    symmetry::define_in(builtins)?;

    builtins.set_fns(hps_fns![
        fn transform(transform: Motor, object: ElementNames) -> HpsOrbitNames {
            object.0.transform_by(&transform)
        }
        fn transform(transform: Motor, object: HpsSymmetry) -> HpsSymmetry {
            transform.transform(&object)
        }

        fn orbit(ctx: EvalCtx, sym: HpsSymmetry, object: Motor) -> Vec<Spanned<Motor>> {
            symmetry::orbit_spanned(ctx, sym, CanonicalMotor::new(object))?
                .into_iter()
                .map(|(CanonicalMotor(m), span)| (m, span))
                .collect()
        }
        fn orbit(ctx: EvalCtx, sym: HpsSymmetry, object: Vector) -> Vec<Spanned<Vector>> {
            symmetry::orbit_spanned(ctx, sym, object)?
        }
        fn orbit(ctx: EvalCtx, sym: HpsSymmetry, object: Point) -> Vec<Spanned<Point>> {
            symmetry::orbit_spanned(ctx, sym, object)?
        }
    ])?;

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

        fn add_colors_override(ctx: EvalCtx, id_pattern: Str, _f: Arc<FnValue>) -> () {
            ctx.warn(format!("adding color override for {id_pattern:?}"));
        }
    ])?;

    Ok(())
}

fn named_point_orbit_from_value(
    ctx: &mut EvalCtx<'_>,
    generators: &[(GenSeq, Motor)],
    value: Value,
    autonames: &mut impl Iterator<Item = String>,
) -> hyperpuzzlescript::Result<NamedPointOrbitSpec> {
    if value.is::<Map>() {
        let mut map = value.as_ref::<Map>()?.clone();
        let init_vector: Vector = pop_map_key(&mut map, value.span, "vector")?;
        let ElementNames(orbit_names) = pop_map_key(&mut map, value.span, "names")?;
        expect_end_of_map(map, value.span)?;

        let raw_orbit_members = hypergroup::orbit_geometric_with_gen_seq(
            hypergroup::ORBIT_LIMIT,
            generators,
            init_vector,
        )
        .at(ctx.caller_span)?;

        let mut vectors = vec![];
        let mut gen_seqs = vec![];
        let mut transforms = vec![];
        for (gen_seq, motor, v) in raw_orbit_members {
            vectors.push(v);
            gen_seqs.push(gen_seq);
            transforms.push(motor);
        }
        let names = orbit_names.to_strings(ctx, &transforms)?;

        let orbit_members = itertools::izip!(vectors, names, gen_seqs)
            .map(|(vector, name, abbr_gen_seq)| NamedPointSpec {
                vector,
                name,
                abbr_gen_seq,
            })
            .collect();
        Ok(NamedPointOrbitSpec { orbit_members })
    } else if value.is::<Vector>() {
        ctx.warn_at(
            value.span,
            "auto-generated named points may not be future-compatible! \
            use the HPS generator tool to make stable named points",
        );

        let init_vector = value.to::<Vector>()?;
        let orbit_members = hypergroup::orbit_geometric_with_gen_seq(
            hypergroup::ORBIT_LIMIT,
            generators,
            init_vector,
        )
        .at(ctx.caller_span)?
        .into_iter()
        .map(|(abbr_gen_seq, _, vector)| NamedPointSpec {
            vector,
            name: autonames.next().expect("exhausted autonames").into(),
            abbr_gen_seq,
        })
        .collect();
        Ok(NamedPointOrbitSpec { orbit_members })
    } else {
        Err(value.type_error(Type::Map | Type::Vec))
    }
}

fn simple_orbit_from_value(list: Vec<Value>) -> hyperpuzzlescript::Result<Vec<SimpleOrbitSpec>> {
    list.into_iter()
        .map(|value| {
            if value.is::<Vector>() {
                let vector = value.to::<Vector>()?;
                let prefix = Str::new();
                Ok(SimpleOrbitSpec { prefix, vector })
            } else if value.is::<Map>() {
                let mut map = value.as_ref::<Map>()?.clone();
                let vector = pop_map_key(&mut map, value.span, "vector")?;
                let prefix = pop_map_key(&mut map, value.span, "prefix")?;
                expect_end_of_map(map, value.span)?;
                Ok(SimpleOrbitSpec { prefix, vector })
            } else {
                Err(value.type_error(Type::Vec | Type::Map))
            }
        })
        .collect()
}

fn new_hps_list() -> Value {
    ValueData::List(Arc::new(vec![])).at(BUILTIN_SPAN)
}
fn new_hps_map() -> Value {
    ValueData::Map(Arc::new(Map::new())).at(BUILTIN_SPAN)
}

#[derive(Debug, Clone)]
struct CanonicalMotor(Motor);
impl CanonicalMotor {
    pub fn new(m: Motor) -> Self {
        Self(m.canonicalize_up_to_180().unwrap_or(m))
    }
}
impl Ndim for CanonicalMotor {
    fn ndim(&self) -> u8 {
        self.0.ndim()
    }
}
impl TransformByMotor for CanonicalMotor {
    fn transform_by(&self, m: &Motor) -> Self {
        Self::new(self.0.transform_by(m))
    }
}
impl ApproxEq for CanonicalMotor {
    fn approx_eq(&self, other: &Self, prec: Precision) -> bool {
        prec.eq(&self.0, &other.0)
    }
}
impl ApproxInternable for CanonicalMotor {
    fn intern_floats<F: FnMut(&mut f64)>(&mut self, f: &mut F) {
        self.0.intern_floats(f);
    }
}
impl ApproxHash for CanonicalMotor {
    fn interned_eq(&self, other: &Self) -> bool {
        self.0.interned_eq(&other.0)
    }

    fn interned_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.interned_hash(state);
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub(super) enum HpsEuclidError {
    #[error("missing coset {0}")]
    MissingCoset(Point),
}
