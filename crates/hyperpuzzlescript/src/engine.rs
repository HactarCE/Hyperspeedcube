//! Engines are used to construct a catalog object from keyword arguments.
//!
//! In the example below, the engine registered with the name `"my_engine"`
//! receives a map containing the keyword arguments `a`, `b`, and `c`.
//!
//! ```hps
//! add_puzzle(
//!     // Certain keys are common for all engines
//!     // so they are handled automatically
//!     id = "my_puzzle",
//!     name = "My Puzzle",
//!     aliases = ["My Example Puzzle"],
//!     version = "0.1.0",
//!     tags = #{ big = true },
//!     engine = "my_engine",
//!
//!     // All other keys are passed to the engine
//!     a = "foo",
//!     b = 36,
//!     c = "baz",
//! )
//! ```

use std::sync::Arc;

use ecow::eco_format;
use hyperpuzzle_core::{
    BuildCtx, CatalogBuilder, CatalogIdent, CatalogObject, Generator, GeneratorParam,
    GeneratorParamValidation, GeneratorSubsetParam, TypedCatalogIdValue,
};
use itertools::Itertools;

use crate::{
    EvalCtx, EvalRequestTx, FnValue, FullDiagnostic, Map, Num, Scope, Span, Type, Value, ValueData,
};

#[derive(Debug)]
pub enum HpsEngineError {
    Hps(FullDiagnostic),
    Eyre(eyre::Report),
}

impl From<FullDiagnostic> for HpsEngineError {
    fn from(value: FullDiagnostic) -> Self {
        Self::Hps(value)
    }
}

impl From<eyre::Report> for HpsEngineError {
    fn from(value: eyre::Report) -> Self {
        Self::Eyre(value)
    }
}

impl HpsEngineError {
    pub fn to_eyre(self, tx: &EvalRequestTx) -> eyre::Report {
        match self {
            HpsEngineError::Hps(full_diagnostic) => {
                tx.eval_blocking_raw(|runtime| runtime.report_and_convert_to_eyre(full_diagnostic))
            }
            HpsEngineError::Eyre(report) => report,
        }
    }

    pub fn to_full_diagnostic(self, span: Span) -> FullDiagnostic {
        match self {
            HpsEngineError::Hps(full_diagnostic) => full_diagnostic,
            // Alternate formatting includes more detail for `eyre::Report`s.
            HpsEngineError::Eyre(report) => crate::Error::User(eco_format!("{report:#}")).at(span),
        }
    }
}

/// Trait that serves as the primary entry point for custom puzzle engines.
///
/// This may add tags, aliases or other metadata to the provided
/// [`PuzzleListEntry`] for the puzzle generator but must not modify the ID,
/// version, or name. Expensive computation should be deferred to the `Box<dyn
/// Fn() -> Arc<Puzzle>>`.
///
/// See the [module documentation](`self`) for more info.
pub trait HpsEngine: Send + Sync {
    /// Adds catalog entries for an object using [`CatalogBuilder::add`] the
    /// provided [`CatalogBuilder`].
    ///
    /// The provided [`EvalRequestTx`] may be used to evaluate HPS functions,
    /// however only in callbacks. **Evaluating HPS functions via
    /// [`EvalRequestTx`] during the runtime of `add_catalog_entries()` _will_
    /// deadlock.** Instead use the provided [`EvalCtx`].
    ///
    /// For a puzzle, this must add a [`Puzzle`] generator and a
    /// [`PuzzleListEntry`] generator. It may add other generators if needed.
    /// When passed an empty parameter list, the [`PuzzleListEntry`] generator
    /// must return the list entry for the generator itself.
    ///
    /// For a twist system, this must add a [`TwistSystem`] generator.
    ///
    /// This function is run for every generator whenever the catalog is loaded
    /// so any expensive computations should be deferred to
    /// [`Generator::generate`].
    fn add_catalog_entries(
        &self,
        catalog: &CatalogBuilder,
        eval_tx: &EvalRequestTx,
        ctx: &mut EvalCtx<'_>,
        hps_gen: HpsGenerator,
    ) -> Result<(), HpsEngineError>;
}

/// HPS generator data.
///
/// This struct is the input passed to implementors of [`HpsEngine`], which
/// construct puzzles, twist systems, etc.
#[derive(Debug, Clone)]
pub struct HpsGenerator {
    pub id: CatalogIdent,
    /// Name and aliases. This may be empty.
    pub names: Vec<String>,
    pub kwargs: Map,
    /// `gen` function and parameters, or `None` if the generator takes no
    /// parameters.
    pub gen_fn: Option<HpsGeneratorFn>,
}

/// HPS generator function.
///
/// See [`HpsEngine`].
#[derive(Debug, Clone)]
pub struct HpsGeneratorFn {
    pub params: Vec<GeneratorParam>,
    pub subset_param: Option<GeneratorSubsetParam>,
    pub gen_fn: Arc<FnValue>,
    pub gen_fn_span: Span,
}

