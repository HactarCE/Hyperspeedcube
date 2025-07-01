use std::collections::HashMap;
use std::sync::Arc;

use ecow::eco_format;
use eyre::eyre;
use hyperpuzzle_core::catalog::BuildTask;
use hyperpuzzle_core::{
    Catalog, PuzzleListMetadata, PuzzleSpec, PuzzleSpecGenerator, Redirectable, TAGS, TagSet,
    TagType, TagValue,
};
use itertools::Itertools;

use crate::util::pop_map_key;
use crate::{
    Builtins, ErrorExt, EvalCtx, EvalRequestTx, FnValue, List, Map, Result, Scope, Spanned, Str,
    Type, Value, ValueData,
};

/// Adds the built-in functions.
pub fn define_in(
    builtins: &mut Builtins<'_>,
    catalog: &Catalog,
    eval_tx: &EvalRequestTx,
) -> Result<()> {
    let cat = catalog.clone();
    let tx = eval_tx.clone();
    builtins.set_fns(hps_fns![
        /// Adds a puzzle to the catalog.
        ///
        /// This function takes the following named arguments:
        ///
        /// - `id: Str`
        /// - `name: Str?`
        /// - `aliases: List[Str]?`
        /// - `tags: Map?`
        /// - `engine: Str`
        ///
        /// The function takes other keyword arguments depending on the value of
        /// `engine`.
        #[kwargs(kwargs)]
        fn add_puzzle(ctx: EvalCtx) -> () {
            cat.add_puzzle(Arc::new(puzzle_spec_from_kwargs(
                ctx, kwargs, &cat, &tx, None, None,
            )?))
            .at(ctx.caller_span)?
        }
    ])?;

    let cat = catalog.clone();
    let tx = eval_tx.clone();
    builtins.set_fns(hps_fns![
        /// Adds a puzzle generator to the catalog.
        ///
        /// This function takes the following named arguments:
        ///
        /// - `id: Str`
        /// - `name: Str?`
        /// - `aliases: List[Str]?`
        /// - `version: Str?`
        /// - `tags: Map?`
        /// - `params: List[Map]`
        /// - `examples: List[Map]`
        /// - `gen: Fn(..) -> Map`
        ///
        /// Other keyword arguments are copied into the output of `gen`.
        #[kwargs(kwargs)]
        fn add_puzzle_generator(ctx: EvalCtx) -> () {
            pop_kwarg!(kwargs, id: String);
            pop_kwarg!(kwargs, name: String = {
                ctx.warn(eco_format!("missing `name` for puzzle generator `{id}`"));
                id.clone()
            });
            pop_kwarg!(kwargs, aliases: Vec<String> = vec![]);
            pop_kwarg!(kwargs, tags: Option<Arc<Map>>);
            pop_kwarg!(kwargs, params: Vec<Spanned<Arc<Map>>> );
            pop_kwarg!(kwargs, examples: Vec<Spanned<Arc<Map>>> = vec![]);
            pop_kwarg!(kwargs, (r#gen, gen_span): Arc<FnValue>);
            let tags_map = tags;

            let caller_span = ctx.caller_span;

            // Parse `version`, but leave it in so that the puzzles inherit it.
            let version = super::parse_version(
                ctx,
                &format!("puzzle generator `{id}`"),
                kwargs
                    .get("version")
                    .map(|v| v.as_ref::<str>())
                    .transpose()?,
            )?;

            // Add `#generator` tag.
            let mut tags = tags_map.map(|m| tags_from_map(ctx, m)).unwrap_or_default();
            for tag in tags.0.keys().filter(|tag| tag.starts_with("type/")) {
                ctx.warn(format!("generator `{id}` should not have tag {tag:?}"));
            }
            tags.insert_named("type/generator", TagValue::True)
                .at(caller_span)?;

            // Add `#experimental` tag.
            tags.set_experimental_or_expect_stable(
                version,
                ctx.warnf(),
                &format!("generator `{id}`"),
            )
            .at(ctx.caller_span)?;

            let gen_meta = super::generators::GeneratorMeta {
                id,
                params: super::generators::params_from_array(params)?,
                gen_fn: r#gen,
                gen_span,
                extra: kwargs,
            };

            // Add examples.
            let mut example_specs = HashMap::new();
            for (example, example_span) in examples {
                let mut example = Arc::unwrap_or_clone(example);
                let params: Vec<Value> = pop_map_key(&mut example, example_span, "params")?;
                let generator_param_values = params.iter().map(|v| v.to_string()).collect();

                let puzzle_spec_result = match gen_meta.generate_spec(ctx, generator_param_values) {
                    Ok(Redirectable::Direct(spec_kwargs)) => puzzle_spec_from_kwargs(
                        ctx,
                        spec_kwargs,
                        &cat,
                        &tx,
                        Some(tags.clone()),
                        Some((example, example_span)),
                    ),
                    Ok(Redirectable::Redirect(other)) => {
                        ctx.warn_at(
                            example_span,
                            format!("ignoring example because it redirects to {other:?}"),
                        );
                        continue;
                    }
                    Err(e) => Err(e),
                };

                match puzzle_spec_result {
                    Ok(puzzle_spec) => {
                        example_specs.insert(puzzle_spec.meta.id.clone(), Arc::new(puzzle_spec));
                    }
                    Err(e) => {
                        ctx.runtime.report_diagnostic(e);
                        ctx.warn_at(example_span, "error building example");
                    }
                }
            }

            let cat2 = cat.clone();
            let tx = tx.clone();

            let mut generator_tags = tags.clone();
            generator_tags.inherit_parent_tags();

            let spec = PuzzleSpecGenerator {
                meta: Arc::new(PuzzleListMetadata {
                    id: gen_meta.id.clone(),
                    version,
                    name,
                    aliases,
                    tags: generator_tags,
                }),
                params: gen_meta.params.clone(),
                examples: example_specs,
                generate: Box::new(move |build_ctx, param_values| {
                    build_ctx.progress.lock().task = BuildTask::GeneratingSpec;

                    let cat2 = cat2.clone();
                    let tx2 = tx.clone();
                    let tags = tags.clone();

                    let scope = Scope::new();
                    let gen_meta = gen_meta.clone();

                    tx.clone().eval_blocking(move |runtime| {
                        let mut ctx = EvalCtx {
                            scope: &scope,
                            runtime,
                            caller_span,
                            exports: &mut None,
                        };

                        // IIFE to mimic try_block
                        (|| {
                            gen_meta
                                .generate_spec(&mut ctx, param_values)?
                                .try_map(|spec| {
                                    // TODO: add tags
                                    puzzle_spec_from_kwargs(
                                        &mut ctx,
                                        spec,
                                        &cat2,
                                        &tx2,
                                        Some(tags.clone()),
                                        None,
                                    )
                                    .map(Arc::new)
                                })
                        })()
                        .map_err(|e| {
                            let s = e.to_string(&*ctx.runtime);
                            ctx.runtime.report_diagnostic(e);
                            eyre!(s)
                        })
                    })
                }),
            };

            cat.add_puzzle_generator(Arc::new(spec))
                .at(ctx.caller_span)?
        }
    ])
}

fn puzzle_spec_from_kwargs(
    ctx: &mut EvalCtx<'_>,
    mut kwargs: Map,
    catalog: &Catalog,
    eval_tx: &EvalRequestTx,
    generator_tags: Option<TagSet>,
    example_data: Option<Spanned<Map>>,
) -> Result<PuzzleSpec> {
    pop_kwarg!(kwargs, id: String);
    pop_kwarg!(kwargs, name: String = {
        ctx.warn(eco_format!("missing `name` for puzzle `{id}`"));
        id.clone()
    });
    pop_kwarg!(kwargs, aliases: Vec<String> = vec![]);
    pop_kwarg!(kwargs, version: Option<String>);
    pop_kwarg!(kwargs, tags: Option<Arc<Map>>); // TODO
    pop_kwarg!(kwargs, (engine, engine_span): Str);
    let mut name = name;
    let mut aliases = aliases;
    let tags_map = tags;

    let mut tags = generator_tags.unwrap_or_default();
    if let Some(tags_map) = tags_map {
        tags.merge_from(tags_from_map(ctx, tags_map));
    }

    if let Some((mut example, example_span)) = example_data {
        let new_name: Option<String> = pop_map_key(&mut example, example_span, "name")?;
        if let Some(new_name) = new_name {
            name = new_name;
        }

        let new_aliases: Option<Vec<String>> = pop_map_key(&mut example, example_span, "aliases")?;
        if let Some(new_aliases) = new_aliases {
            aliases.extend(new_aliases);
        }

        let new_tags: Option<Arc<Map>> = pop_map_key(&mut example, example_span, "tags")?;
        if let Some(new_tags) = new_tags {
            tags.merge_from(tags_from_map(ctx, new_tags));
        }

        crate::util::expect_end_of_map(example, example_span)?;
    }

    let version = super::parse_version(ctx, &format!("puzzle `{id}`"), version.as_deref())?;

    if tags.has_present("type/generator") {
        // Remove `#generator` tag.
        tags.0.remove("type/generator");

        // Add `#generated` tag.
        tags.insert_named("generated", TagValue::True)
            .at(ctx.caller_span)?;
    }

    // Add `#experimental` tag.
    tags.set_experimental_or_expect_stable(version, ctx.warnf(), &format!("puzzle `{id}`"))
        .at(ctx.caller_span)?;

    tags.inherit_parent_tags();

    let meta = PuzzleListMetadata {
        id,
        version,
        name,
        aliases,
        tags,
    };

    let Some(engine) = ctx.runtime.puzzle_engines.get(&*engine).map(Arc::clone) else {
        let engine_list = ctx.runtime.puzzle_engines.keys().collect_vec();
        return Err(
            format!("unknown engine {engine:?}; supported engines: {engine_list:?}",)
                .at(engine_span),
        );
    };

    engine.new(ctx, meta, kwargs, catalog.clone(), eval_tx.clone())
}

fn tags_from_map(ctx: &mut EvalCtx<'_>, m: Arc<Map>) -> TagSet {
    let mut tags = TagSet::new();
    unpack_tags_recursive(ctx, &mut tags, Arc::unwrap_or_clone(m), "");
    tags
}

fn unpack_tags_recursive(ctx: &mut EvalCtx<'_>, tags: &mut TagSet, m: Map, prefix: &str) {
    for (k, v) in m {
        let v_span = v.span;

        let tag_name = format!("{prefix}{k}");
        let tag = match TAGS.get(&tag_name) {
            Ok(t) => t,
            Err(e) => {
                ctx.warn_at(v_span, e.to_string());
                continue;
            }
        };

        // IIFE to mimic try_block
        let result = (|| {
            if v.is::<Map>() {
                unpack_tags_recursive(ctx, tags, v.unwrap_or_clone_arc()?, &format!("{tag_name}/"));
            } else if v.is::<str>() && tag.ty == TagType::Bool {
                tags.insert_named(&format!("{k}/{v}"), TagValue::True)
                    .at(v_span)?;
            } else if v.is::<List>() && tag.ty == TagType::Bool {
                for value in v.to::<List>()? {
                    let value_span = value.span;
                    if value.is::<str>() {
                        tags.insert_named(&format!("{k}/{value}"), TagValue::True)
                            .at(value_span)?;
                    } else if value.is::<Map>() {
                        unpack_tags_recursive(ctx, tags, value.unwrap_or_clone_arc()?, prefix);
                    }
                }
            } else {
                tags.insert(tag, unpack_tag_value(v, tag.ty)?);
            }
            Ok(())
        })();

        if let Err(e) = result {
            ctx.runtime.report_diagnostic(e);
        }
    }
}

fn unpack_tag_value(value: Value, expected_type: TagType) -> Result<TagValue> {
    if matches!(value.data, ValueData::Bool(false)) {
        return Ok(TagValue::False);
    }
    match expected_type {
        TagType::Bool => match value.to()? {
            true => Ok(TagValue::True),
            false => Ok(TagValue::False),
        },
        TagType::Int => Ok(TagValue::Int(value.to()?)),
        TagType::Str => Ok(TagValue::Str(value.to()?)),
        TagType::StrList => Ok({
            if value.is::<str>() {
                TagValue::StrList(vec![value.to()?])
            } else if value.is::<List>() {
                TagValue::StrList(value.to()?)
            } else {
                return Err(value.type_error(Type::Str | Type::List(Some(Box::new(Type::Str)))));
            }
        }),
        TagType::Puzzle => Ok(TagValue::Puzzle(value.to()?)),
    }
}
