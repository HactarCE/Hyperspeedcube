//! Hyperpuzzlescript interface for the symmetric puzzle engine.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use eyre::{Context, eyre};
use hyperpuzzle_core::CatalogBuilder;
use hyperpuzzle_core::catalog::{Menu, MenuContent};
use hyperpuzzlescript::{Builtins, ErrorExt, FnValue, ListOf, Map, Spanned, Str, hps_fns};
use parking_lot::Mutex;

/// Hyperpuzzlescript interface for the symmetric puzzle engine.
pub struct HpsSymmetric;

impl HpsSymmetric {
    /// ID for the symmetric puzzle [`Menu`].
    pub const MENU_ID: &'static str = "symmetric";
}

impl fmt::Display for HpsSymmetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "symmetric")
    }
}

/// Adds the built-ins.
pub fn define_in(
    builtins: &mut Builtins<'_>,
    catalog: &CatalogBuilder,
) -> hyperpuzzlescript::Result<()> {
    let cat = catalog.clone();

    cat.add_menu(HpsSymmetric::MENU_ID, "Symmetric Puzzles".to_string());

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

            cat.add_menu_node(
                HpsSymmetric::MENU_ID,
                path.into(),
                next,
                priority.unwrap_or(0),
                default,
            )
            .at(ctx.caller_span)?;
        }

        fn add_colors_override(ctx: EvalCtx, id_pattern: Str, f: Arc<FnValue>) -> () {
            ctx.warn(format!("adding color override for {id_pattern:?}"));
        }
    ])
}
