use std::sync::Arc;

use hyperpuzzle_core::{TAGS, TagSet, TagType, TagValue};

use crate::{EvalCtx, List, Map, Result, Type, Value, ValueData, Warning};

pub fn tags_from_map(ctx: &mut EvalCtx<'_>, m: Arc<Map>) -> TagSet {
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
            } else if v.is::<str>() && !matches!(tag.ty, TagType::Str | TagType::StrList) {
                tags.insert_named(&format!("{k}/{v}"), TagValue::True)
                    .map_err(|e| Warning::from(e.to_string()).at(v_span))?;
            } else if v.is::<List>() && tag.ty == TagType::Bool {
                for value in v.to::<List>()? {
                    if value.is::<str>() {
                        tags.insert_named(&format!("{k}/{value}"), TagValue::True)
                            .map_err(|e| Warning::from(e.to_string()).at(value.span))?;
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
