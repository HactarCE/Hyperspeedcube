use std::sync::Arc;

use super::*;
use crate::builder::PuzzleBuilder;
use crate::lua::lua_warn_fn;
use crate::Puzzle;

/// Set of parameters that define a puzzle.
#[derive(Debug)]
pub struct PuzzleParams {
    /// String ID of the puzzle.
    pub id: String,
    /// Number of dimensions of the space in which the puzzle is constructed.
    pub ndim: LuaNdim,

    /// User-friendly name for the puzzle.
    pub name: Option<String>,
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

        let name: Option<String>;
        let ndim: LuaNdim;
        let build: LuaFunction<'lua>;
        let aliases: Option<Vec<String>>;
        let meta: Option<LuaTable<'lua>>;
        let properties: Option<LuaTable<'lua>>;

        unpack_table!(lua.unpack(table {
            name,
            ndim,
            build,

            aliases,
            meta,
            properties,
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

impl PuzzleParams {
    /// Runs initial setup, user Lua code, and final construction for a puzzle.
    pub fn build(&self, lua: &Lua) -> LuaResult<Arc<Puzzle>> {
        let LuaNdim(ndim) = self.ndim;
        let id = self.id.clone();
        let name = self.name.clone().unwrap_or(self.id.clone());
        let puzzle_builder = PuzzleBuilder::new(id, name, ndim).into_lua_err()?;
        let space = puzzle_builder.lock().space();

        let () = LuaSpace(space).with_this_as_global_space(lua, || {
            lua.registry_value::<LuaFunction<'_>>(&self.user_build_fn)?
                .call(LuaPuzzleBuilder(Arc::clone(&puzzle_builder)))
                .context("error executing puzzle definition")
        })?;

        let puzzle_builder = puzzle_builder.lock();
        puzzle_builder.build(lua_warn_fn(lua)).into_lua_err()
    }

    /// Returns the name or the ID of the puzzle.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }
}
