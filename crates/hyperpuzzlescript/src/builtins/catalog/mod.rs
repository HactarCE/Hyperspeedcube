//! Hyperpuzzle catalog functions.

use ecow::eco_format;
use hyperpuzzle_core::{Catalog, Version};

mod color_systems;
mod generators;
mod puzzles;
mod twist_systems;

use crate::{EvalCtx, EvalRequestTx, Result, Scope};

/// Adds the built-in functions to the scope.
pub fn define_in(scope: &Scope, catalog: &Catalog, eval_tx: &EvalRequestTx) -> Result<()> {
    color_systems::define_in(scope, catalog, eval_tx)?;
    puzzles::define_in(scope, catalog, eval_tx)?;
    twist_systems::define_in(scope, catalog, eval_tx)?;
    Ok(())
}

fn parse_version(ctx: &mut EvalCtx<'_>, thing: &str, s: Option<&str>) -> Result<Version> {
    let Some(version_string) = s else {
        ctx.warn(format!("missing `version` for {thing}"));
        return Ok(Version::PLACEHOLDER);
    };

    fn parse_component(s: &str) -> Result<u32, String> {
        s.parse()
            .map_err(|e| format!("invalid version number: {e}"))
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
