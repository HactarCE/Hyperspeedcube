use std::sync::Arc;

use super::*;
use crate::builder::PuzzleBuilder;
use crate::lua::lua_warn_fn;
use crate::{LibraryDb, Puzzle, TagSet, TagValue};

/// Specification for a puzzle.
#[derive(Debug)]
pub struct PuzzleSpec {
    /// String ID of the puzzle.
    pub id: String,
    /// Version of the puzzle.
    pub version: Version,

    /// User-friendly name for the puzzle. (default = same as ID)
    pub name: Option<String>,
    /// Aliases for the puzzle.
    pub aliases: Vec<String>,
    /// Lua table containing tags for the puzzle.
    pub tags: TagSet,

    /// Color system ID.
    pub colors: Option<String>,

    /// Number of dimensions of the space in which the puzzle is constructed.
    pub ndim: LuaNdim,
    /// Lua function to build the puzzle.
    user_build_fn: LuaRegistryKey,

    /// Whether to automatically remove internal pieces as they are constructed.
    pub remove_internals: Option<bool>,
    /// Number of moves for a full scramble.
    pub full_scramble_length: Option<u32>,
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

impl FromLua for PuzzleSpec {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let table: LuaTable = lua.unpack(value)?;

        let id: String;
        let version: Version;
        let name: Option<String>;
        let aliases: Option<Vec<String>>;
        let tags: Option<LuaTable>;
        let colors: Option<String>;
        let ndim: LuaNdim;
        let build: LuaFunction;
        let remove_internals: Option<bool>;
        let scramble: Option<u32>;
        unpack_table!(lua.unpack(table {
            id,
            version,
            name,
            aliases,
            tags,
            colors,
            ndim,
            build,
            remove_internals,
            scramble,
        }));

        let id = crate::validate_id(id).into_lua_err()?;
        let mut tags = crate::lua::tags::unpack_tags_table(lua, tags)?;

        if let Some(color_system_id) = colors.clone() {
            tags.insert_named("colors/system", TagValue::Str(color_system_id))
                .map_err(LuaError::external)?;
        }

        crate::lua::tags::inherit_parent_tags(&mut tags);

        Ok(PuzzleSpec {
            id,
            version,

            name,
            aliases: aliases.unwrap_or_default(),
            tags,

            colors,

            ndim,
            user_build_fn: lua.create_registry_value(build)?,

            remove_internals,
            full_scramble_length: scramble,
        })
    }
}

impl PuzzleSpec {
    /// Runs initial setup, user Lua code, and final construction for a puzzle.
    pub fn build(&self, lua: &Lua) -> LuaResult<Arc<Puzzle>> {
        let LuaNdim(ndim) = self.ndim;
        let id = self.id.clone();
        let version = self.version.clone();
        let name = self.name.clone().unwrap_or_else(|| {
            lua.warning(format!("missing `name` for puzzle `{id}`"), false);
            self.id.clone()
        });
        let aliases = self.aliases.clone();
        let puzzle_builder =
            PuzzleBuilder::new(id, version, name, aliases, ndim, self.tags.clone())
                .into_lua_err()?;
        if let Some(colors_id) = &self.colors {
            puzzle_builder.lock().shape.colors = LibraryDb::build_color_system(lua, colors_id)?;
        }
        if let Some(remove_internals) = self.remove_internals {
            puzzle_builder.lock().shape.remove_internals = remove_internals;
        }
        if let Some(full_scramble_length) = self.full_scramble_length {
            puzzle_builder.lock().full_scramble_length = full_scramble_length;
        }
        let space = puzzle_builder.lock().space();

        let () = LuaSpace(space).with_this_as_global_space(lua, || {
            lua.registry_value::<LuaFunction>(&self.user_build_fn)?
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
}
