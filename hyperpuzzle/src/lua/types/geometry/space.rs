use std::sync::Arc;

use hypershape::prelude::*;
use parking_lot::{Mutex, MutexGuard};

use super::*;

/// Lua handle to a space.
#[derive(Debug, Clone)]
pub struct LuaSpace(pub Arc<Mutex<Space>>);

impl<'lua> FromLua<'lua> for LuaSpace {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaSpace {
    /// Returns a mutex guard granting temporary access to the underlying
    /// [`Space`].
    pub fn lock(&self) -> MutexGuard<'_, Space> {
        self.0.lock()
    }

    /// Returns the global space, given a Lua instance.
    pub fn get(lua: &Lua) -> LuaResult<Self> {
        lua.globals().get("SPACE").context("no global space")
    }

    /// Locks the global space and executes `f` with it.
    pub fn with<R>(lua: &Lua, f: impl FnOnce(&mut Space) -> LuaResult<R>) -> LuaResult<R> {
        f(&mut Self::get(lua)?.lock()).into_lua_err()
    }

    /// Sets a space to be the global space, executes `f`, and then restores the
    /// global space to its prior value.
    pub fn with_this_as_global_space<R>(
        &self,
        lua: &Lua,
        f: impl FnOnce() -> LuaResult<R>,
    ) -> LuaResult<R> {
        let old_space: LuaValue<'_> = lua.globals().get("SPACE")?;
        let old_ndim: LuaValue<'_> = lua.globals().get("NDIM")?;
        lua.globals().set("SPACE", self.clone())?;
        lua.globals().set("NDIM", self.0.lock().ndim())?;
        let result = f();
        lua.globals().set("SPACE", old_space)?;
        lua.globals().set("NDIM", old_ndim)?;
        result
    }
}

impl LuaUserData for LuaSpace {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("space"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            Ok(format!("space(ndim = {})", this.lock().ndim()))
        });

        methods.add_method("ndim", |_lua, Self(this), ()| Ok(this.lock().ndim()));
    }
}
