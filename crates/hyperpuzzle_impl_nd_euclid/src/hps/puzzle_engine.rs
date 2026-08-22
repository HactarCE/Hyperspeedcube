use eyre::eyre;
use hyperpuzzle_core::prelude::*;
use hyperpuzzlescript::engine::{HpsEngineError, HpsGenerator};
use hyperpuzzlescript::*;

pub struct NdEuclidPuzzleEngine;

impl HpsEngine for NdEuclidPuzzleEngine {
    fn add_catalog_entries(
        &self,
        catalog: &CatalogBuilder,
        eval_tx: &EvalRequestTx,
        ctx: &mut EvalCtx<'_>,
        hps_gen: HpsGenerator,
    ) -> Result<(), HpsEngineError> {
        Err(eyre!("todo: nd euclid puzzle engine").into())
    }
}

// impl HpsEngine for NdEuclidPuzzleEngine {
//     fn add_catalog_entries(
//         &self,
//         catalog: &CatalogBuilder,
//         eval_tx: &EvalRequestTx,
//         ctx: &mut EvalCtx<'_>,
//         hps_gen: HpsGenerator,
//     ) -> hyperpuzzlescript::Result<()> {
//         let caller_span = ctx.caller_span;
//         let id = &hps_gen.id;
//         let kwargs = &mut hps_gen.kwargs;

//         pop_kwarg!(kwargs, name: String = {
//             ctx.warn(format!("missing `name` for puzzle generator `{id}`"));
//             id.to_string()
//         });
//         pop_kwarg!(kwargs, aliases: Vec<String> = vec![]);
//         pop_kwarg!(kwargs, (version, version_span): Option<String>);
//         pop_kwarg!(kwargs, tags: Option<Arc<Map>>);
//         pop_kwarg!(kwargs, colors: Option<String>);
//         pop_kwarg!(kwargs, twists: Option<String>);

//         let version = version.map(|s|
// s.parse().at(version_span)).transpose()?;

//         catalog.add::<NdEuclidPuzzle>(hps_gen.make_generator(
//             eval_tx,
//             move |build_ctx, runtime, kwargs| {
//                 pop_kwarg!(kwargs, ndim: u8);
//                 pop_kwarg!(kwargs, remove_internals: bool);
//                 pop_kwarg!(kwargs, scramble: usize);

//                 let list_entry = Arc::new(PuzzleListEntry {
//                     id: build_ctx.id().clone(),
//                     version,
//                     name,
//                     aliases,
//                     tags: TagSet::todo(),
//                 });
//                 Ok(Arc::new(NdEuclidPuzzle {
//                     list_entry: Arc::clone(&list_entry),
//                     build: Arc::new(move|build_ctx, runtime| {
//                         let logger = &build_ctx.logger();
//                         let builder = Arc::new(Mutex::new(PuzzleBuilder::new(
//                             Arc::clone(&list_entry),
//                             ndim,
//                         ).at(caller_span)?));
//                         let id = &list_entry.id;

//                         // Build color system.
//                         if let Some(colors_id) = &colors {
//                             builder.lock().shape.lock().colors =
// ColorSystemBuilder(
// MaybeAdHoc::Fixed(build_ctx.build_str_blocking(colors_id).at(caller_span )?),
//                             );
//                         } else {
//                             logger.warn(format!("using ad-hoc color system
// for puzzle {id:?}"));                         }

//                         // Build twist system.
//                         if let Some(twists_id) = &twists {
//                             builder.lock().twists =
// TwistSystemBuilder(MaybeAdHoc::Fixed(
// build_ctx.build_str_blocking(twists_id).at(caller_span)?,
// ));                         } else {
//                             logger.warn(format!("using ad-hoc color system
// for puzzle {id:?}"));                         }

//                         if let Some(remove_internals) = remove_internals {
//                             builder.lock().shape.lock().remove_internals =
// remove_internals;                         }
//                         if let Some(full_scramble_length) = scramble {
//                             builder.lock().full_scramble_length =
// full_scramble_length;                         };

//                         let mut scope = Scope::default();
//                         scope.special.ndim = Some(ndim);
//                         scope.special.puz =
//
// Arc::new(Mutex::new(HpsPuzzle(builder.clone()).at(BUILTIN_SPAN)));
//                         scope.special.shape = Arc::new(Mutex::new(
//
// HpsShape(builder.lock().shape.clone()).at(BUILTIN_SPAN),
// ));                         scope.special.twists = Arc::new(Mutex::new(
//
// HpsTwistSystem(builder.lock().twists.clone()).at(BUILTIN_SPAN),
// ));                         scope.special.axes = Arc::new(Mutex::new(
//
// HpsAxisSystem(builder.lock().twists.clone()).at(BUILTIN_SPAN),
// ));                         scope.special.id = Some(id.to_string().into());
//                         let scope = Arc::new(scope);
//                         let mut exports = None;
//                         let mut eval_ctx = EvalCtx::new(&scope, runtime,
// caller_span, &mut exports);

//                         let build_fn = Arc::clone(&build);

//                         build_fn
//                             .call(build_span, &mut eval_ctx, vec![],
// Map::new())                             .map_err(|e|
// eval_ctx.runtime.report_and_convert_to_eyre(e))
// .wrap_err("error building puzzle")?;

//                         let b = builder.lock();

//                         // Assign default piece type to remaining pieces.
//                         b.shape.lock().mark_untyped_pieces()?;

//                         b.build(Some(&build_ctx), &mut
// eval_ctx.warnf()).at(caller_span)                     }),
//                 }))
//             },
//         ))?;

