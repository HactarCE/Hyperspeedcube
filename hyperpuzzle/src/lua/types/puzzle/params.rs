use hypershape::Space;
use parking_lot::Mutex;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::builder::PuzzleBuilder;
use crate::library::{Cached, LibraryDb, LibraryFile, LibraryFileLoadResult, LibraryObjectParams};
use crate::Puzzle;

use super::*;

#[derive(Debug)]
pub struct PuzzleParams {
    pub id: String,

    pub name: String,
    pub ndim: LuaNdim,
    pub symmetry: Option<LuaSymmetry>,

    shape: NilStringOrRegisteredTable,
    twists: NilStringOrRegisteredTable,

    aliases: Option<LuaRegistryKey>,
    meta: Option<LuaRegistryKey>,
    properties: Option<LuaRegistryKey>,

    user_build_fn: LuaRegistryKey,
}

impl<'lua> FromLua<'lua> for PuzzleParams {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table: LuaTable<'lua> = lua.unpack(value)?;

        let name: String;
        let ndim: LuaNdim;
        let symmetry: Option<LuaSymmetry>;
        let shape: LuaNilStringOrTable<'lua>;
        let twists: LuaNilStringOrTable<'lua>;
        let aliases: Option<LuaTable<'lua>>;
        let meta: Option<LuaTable<'lua>>;
        let properties: Option<LuaTable<'lua>>;
        let build: LuaFunction<'lua>;

        unpack_table!(lua.unpack(table {
            name,

            ndim,
            symmetry,

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

            name,
            ndim,
            symmetry,

            shape: shape.to_lua_registry(lua)?,
            twists: twists.to_lua_registry(lua)?,

            aliases: create_opt_registry_value(aliases)?,
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

    fn new_constructed(_space: &Arc<Mutex<Space>>) -> LuaResult<Self::Constructed> {
        Err(LuaError::external("missing puzzle constructor"))
    }
    fn clone_constructed(
        existing: &Self::Constructed,
        _space: &Arc<Mutex<Space>>,
    ) -> LuaResult<Self::Constructed> {
        // Ignore `space` if we don't need it.
        Ok(Arc::clone(existing))
    }
    fn build(&self, lua: &Lua, space: &Arc<Mutex<Space>>) -> LuaResult<Self::Constructed> {
        let shape = LibraryDb::build_from_value::<ShapeParams>(lua, space, &self.shape)?;
        let twists = LibraryDb::build_from_value::<TwistSystemParams>(lua, space, &self.twists)?;

        let puzzle_builder = Arc::new(Mutex::new(PuzzleBuilder {
            id: self.id.clone(),
            name: self.name.clone(),

            symmetry: self.symmetry.clone().map(|sym| sym.schlafli),

            shape,
            twists,
        }));

        let () = LuaSpace(Arc::clone(space)).with_this_as_global_space(lua, || {
            lua.registry_value::<LuaFunction<'_>>(&self.user_build_fn)?
                .call(LuaPuzzleBuilder(Arc::clone(&puzzle_builder)))
                .context("error executing puzzle definition")
        })?;

        let puzzle_builder = puzzle_builder.lock();
        puzzle_builder.build().into_lua_err()
    }
}
