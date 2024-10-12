use std::collections::{hash_map, HashMap};

use itertools::Itertools;
use mlua::prelude::*;

use crate::{TagType, TagValue, TAGS};

use super::LuaVecString;

pub(super) fn unpack_tags_table<'lua>(
    lua: &'lua Lua,
    table: Option<LuaTable<'lua>>,
) -> LuaResult<HashMap<String, TagValue>> {
    let mut tags = HashMap::new();
    if let Some(table) = table {
        unpack_tags_table_recursive(lua, &mut tags, table, "")?;
    }
    Ok(tags)
}

pub(super) fn merge_tag_sets(
    mut higher_priority: HashMap<String, TagValue>,
    lower_priority: impl IntoIterator<Item = (String, TagValue)>,
) -> HashMap<String, TagValue> {
    for (tag_name, tag_value) in lower_priority {
        if matches!(tag_value, TagValue::Inherited) {
            continue;
        }

        match tag_value {
            // Ignore inherited tags
            TagValue::Inherited => (),

            // Concatenate string lists
            TagValue::StrList(vec) => match higher_priority.entry(tag_name) {
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
                if let hash_map::Entry::Vacant(e) = higher_priority.entry(tag_name) {
                    e.insert(tag_value);
                }
            }
        }
    }

    higher_priority
}

pub(super) fn inherit_parent_tags(tags: &mut HashMap<String, TagValue>) {
    let inherited_tags = tags
        .iter()
        .filter(|(_k, v)| !matches!(v, TagValue::False | TagValue::Inherited))
        .map(|(k, _v)| k.to_owned())
        .collect_vec();

    for tag in inherited_tags {
        // Add parent tags
        let mut s = tag.as_str();
        while let Some((rest, _)) = s.rsplit_once('/') {
            s = rest;
            match tags.entry(s.to_owned()) {
                hash_map::Entry::Occupied(mut e) => match e.get() {
                    TagValue::False => {
                        e.insert(TagValue::Inherited);
                    }
                    _ => (),
                },
                hash_map::Entry::Vacant(e) => {
                    e.insert(TagValue::Inherited);
                }
            }
        }
    }
}

fn unpack_tags_table_recursive<'lua>(
    lua: &'lua Lua,
    tags: &mut HashMap<String, TagValue>,
    table: LuaTable<'lua>,
    prefix: &str,
) -> LuaResult<()> {
    let warn_unknown_tag =
        |tag_name: &str| lua.warning(format!("unknown tag name {tag_name:?}"), false);

    // Sequence entries -- key is ignored and value is tag name
    for v in table.clone().sequence_values() {
        let s: String = v?;
        if let Some(rest) = s.strip_prefix('!') {
            let tag_name = format!("{prefix}{rest}");
            match TAGS.get(&tag_name) {
                Some(tag) if tag.auto => {
                    lua.warning(format!("tag {tag_name:?} cannot be added manually"), false);
                }
                Some(_) => {
                    tags.insert(tag_name.to_owned(), TagValue::False);
                }
                None => {
                    warn_unknown_tag(&tag_name);
                }
            }
        } else {
            let tag_name = format!("{prefix}{s}");
            if let Some(tag) = TAGS.get(&tag_name) {
                match tag.ty {
                    TagType::Bool => {
                        tags.insert(tag_name, TagValue::True);
                    }
                    ty => lua.warning(
                        format!("tag {tag_name:?} requires a value of type {ty:?}"),
                        false,
                    ),
                }
            } else {
                warn_unknown_tag(&tag_name);
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
                let Some(tag) = TAGS.get(&tag_name) else {
                    lua.warning(format!("unknown tag name {s:?}"), false);
                    continue;
                };

                if !tag.ty.may_be_table() {
                    if let LuaValue::Table(subtable) = v {
                        unpack_tags_table_recursive(lua, tags, subtable, &format!("{tag_name}/"))?;
                        continue;
                    }
                }

                match tag_value_from_lua(lua, v, tag.ty) {
                    Ok(tag_value) => {
                        tags.insert(tag_name, tag_value);
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

fn tag_value_from_lua<'lua>(
    lua: &'lua Lua,
    value: LuaValue<'lua>,
    expected_type: TagType,
) -> LuaResult<TagValue> {
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