//         let generator_id = CatalogId::new(hps_gen.id, vec![], None);
//         catalog.add::<PuzzleListEntry>(hps_gen.make_generator_with_empty(
//             eval_tx,
//             move |build_ctx| {
//                 let kwargs =
//                 Ok(Arc::new(PuzzleListEntry {
//                     id: generator_id.clone(),
//                     version,
//                     name,
//                     aliases,
//                     tags: TagSet::todo(),
//                 }))
//             },
//             |build_ctx, _kwargs| {
//                 build_ctx
//                     .build_blocking::<NdEuclidPuzzle>(build_ctx.id())
//                     .map(|nd_euclid_puzzle|
// Arc::clone(&nd_euclid_puzzle.list_entry))             },
//         ))?;

//         let tx = eval_tx.clone();
//         catalog.add::<Puzzle>(hps_gen.make_generator(eval_tx, |build_ctx| {
//             build_ctx
//                 .build_blocking::<NdEuclidPuzzle>(build_ctx.id())
//                 .and_then(|nd_euclid_puzzle| {
//                     tx.eval_blocking(move |runtime| {
//                         (nd_euclid_puzzle.build)(build_ctx, runtime)
//                             .map_err(|e|
// runtime.report_and_convert_to_eyre(e))                     })
//                 })
//         }))?;

//         Ok(())
//     }
// }

// struct NdEuclidPuzzle {
//     list_entry: Arc<PuzzleListEntry>,
//     build: Arc<dyn Send + Sync + Fn(BuildCtx, &mut Runtime) ->
// Result<Arc<Puzzle>>>, }

// impl CatalogObject for NdEuclidPuzzle {
//     fn catalog_type_name() -> &'static str {
//         "ndeuclid puzzle"
//     }

//     fn id(&self) -> &CatalogId {
//         self.list_entry.id()
//     }
// }

// impl HpsEngine for NdEuclidPuzzleEngine {
//     fn make_lazy_puzzle(
//         &self,
//         build_ctx: BuildCtx,
//         ctx: &mut EvalCtx<'_>,
//         mut meta: PuzzleListEntry,
//         kwargs: Map,
//     ) -> eyre::Result<Arc<HpsPuzzleEngineOutput>> {
//         let caller_span = ctx.caller_span;

//         unpack_kwargs_eyre!(
//             ctx.runtime, kwargs,
//             colors: Option<String>,
//             twists: Option<String>,
//             ndim: u8,
//             (build, build_span): Arc<FnValue>,
//             remove_internals: Option<bool>,
//             scramble: Option<u32>,
//         );

//         meta.tags.set_opt_color_system(colors.as_deref());
//         meta.tags.set_opt_twist_system(twists.as_deref());

//         if let Err(e) = meta.tags.insert_named("ndim", TagValue::Int(ndim as
// i64)) {             ctx.warn(e.to_string());
//         }

//         let meta = Arc::new(meta);

//         Ok(Arc::new(HpsPuzzleEngineOutput {
//             meta: Arc::clone(&meta),
//             build_puzzle: Box::new(move |build_ctx, runtime| {
//                 let logger = &build_ctx.logger();
//                 let builder =
// Arc::new(Mutex::new(PuzzleBuilder::new(Arc::clone(&meta), ndim)?));
//                 let id = &meta.id;

//                 // Build color system.
//                 if let Some(colors_id) = &colors {
//                     builder.lock().shape.lock().colors =
// ColorSystemBuilder(MaybeAdHoc::Fixed(
// build_ctx.build_str_blocking(colors_id)?,                     ));
//                 } else {
//                     logger.warn(format!("using ad-hoc color system for puzzle
// {id:?}"));                 }

//                 // Build twist system.
//                 if let Some(twists_id) = &twists {
//                     builder.lock().twists =
// TwistSystemBuilder(MaybeAdHoc::Fixed(
// build_ctx.build_str_blocking(twists_id)?,                     ));
//                 } else {
//                     logger.warn(format!("using ad-hoc color system for puzzle
// {id:?}"));                 }

//                 if let Some(remove_internals) = remove_internals {
//                     builder.lock().shape.lock().remove_internals =
// remove_internals;                 }
//                 if let Some(full_scramble_length) = scramble {
//                     builder.lock().full_scramble_length =
// full_scramble_length;                 };

//                 let mut scope = Scope::default();
//                 scope.special.ndim = Some(ndim);
//                 scope.special.puz =
//
// Arc::new(Mutex::new(HpsPuzzle(builder.clone()).at(BUILTIN_SPAN)));
//                 scope.special.shape = Arc::new(Mutex::new(
//                     HpsShape(builder.lock().shape.clone()).at(BUILTIN_SPAN),
//                 ));
//                 scope.special.twists = Arc::new(Mutex::new(
//
// HpsTwistSystem(builder.lock().twists.clone()).at(BUILTIN_SPAN),
// ));                 scope.special.axes = Arc::new(Mutex::new(
//
// HpsAxisSystem(builder.lock().twists.clone()).at(BUILTIN_SPAN),
// ));                 scope.special.id = Some(id.to_string().into());
//                 let scope = Arc::new(scope);
//                 let mut exports = None;
//                 let mut eval_ctx = EvalCtx::new(&scope, runtime, caller_span,
// &mut exports);

//                 let build_fn = Arc::clone(&build);

//                 build_fn
//                     .call(build_span, &mut eval_ctx, vec![], Map::new())
//                     .map_err(|e|
// eval_ctx.runtime.report_and_convert_to_eyre(e))
// .wrap_err("error building puzzle")?;

//                 let b = builder.lock();

//                 // Assign default piece type to remaining pieces.
//                 b.shape.lock().mark_untyped_pieces()?;

//                 b.build(Some(&build_ctx), &mut eval_ctx.warnf())
//             }),
//         }))
//     }
// }
