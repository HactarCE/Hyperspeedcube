//! Rhai `add_puzzle()` function.

use std::sync::Arc;

use eyre::{Context, eyre};
use hypermath::Hyperplane;
use hyperpuzzle_core::catalog::{BuildCtx, BuildTask};
use hyperpuzzle_impl_nd_euclid::builder::*;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use rhai::Array;

use super::axis_system::RhaiAxisSystem;
use super::*;
use crate::package::types::elements::{LockAs, RhaiAxis};
use crate::package::types::name_strategy::RhaiNameStrategy;
use crate::package::types::symmetry::RhaiSymmetry;

pub fn init_engine(engine: &mut Engine) {
    engine.register_type_with_name::<RhaiPuzzle>("puzzle");
}

pub fn register(module: &mut Module, catalog: &Catalog, eval_tx: &RhaiEvalRequestTx) {
    let cat = catalog.clone();
    let tx = eval_tx.clone();
    new_fn("add_puzzle").set_into_module(module, move |ctx: Ctx<'_>, map: Map| -> Result {
        let spec = puzzle_spec_from_rhai_map(&ctx, cat.clone(), tx.clone(), map)?;
        cat.add_puzzle(Arc::new(spec)).eyrefmt()
    });

    let cat = catalog.clone();
    let tx = eval_tx.clone();
    new_fn("add_puzzle_generator").set_into_module(
        module,
        move |ctx: Ctx<'_>, gen_map: Map| -> Result {
            let cat2 = cat.clone();
            let tx = tx.clone();
            let generator = puzzle_spec_generator_from_rhai_map(
                &ctx,
                tx.clone(),
                gen_map,
                move |ctx, build_ctx, map| {
                    build_ctx.progress.lock().task = BuildTask::BuildingTwists;
                    puzzle_spec_from_rhai_map(&ctx, cat2.clone(), tx.clone(), map)
                },
            )?;
            cat.add_puzzle_generator(Arc::new(generator)).eyrefmt()
        },
    );

    FuncRegistration::new_getter("ndim")
        .set_into_module(module, |puzzle: &mut RhaiPuzzle| -> Result<i64> {
            Ok(puzzle.lock()?.ndim().into())
        });

    FuncRegistration::new_getter("axes").set_into_module(
        module,
        |puzzle: &mut RhaiPuzzle| -> RhaiAxisSystem {
            RhaiAxisSystem(RhaiTwistSystem::Puzzle(puzzle.clone()))
        },
    );

    // FuncRegistration::new("carve").set_into_module(
    //     module,
    //     |puzzle: &mut RhaiPuzzle, plane: Hyperplane| -> Result<()> {

    //     },
    // );
    FuncRegistration::new("carve").set_into_module(
        module,
        |ctx: Ctx<'_>,
         puzzle: &mut RhaiPuzzle,
         plane: Hyperplane,
         color_names: Dynamic|
         -> Result<()> {
            let color_names = from_rhai::<RhaiNameStrategy>(&ctx, color_names)?;
            puzzle.cut(
                &ctx,
                plane,
                color_names,
                cut::CutMode::Carve,
                None,
                cut::StickerMode::NewColor,
            )
        },
    );

    FuncRegistration::new("slice_layers").set_into_module(
        module,
        |ctx: Ctx<'_>, puzzle: &mut RhaiPuzzle, axis: RhaiAxis, depths: Array| -> Result<()> {
            let depths: Vec<f32> = from_rhai_array(&ctx, depths)?;
            let mut puz = puzzle.lock()?;
            let twists = puz.twists.lock();
            let axes = match RhaiSymmetry::get(&ctx) {
                Some(sym) => {
                    let vector = axis.vector()?;
                    sym.orbit(vector)
                        .iter()
                        .filter_map(|(_, _, v)| twists.axes.vector_to_id(v))
                        .collect()
                }
                None => {
                    vec![axis.id]
                }
            };
            drop(twists);
            for axis in axes {
                let twists = puz.twists.lock();
                let Ok(axis_info) = twists.axes.get(axis) else {
                    break;
                };
                let vector = axis_info.vector().clone();
                drop(twists);

                let mut shape = puz.shape.lock();
                for &depth in &depths {
                    let plane = Hyperplane::new(&vector, depth as _)
                        .ok_or("invalid hyperplane (axis vector may be zero)")?;
                    shape.slice(None, plane, None).eyrefmt()?;
                }
                drop(shape);

                let axis_layers = &mut puz.axis_layers().eyrefmt()?[axis].0;
                for layer in depths.windows(2) {
                    axis_layers
                        .push(AxisLayerBuilder {
                            top: layer[0] as _,
                            bottom: layer[1] as _,
                        })
                        .map_err(|e| e.to_string())?;
                }
            }
            Ok(())
        },
    );
}

pub fn puzzle_spec_generator_from_rhai_map(
    ctx: &Ctx<'_>,
    eval_tx: RhaiEvalRequestTx,
    data: Map,
    generate_from_spec: impl 'static + Send + Sync + Fn(&Ctx<'_>, BuildCtx, Map) -> Result<PuzzleSpec>,
) -> Result<PuzzleSpecGenerator> {
    todo!("puzzle generators")
}

