use std::sync::Arc;

use hypershape::prelude::*;
use parking_lot::{Mutex, MutexGuard};

use super::*;

lua_userdata_value_conversion_wrapper! {
    #[name = "space"]
    pub struct LuaSpace(Arc<Mutex<Space>>);
}

impl LuaUserData for LuaNamedUserData<Arc<Mutex<Space>>> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method("ndim", |_lua, Self(this), ()| Ok(this.lock().ndim()));
    }
}

impl LuaSpace {
    pub fn lock(&self) -> MutexGuard<'_, Space> {
        self.0.lock()
    }

    pub fn get(lua: LuaContext<'_>) -> LuaResult<Self> {
        lua.globals().get("SPACE")
    }
    pub fn with<T, E: LuaExternalError>(
        lua: LuaContext<'_>,
        f: impl FnOnce(&mut Space) -> Result<T, E>,
    ) -> LuaResult<T> {
        f(&mut Self::get(lua)?.lock()).to_lua_err()
    }
}
