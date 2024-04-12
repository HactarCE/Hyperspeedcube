use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use hypershape::Space;
use parking_lot::Mutex;

use crate::builder::{AxisSystemBuilder, TwistSystemBuilder};
use crate::library::{Cached, LibraryDb, LibraryFile, LibraryFileLoadResult, LibraryObjectParams};

use super::*;

#[derive(Debug)]
pub struct TwistSystemParams {
    pub id: Option<String>,

    pub ndim: LuaNdim,
    pub symmetry: Option<LuaSymmetry>,

    axes: NilStringOrRegisteredTable,

    user_build_fn: LuaRegistryKey,
}

impl<'lua> FromLua<'lua> for TwistSystemParams {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table = lua.unpack(value)?;

        let ndim: LuaNdim;
        let symmetry: Option<LuaSymmetry>;
        let axes: LuaNilStringOrTable<'_>;
        let build: LuaFunction<'_>;
        unpack_table!(lua.unpack(table {
            ndim,
            symmetry,
            build,
            axes,
        }));

        Ok(TwistSystemParams {
            id: None,

            ndim,
            symmetry,

            axes: axes.to_lua_registry(lua)?,

            user_build_fn: lua.create_registry_value(build)?,
        })
    }
}

impl LibraryObjectParams for TwistSystemParams {
    const NAME: &'static str = "twist system";

    type Constructed = Arc<Mutex<TwistSystemBuilder>>;

    fn get_file_map(lib: &LibraryDb) -> &BTreeMap<String, Arc<LibraryFile>> {
        &lib.twist_systems
    }
    fn get_id_map_within_file(
        result: &mut LibraryFileLoadResult,
    ) -> &mut HashMap<String, Cached<Self>> {
        &mut result.twist_systems
    }

    fn new_constructed(space: &Arc<Mutex<Space>>) -> LuaResult<Self::Constructed> {
        let axes = AxisSystemBuilder::new(None, Arc::clone(space));
        Ok(TwistSystemBuilder::new(None, axes))
    }
    fn clone_constructed(
        existing: &Self::Constructed,
        space: &Arc<Mutex<Space>>,
    ) -> LuaResult<Self::Constructed> {
        existing.lock().clone(space).into_lua_err()
    }
    fn build(&self, lua: &Lua, space: &Arc<Mutex<Space>>) -> LuaResult<Self::Constructed> {
        let axes = LibraryDb::build_from_value::<AxisSystemParams>(lua, space, &self.axes)?;

        let twist_system_builder = TwistSystemBuilder::new(self.id.clone(), axes);

        let () = LuaSpace(Arc::clone(space)).with_this_as_global_space(lua, || {
            lua.registry_value::<LuaFunction<'_>>(&self.user_build_fn)?
                .call(LuaTwistSystem(Arc::clone(&twist_system_builder)))
                .context("error executing puzzle definition")
        })?;

        Ok(twist_system_builder)
    }
}