/// Constructs a puzzle spec from a Rhai specification.
pub fn puzzle_spec_from_rhai_map(
    ctx: &Ctx<'_>,
    catalog: Catalog,
    eval_tx: RhaiEvalRequestTx,
    data: Map,
) -> Result<PuzzleSpec> {
    let_from_map!(ctx, data, {
        let id: String;
        let name: Option<String>;
        let aliases: Option<Vec<String>>;
        let version: Option<Version>;
        let tags: Option<Map>; // TODO
        let colors: Option<String>; // TODO: string or array of strings (gen ID + params)
        let twists: Option<String>; // TODO: string or array of strings (gen ID + params)
        let ndim: u8;
        let build: FnPtr;
        let remove_internals: Option<bool>;
        let scramble: Option<i64>;
    });

    let mut tags = TagSet::new(); // TODO

    if let Some(color_system_id) = colors.clone() {
        tags.insert_named("colors/system", TagValue::Str(color_system_id))
            .map_err(|e| e.to_string())?;
    }

    if let Some(twist_system_id) = twists.clone() {
        tags.insert_named("twists/system", TagValue::Str(twist_system_id))
            .map_err(|e| e.to_string())?;
    }

    // TODO: inherit parent tags

    // TODO: copy this to twist system and color system
    let name = match name {
        Some(s) => s,
        None => {
            warn(ctx, format!("missing `name` for puzzle `{id}`"));
            id.clone()
        }
    };

    let meta = Arc::new(PuzzleListMetadata {
        id: id.clone(),
        version: version.unwrap_or(Version::PLACEHOLDER),
        name,
        aliases: aliases.unwrap_or_default(),
        tags,
    });
    let meta_clone = Arc::clone(&meta);

    // Part of the puzzle-building process that happens on non-Rhai thread
    let create_puzzle_builder = move |build_ctx: &BuildCtx| -> eyre::Result<RhaiPuzzle> {
        let builder = RhaiPuzzle(Arc::new(Mutex::new(PuzzleBuilder::new(
            Arc::clone(&meta),
            ndim,
        )?)));

        if let Some(color_system_id) = &colors {
            build_ctx.progress.lock().task = BuildTask::BuildingColors;
            let colors = catalog
                .build_blocking(color_system_id)
                .map_err(|e| eyre!("{e}"))?;
            builder.lock()?.shape.lock().colors = ColorSystemBuilder::unbuild(&colors)?;
        }

        if let Some(twist_system_id) = &twists {
            build_ctx.progress.lock().task = BuildTask::BuildingTwists;
            let twists = catalog
                .build_blocking(twist_system_id)
                .map_err(|e| eyre!("{e}"))?;
            *builder.lock()?.twists.lock() = TwistSystemBuilder::unbuild(&twists)?;
        }

        build_ctx.progress.lock().task = BuildTask::BuildingPuzzle;

        if let Some(remove_internals) = remove_internals {
            builder.lock()?.shape.lock().remove_internals = remove_internals;
        }
        if let Some(full_scramble_length) = scramble {
            builder.lock()?.full_scramble_length = full_scramble_length
                .try_into()
                .wrap_err("bad scramble length")?;
        };

        Ok(builder)
    };

    // Part of the puzzle-building process that happens on Rhai thread
    let build_from_puzzle_builder = crate::util::rhai_eval_fn(
        ctx,
        eval_tx,
        &build.clone(),
        move |ctx,
              (build_ctx, builder): (BuildCtx, RhaiPuzzle)|
              -> eyre::Result<Redirectable<Arc<Puzzle>>> {
            let mut this = Dynamic::from(builder.clone());

            let () = RhaiState::with_ndim(&ctx, ndim, |ctx| {
                Ok(from_rhai(ctx, build.call_raw(ctx, Some(&mut this), [])?)?)
            })?;

            // Assign default piece type to remaining pieces.
            builder.lock()?.shape.lock().mark_untyped_pieces()?;

            builder
                .lock()?
                .build(Some(&build_ctx), warnf(&ctx))
                .map(|ok| Redirectable::Direct(ok))
        },
    );

    Ok(PuzzleSpec {
        meta: meta_clone,
        build: Box::new(move |build_ctx| {
            let builder = create_puzzle_builder(&build_ctx)?;
            build_from_puzzle_builder((build_ctx, builder))?
        }),
    })
}

#[derive(Debug, Clone)]
pub struct RhaiPuzzle(pub Arc<Mutex<PuzzleBuilder>>);
impl RhaiPuzzle {
    // TODO: THIS IS A REALLY NASTY HACK
    pub fn lock(&self) -> Result<MappedMutexGuard<'_, PuzzleBuilder>> {
        <Self as LockAs<PuzzleBuilder>>::lock(self)
    }
}
impl LockAs<PuzzleBuilder> for RhaiPuzzle {
    fn lock(&self) -> Result<MappedMutexGuard<'_, PuzzleBuilder>> {
        MutexGuard::try_map(self.0.lock(), |contents| Some(contents))
            .map_err(|_| "no puzzle".into())
    }
}
impl LockAs<TwistSystemBuilder> for RhaiPuzzle {
    fn lock(&self) -> Result<MappedMutexGuard<'_, TwistSystemBuilder>> {
        unimplemented!()
    }
}
impl LockAs<AxisSystemBuilder> for RhaiPuzzle {
    fn lock(&self) -> Result<MappedMutexGuard<'_, AxisSystemBuilder>> {
        unimplemented!()
    }
}
