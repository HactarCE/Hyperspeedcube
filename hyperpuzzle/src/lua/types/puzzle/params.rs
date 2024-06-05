use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use hypershape::Space;

use super::*;
use crate::builder::PuzzleBuilder;
use crate::library::{Cached, LibraryDb, LibraryFile, LibraryFileLoadResult, LibraryObjectParams};
use crate::Puzzle;

/// Set of parameters that define a puzzle.
#[derive(Debug)]
pub struct PuzzleParams {
    /// String ID of the puzzle.
    pub id: String,
    /// Number of dimensions of the space in which the puzzle is constructed.
    pub ndim: LuaNdim,

    /// User-friendly name for the puzzle.
    pub name: String,
    /// Alternative user-friendly names for the puzzle.
    pub aliases: Vec<String>,
    /// Lua table containing metadata about the puzzle.
    pub meta: Option<LuaRegistryKey>,
    /// Lua table containing additional properties of the puzzle.
    pub properties: Option<LuaRegistryKey>,

    /// Lua function to build the puzzle.
    user_build_fn: LuaRegistryKey,
}

impl<'lua> FromLua<'lua> for PuzzleParams {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table: LuaTable<'lua> = lua.unpack(value)?;

        let name: String;
        let ndim: LuaNdim;
        let shape: LuaNilStringOrTable<'lua>;
        let twists: LuaNilStringOrTable<'lua>;
        let aliases: Option<Vec<String>>;
        let meta: Option<LuaTable<'lua>>;
        let properties: Option<LuaTable<'lua>>;
        let build: LuaFunction<'lua>;

        unpack_table!(lua.unpack(table {
            name,

            ndim,

            shape,
            twists,

            aliases,
            meta,
            properties,

            build,
        }));

        let create_opt_registry_value = |v| -> LuaResult<Option<LuaRegistryKey>> {
            match v {
                Some(v) => Ok(Some(lua.create_registry_value(v)?)),
                None => Ok(None),
            }
        };

        Ok(PuzzleParams {
            id: String::new(), // This is overwritten in `puzzledb:add()`.
            ndim,

            name,
            aliases: aliases.unwrap_or(vec![]),
            meta: create_opt_registry_value(meta)?,
            properties: create_opt_registry_value(properties)?,

            user_build_fn: lua.create_registry_value(build)?,
        })
    }
}

impl LibraryObjectParams for PuzzleParams {
    const NAME: &'static str = "puzzle";

    type Constructed = Arc<Puzzle>;

    fn get_file_map(lib: &LibraryDb) -> &BTreeMap<String, Arc<LibraryFile>> {
        &lib.puzzles
    }
    fn get_id_map_within_file(
        result: &mut LibraryFileLoadResult,
    ) -> &mut HashMap<String, Cached<Self>> {
        &mut result.puzzles
    }

    fn new_constructed(_space: &Arc<Space>) -> LuaResult<Self::Constructed> {
        Err(LuaError::external("missing puzzle constructor"))
    }
    fn clone_constructed(
        existing: &Self::Constructed,
        _space: &Arc<Space>,
    ) -> LuaResult<Self::Constructed> {
        // Ignore `space` if we don't need it.
        Ok(Arc::clone(existing))
    }
    fn build(&self, lua: &Lua, space: &Arc<Space>) -> LuaResult<Self::Constructed> {
        let LuaNdim(ndim) = self.ndim;
        let puzzle_builder =
            PuzzleBuilder::new(self.id.clone(), self.name.clone(), ndim).into_lua_err()?;
        let space = puzzle_builder.lock().space();

        let () = LuaSpace(space).with_this_as_global_space(lua, || {
            lua.registry_value::<LuaFunction<'_>>(&self.user_build_fn)?
                .call(LuaPuzzleBuilder(Arc::clone(&puzzle_builder)))
                .context("error executing puzzle definition")
        })?;

        let puzzle_builder = puzzle_builder.lock();
        puzzle_builder.build().into_lua_err()
    }
}
