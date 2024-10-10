use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use crate::builder::PuzzleBuilder;
use crate::lua::lua_warn_fn;
use crate::{
    LibraryDb, Puzzle, PuzzleMetadata, PuzzleMetadataExternal, PuzzleProperties, TagType, TagValue,
    TAGS,
};

/// Specification for a puzzle.
#[derive(Debug)]
pub struct PuzzleSpec {
    /// String ID of the puzzle.
    pub id: String,
    /// Version of the puzzle.
    pub version: Version,

    /// Number of dimensions of the space in which the puzzle is constructed.
    pub ndim: LuaNdim,
    /// Lua function to build the puzzle.
    user_build_fn: LuaRegistryKey,

    /// Color system ID.
    pub colors: Option<String>,

    /// User-friendly name for the puzzle. (default = same as ID)
    pub name: Option<String>,
    /// Lua table containing metadata about the puzzle.
    pub meta: PuzzleMetadata,
    /// Lua table containing tags for the puzzle.
    pub tags: HashMap<String, TagValue>,

    /// Whether to automatically remove internal pieces as they are constructed.
    pub remove_internals: Option<bool>,
}

/// Compare by puzzle ID.
impl PartialEq for PuzzleSpec {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
/// Compare by puzzle ID.
impl Eq for PuzzleSpec {}

/// Compare by puzzle ID.
impl PartialOrd for PuzzleSpec {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
/// Compare by puzzle ID.
impl Ord for PuzzleSpec {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        crate::compare_puzzle_ids(&self.id, &other.id)
    }
}

impl<'lua> FromLua<'lua> for PuzzleSpec {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table: LuaTable<'lua> = lua.unpack(value)?;

        let id: String;
        let version: Version;
        let ndim: LuaNdim;
        let build: LuaFunction<'lua>;
        let name: Option<String>;
        let colors: Option<String>;
        let meta: PuzzleMetadata;
        let tags: Option<LuaTable<'lua>>;
        let __generator_tags__: Option<LuaTable<'lua>>; // from generator
        let __example_tags__: Option<LuaTable<'lua>>; // from generator
        let remove_internals: Option<bool>;
        let __generated__: Option<bool>;
        unpack_table!(lua.unpack(table {
            id,
            name,
            version,

            ndim,
            build,

            colors,

            meta,
            tags,
            __generator_tags__,
            __example_tags__,

            remove_internals,

            __generated__, // TODO: this can be hacked
        }));

        let id = if __generated__ == Some(true) {
            id // ID already validated
        } else {
            crate::validate_id(id).into_lua_err()?
        };

        let mut tags_list = vec![];
        // Set tags in order from lowest to highest priority
        for tags_table in [__generator_tags__, tags, __example_tags__] {
            if let Some(table) = tags_table {
                unpack_tags_table(lua, &mut tags_list, table, "")?;
            }
        }

        let mut tags = HashMap::new();
        for (tag, value) in tags_list {
            if !matches!(value, TagValue::False | TagValue::Inherited) {
                // Add parent tags
                let mut s = tag.as_str();
                while let Some((rest, _)) = s.rsplit_once('/') {
                    s = rest;
                    tags.insert(s.to_owned(), TagValue::Inherited);
                }
            }

            tags.insert(tag, value);
        }

        Ok(PuzzleSpec {
            id,
            version,

            ndim,
            user_build_fn: lua.create_registry_value(build)?,

            colors,

            name,
            meta,
            tags,

            remove_internals,
        })
    }
}

impl PuzzleSpec {
    /// Runs initial setup, user Lua code, and final construction for a puzzle.
    pub fn build(&self, lua: &Lua) -> LuaResult<Arc<Puzzle>> {
        let LuaNdim(ndim) = self.ndim;
        let id = self.id.clone();
        let name = self.name.clone().unwrap_or_else(|| {
            lua.warning(format!("missing `name` for puzzle `{id}`"), false);
            self.id.clone()
        });
        let version = self.version.clone();
        let puzzle_builder =
            PuzzleBuilder::new(id, name, version, ndim, self.tags.clone()).into_lua_err()?;
        if let Some(colors_id) = &self.colors {
            puzzle_builder.lock().shape.colors = LibraryDb::build_color_system(lua, colors_id)?;
        }
        if let Some(remove_internals) = self.remove_internals {
            puzzle_builder.lock().shape.remove_internals = remove_internals;
        }
        let space = puzzle_builder.lock().space();

        let () = LuaSpace(space).with_this_as_global_space(lua, || {
            lua.registry_value::<LuaFunction<'_>>(&self.user_build_fn)?
                .call(LuaPuzzleBuilder(Arc::clone(&puzzle_builder)))
                .context("error executing puzzle definition")
        })?;

        let mut puzzle_builder = puzzle_builder.lock();

        // Assign default piece type to remaining pieces.
        puzzle_builder.shape.mark_untyped_pieces().into_lua_err()?;

        puzzle_builder.build(lua_warn_fn(lua)).into_lua_err()
    }

    /// Returns the name or the ID of the puzzle.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }

    /// Returns the URL of the puzzle's WCA page.
    pub fn wca_url(&self) -> Option<String> {
        Some(format!(
            "https://www.worldcubeassociation.org/results/rankings/{}/single",
            self.tags.get("external/wca")?.as_str()?,
        ))
    }
}

impl<'lua> FromLua<'lua> for PuzzleMetadata {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if value.is_nil() {
            return Ok(PuzzleMetadata::default());
        }

        let table: LuaTable<'lua> = lua.unpack(value)?;

        let author: LuaVecString;
        let inventor: LuaVecString;
        let aliases: LuaVecString;
        let external: PuzzleMetadataExternal;
        unpack_table!(lua.unpack(table {
            author,
            inventor,
            aliases,
            external,
        }));

        Ok(PuzzleMetadata {
            authors: author.0,
            inventors: inventor.0,
            aliases: aliases.0,
            external,
            properties: PuzzleProperties::default(),
            generated: false,
            canonical: false,
            meme: false,
        })
    }
}

impl<'lua> FromLua<'lua> for PuzzleMetadataExternal {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if value.is_nil() {
            return Ok(PuzzleMetadataExternal::default());
        }

        let table: LuaTable<'lua> = lua.unpack(value)?;

        let wca: Option<String>;
        unpack_table!(lua.unpack(table { wca }));

        Ok(PuzzleMetadataExternal { wca })
    }
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

fn unpack_tags_table<'lua>(
    lua: &'lua Lua,
    tags_list: &mut Vec<(String, TagValue)>,
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
            if TAGS.contains_key(&tag_name) {
                tags_list.push((tag_name.to_owned(), TagValue::False));
            } else {
                warn_unknown_tag(&tag_name);
            }
        } else {
            let tag_name = format!("{prefix}{s}");
            if let Some(tag) = TAGS.get(&tag_name) {
                match tag.ty {
                    TagType::Bool => {
                        tags_list.push((tag_name, TagValue::True));
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
                        unpack_tags_table(lua, tags_list, subtable, &format!("{tag_name}/"))?;
                        continue;
                    }
                }

                match tag_value_from_lua(lua, v, tag.ty) {
                    Ok(tag_value) => tags_list.push((tag_name, tag_value)),
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
