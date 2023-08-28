use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::PuzzleBuilder;

lua_userdata_value_conversion_wrapper! {
    #[name = "puzzlebuilder"]
    pub struct LuaPuzzleBuilder(Arc<Mutex<PuzzleBuilder>>);
}

impl LuaUserData for LuaNamedUserData<Arc<Mutex<PuzzleBuilder>>> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        // TODO
    }
}

impl LuaPuzzleBuilder {
    pub fn lock(&self) -> MutexGuard<'_, PuzzleBuilder> {
        self.0.lock()
    }

    pub fn get(lua: LuaContext<'_>) -> LuaResult<Self> {
        lua.globals().get("PUZZLE")
    }
    pub fn with<R>(
        lua: LuaContext<'_>,
        f: impl FnOnce(&mut PuzzleBuilder) -> LuaResult<R>,
    ) -> LuaResult<R> {
        f(&mut *Self::get(lua)?.lock())
    }
}