impl HpsGenerator {
    /// Constructs a [`Generator`] with custom parameter validation.
    ///
    /// See [`Self::make_generator()`] for details.
    pub fn make_generator_with_validation<T: CatalogObject>(
        &self,
        eval_tx: &EvalRequestTx,
        validation: GeneratorParamValidation,
        generate: impl 'static
        + Send
        + Sync
        + Fn(BuildCtx, &EvalRequestTx, Map) -> Result<Arc<T>, HpsEngineError>,
    ) -> Arc<Generator<T>> {
        let g = self.gen_fn.clone();
        let tx = eval_tx.clone();
        let self_kwargs = Arc::new(self.kwargs.clone());
        let generate = Arc::new(generate);
        Arc::new(Generator {
            id: self.id.clone(),
            params: g.as_ref().map(|g| g.params.clone()).unwrap_or(vec![]),
            subset_param: g.as_ref().map(|g| g.subset_param.clone()).unwrap_or(None),
            validation,
            generate: Box::new(move |build_ctx| {
                if let Some(g) = &g
                    && !build_ctx.id().args.is_empty()
                {
                    let mut args: Vec<Value> = std::iter::zip(&g.params, &build_ctx.id().args)
                        .map(|(param, arg)| param.typed_value(arg.clone()))
                        .map_ok(|v| param_value_into_hps(&v))
                        .try_collect()?;
                    if let Some(subset) = &build_ctx.id().subset {
                        args.push(ValueData::Str((**subset).into()).at(crate::BUILTIN_SPAN));
                    }

                    let gen_fn = Arc::clone(&g.gen_fn);
                    let gen_fn_span = g.gen_fn_span;
                    let self_kwargs = Arc::clone(&self_kwargs);

                    let mut scope = Scope::default();
                    scope.special.id = Some(build_ctx.id().to_string().into());
                    tx.eval_blocking(Arc::new(scope), move |ctx| {
                        let mut return_value = gen_fn.call(gen_fn_span, ctx, args, Map::new())?;
                        if let Ok(m) = return_value.as_map_mut(crate::BUILTIN_SPAN) {
                            for (k, v) in &*self_kwargs {
                                match m.entry(k.clone()) {
                                    indexmap::map::Entry::Occupied(_) => ctx.warn_at(
                                        v.span,
                                        format!("map key {k:?} is overwritten by generator output"),
                                    ),
                                    indexmap::map::Entry::Vacant(e) => {
                                        e.insert(v.clone());
                                    }
                                }
                            }
                        }
                        Ok(return_value)
                    })
                    .map_err(HpsEngineError::Hps)
                    .and_then(|val| {
                        // `generate` must be called outside of the HPS thread
                        // because it may request other catalog objects, which
                        // may want to use the HPS thread.
                        if val.is::<str>() {
                            Ok(build_ctx.build_str_blocking(val.as_ref()?)?)
                        } else if val.is::<Map>() {
                            Ok(generate(build_ctx, &tx, val.unwrap_or_clone_arc()?)?)
                        } else {
                            Err(val.type_error(Type::Str | Type::Map))?
                        }
                    })
                } else {
                    generate(build_ctx, &tx, (*self_kwargs).clone())
                }
                .map_err(|e| e.to_eyre(&tx))
            }),
        })
    }

    /// Constructs a [`Generator`] that calls `generate` when parameter values
    /// are supplied or no parameters are required, and returns `default` when
    /// required parameter values are required but missing.
    ///
    /// See [`Self::make_generator()`] for details.
    pub fn make_generator_with_empty<T: CatalogObject>(
        &self,
        eval_tx: &EvalRequestTx,
        default: Arc<T>,
        generate: impl 'static
        + Send
        + Sync
        + Fn(BuildCtx, &EvalRequestTx, Map) -> Result<Arc<T>, HpsEngineError>,
    ) -> Arc<Generator<T>> {
        self.make_generator_with_validation(
            eval_tx,
            GeneratorParamValidation { allow_empty: true },
            move |build_ctx, tx, kwargs| {
                if build_ctx.id().args.is_empty() {
                    Ok(Arc::clone(&default))
                } else {
                    generate(build_ctx, tx, kwargs)
                }
            },
        )
    }

    /// Constructs a [`Generator`] that does not allow missing parameter values.
    ///
    /// The `generate` function takes three arguments:
    /// - [`BuildCtx`], which contains the ID, and thus the parameters
    /// - [`EvalRequestTx`], which is the same as the provided `EvalRequestTx`
    ///   for convenience
    /// - [`Map`] returned by the user-provided `gen` function, or `self.kwargs`
    ///   if there is no `gen` function.
    pub fn make_generator<T: CatalogObject>(
        &self,
        eval_tx: &EvalRequestTx,
        generate: impl 'static
        + Send
        + Sync
        + Fn(BuildCtx, &EvalRequestTx, Map) -> Result<Arc<T>, HpsEngineError>,
    ) -> Arc<Generator<T>> {
        self.make_generator_with_validation(
            eval_tx,
            GeneratorParamValidation { allow_empty: false },
            generate,
        )
    }
}

pub(super) fn param_value_into_hps(value: &TypedCatalogIdValue) -> Value {
    match value {
        TypedCatalogIdValue::Id(id) => ValueData::Str(id.to_string().into()),
        TypedCatalogIdValue::Bool(b) => ValueData::Bool(*b),
        TypedCatalogIdValue::Int(i) => ValueData::Num(*i as Num),
        TypedCatalogIdValue::List(l) => {
            ValueData::List(Arc::new(l.iter().map(param_value_into_hps).collect()))
        }
    }
    .at(crate::BUILTIN_SPAN)
}
