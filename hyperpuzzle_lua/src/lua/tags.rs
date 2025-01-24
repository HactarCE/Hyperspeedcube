use std::collections::hash_map;

use hyperpuzzle_core::{TagSet, TagType, TagValue, TAGS};
use itertools::Itertools;
use mlua::prelude::*;

use super::LuaVecString;

pub(super) fn unpack_tags_table(lua: &Lua, table: Option<LuaTable>) -> LuaResult<TagSet> {
    let mut tags = TagSet::new();
    if let Some(table) = table {
        unpack_tags_table_recursive(lua, &mut tags, table, "")?;
    }
    Ok(tags)
}

pub(super) fn tags_table_to_lua(lua: &Lua, tags: &TagSet) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    for (k, v) in &tags.0 {
        let value = match v {
            TagValue::False => false.into_lua(lua)?,
            TagValue::True => true.into_lua(lua)?,
            TagValue::Inherited => continue,
            TagValue::Int(i) => i.into_lua(lua)?,
            TagValue::Str(s) => s.clone().into_lua(lua)?,
            TagValue::StrList(vec) => lua
                .create_sequence_from(vec.iter().cloned())?
                .into_lua(lua)?,
            TagValue::Puzzle(p) => p.clone().into_lua(lua)?,
        };
        t.raw_set(k.to_string(), value)?;
    }
    Ok(t)
}

pub(super) fn merge_tag_sets(mut higher_priority: TagSet, lower_priority: TagSet) -> TagSet {
    merge_tag_sets_into(&mut higher_priority, lower_priority);
    higher_priority
}

pub(super) fn merge_tag_sets_into(result: &mut TagSet, lower_priority: TagSet) {
    for (tag_name, tag_value) in lower_priority.0 {
        if matches!(tag_value, TagValue::Inherited) {
            continue;
        }

        match tag_value {
            // Ignore inherited tags
            TagValue::Inherited => (),

            // Concatenate string lists
            TagValue::StrList(vec) => match result.0.entry(tag_name) {
                hash_map::Entry::Occupied(mut e) => {
                    if let Some(str_list) = e.get_mut().as_str_list_mut() {
                        str_list.extend(vec);
                    }
                }
                hash_map::Entry::Vacant(e) => {
                    e.insert(TagValue::StrList(vec));
                }
            },

            TagValue::False
            | TagValue::True
            | TagValue::Int(_)
            | TagValue::Str(_)
            | TagValue::Puzzle(_) => {
                if let hash_map::Entry::Vacant(e) = result.0.entry(tag_name) {
                    e.insert(tag_value);
                }
            }
        }
    }
}

pub(super) fn inherit_parent_tags(tags: &mut TagSet) {
    let inherited_tags = tags
        .0
        .iter()
        .filter(|(_k, v)| !matches!(v, TagValue::False | TagValue::Inherited))
        .map(|(k, _v)| k.to_owned())
        .collect_vec();

    for tag in inherited_tags {
        // Add parent tags
        for parent_name in hyperpuzzle_core::TAGS.ancestors(&tag) {
            let Ok(parent) = tags.entry_named(parent_name) else {
                continue;
            };
            match parent {
                hash_map::Entry::Occupied(mut e) => {
                    if let TagValue::False = e.get() {
                        e.insert(TagValue::Inherited);
                    }
                }
                hash_map::Entry::Vacant(e) => {
                    e.insert(TagValue::Inherited);
                }
            }
        }
    }
}

fn unpack_tags_table_recursive(
    lua: &Lua,
    tags: &mut TagSet,
    table: LuaTable,
    prefix: &str,
) -> LuaResult<()> {
    // Sequence entries -- key is ignored and value is tag name
    for v in table.clone().sequence_values() {
        let s: String = v?;
        if let Some(rest) = s.strip_prefix('!') {
            let tag_name = format!("{prefix}{rest}");
            match TAGS.get(&tag_name) {
                Ok(tag) if tag.auto => {
                    lua.warning(format!("tag {tag_name:?} cannot be added manually"), false);
                }
                Ok(tag) => tags.insert(tag, TagValue::False),
                Err(e) => lua.warning(e.to_string(), false),
            }
        } else {
            let tag_name = format!("{prefix}{s}");
            match TAGS.get(&tag_name) {
                Ok(tag) => match tag.ty {
                    TagType::Bool => tags.insert(tag, TagValue::True),
                    ty => lua.warning(
                        format!("tag {tag_name:?} requires a value of type {ty:?}"),
                        false,
                    ),
                },
                Err(e) => lua.warning(e.to_string(), false),
            }
        }
    }

    // Non-sequence entry; key is tag name and value is tag value
    for pair in table.pairs() {
        let (k, v) = pair?;
        match k {
            LuaValue::Integer(_) => (), // sequence entries have already been handled

            LuaValue::String(s) => {
                let tag_name = format!("{prefix}{}", s.to_string_lossy());
                let tag = match TAGS.get(&tag_name) {
                    Ok(t) => t,
                    Err(e) => {
                        lua.warning(e.to_string(), false);
                        continue;
                    }
                };

                if !tag.ty.may_be_table() {
                    if let LuaValue::Table(subtable) = v {
                        unpack_tags_table_recursive(lua, tags, subtable, &format!("{tag_name}/"))?;
                        continue;
                    }
                }

                match tag_value_from_lua(lua, v, tag.ty) {
                    Ok(tag_value) => {
                        tags.insert(tag, tag_value);
                    }
                    Err(e) => {
                        lua.warning(format!("bad tag value for {tag_name:?}: {e}"), false);
                    }
                }
            }

            _ => lua.warning(format!("bad key {k:?} in tag table"), false),
        }
    }

    Ok(())
}

fn tag_value_from_lua(lua: &Lua, value: LuaValue, expected_type: TagType) -> LuaResult<TagValue> {
    if matches!(value, LuaValue::Boolean(false)) {
        return Ok(TagValue::False);
    }
    match expected_type {
        TagType::Bool => match bool::from_lua(value, lua)? {
            true => Ok(TagValue::True),
            false => Ok(TagValue::False),
        },
        TagType::Int => Ok(TagValue::Int(i64::from_lua(value, lua)?)),
        TagType::Str => Ok(TagValue::Str(String::from_lua(value, lua)?)),
        TagType::StrList => Ok(TagValue::StrList(LuaVecString::from_lua(value, lua)?.0)),
        TagType::Puzzle => Ok(TagValue::Puzzle(String::from_lua(value, lua)?)),
    }
}
