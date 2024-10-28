use std::sync::Arc;

use hypershape::prelude::*;

use super::*;

/// Lua handle to a space.
#[derive(Debug, Clone)]
pub struct LuaSpace(pub Arc<Space>);

impl FromLua for LuaSpace {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaSpace {
    /// Returns the global space, given a Lua instance.
    pub fn get(lua: &Lua) -> LuaResult<Self> {
        lua.globals().get("SPACE").context("no global space")
    }

    /// Locks the global space and executes `f` with it.
    pub fn with<R>(lua: &Lua, f: impl FnOnce(&Arc<Space>) -> LuaResult<R>) -> LuaResult<R> {
        f(&Self::get(lua)?.0).into_lua_err()
    }

    /// Sets a space to be the global space, executes `f`, and then restores the
    /// global space to its prior value.
    pub fn with_this_as_global_space<R>(
        &self,
        lua: &Lua,
        f: impl FnOnce() -> LuaResult<R>,
    ) -> LuaResult<R> {
        let old_space: LuaValue = lua.globals().get("SPACE")?;
        let old_ndim: LuaValue = lua.globals().get("NDIM")?;
        lua.globals().set("SPACE", self.clone())?;
        lua.globals().set("NDIM", self.0.ndim())?;
        let result = f();
        lua.globals().set("SPACE", old_space)?;
        lua.globals().set("NDIM", old_ndim)?;
        result
    }
}

impl LuaUserData for LuaSpace {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("space"));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            Ok(format!("space(ndim = {})", this.ndim()))
        });

        methods.add_method("ndim", |_lua, Self(this), ()| Ok(this.ndim()));
    }
}
