//! Hyperpuzzle catalog functions.

use std::sync::Arc;

use ecow::eco_format;
use hyperpuzzle_core::{Catalog, PuzzleListMetadata, TagSet, Version};
use itertools::Itertools;

use crate::{ErrorExt, EvalCtx, EvalRequestTx, Map, Result, Scope, Str};

/// Adds the built-in functions to the scope.
pub fn define_in(scope: &Scope, catalog: &Catalog, eval_tx: &EvalRequestTx) -> Result<()> {
    let cat = catalog.clone();
    let tx = eval_tx.clone();
    scope.register_builtin_functions(hps_fns![
        #[kwargs(kwargs)]
        fn add_puzzle(ctx: EvalCtx) -> () {
            pop_kwarg!(kwargs, id: String);
            pop_kwarg!(kwargs, name: String = {
                ctx.warn(eco_format!("missing `name` for puzzle `{id}`"));
                id.clone()
            });
            pop_kwarg!(kwargs, aliases: Vec<String> = vec![]);
            pop_kwarg!(kwargs, version: Option<String>);
            pop_kwarg!(kwargs, tags: Option<Arc<Map>>); // TODO
            pop_kwarg!(kwargs, (engine, engine_span): Str);

            let version = parse_version(ctx, &format!("puzzle `{id}`"), version.as_deref())?;

            let tags = TagSet::new(); // TODO

            let meta = PuzzleListMetadata {
                id,
                version,
                name,
                aliases,
                tags,
            };

            let Some(engine) = ctx.runtime.puzzle_engines.get(&*engine).map(Arc::clone) else {
                let engine_list = ctx.runtime.puzzle_engines.keys().collect_vec();
                return Err(format!(
                    "unknown engine {engine:?}; supported engines are {engine_list:?}",
                )
                .at(engine_span));
            };

            let spec = engine.new(ctx, meta, kwargs, cat.clone(), tx.clone())?;
            cat.add_puzzle(Arc::new(spec)).at(ctx.caller_span)?
        }
    ])
}

fn parse_version(ctx: &mut EvalCtx<'_>, thing: &str, s: Option<&str>) -> Result<Version> {
    let Some(version_string) = s else {
        ctx.warn(format!("missing `version` for {thing}"));
        return Ok(Version::PLACEHOLDER);
    };

    fn parse_component(s: &str) -> Result<u32, String> {
        s.parse()
            .map_err(|e| format!("invalid major version because {e}"))
    }

    // IIFE to mimic try_block
    let result = (|| {
        let mut segments = version_string.split('.');
        let version = Version {
            major: parse_component(segments.next().ok_or("missing major version")?)?,
            minor: parse_component(segments.next().unwrap_or("0"))?,
            patch: parse_component(segments.next().unwrap_or("0"))?,
        };
        if segments.next().is_some() {
            return Err(
                "too many segments; only the form `major.minor.patch` is accepted".to_owned(),
            );
        }
        Ok(version)
    })();

    match result {
        Ok(version) => Ok(version),
        Err(e) => {
            ctx.warn(eco_format!("error parsing version string: {e}"));
            Ok(Version::default())
        }
    }
}
