use mlua::prelude::*;

#[macro_use]
mod wrappers;
mod axis_system;
mod color_system;
mod database_traits;
mod geometry;
mod layer_system;
mod orbit;
mod puzzle;
mod shape;
mod symmetry;
mod twist_system;

pub use axis_system::*;
pub use color_system::*;
pub use database_traits::*;
pub use geometry::*;
pub use layer_system::*;
pub use orbit::*;
pub use puzzle::*;
pub use shape::*;
pub use symmetry::*;
pub use twist_system::*;
pub use wrappers::*;

/// Type that has a user-friendly name for error messages.
pub trait LuaTypeName {
    /// Returns a user-friendly name for the type.
    fn type_name(lua: &Lua) -> LuaResult<&'static str>;
}
impl<T: 'static + LuaUserData> LuaTypeName for T {
    fn type_name(lua: &Lua) -> LuaResult<&'static str> {
        lua_userdata_type_name::<T>(lua)
    }
}

/// Casts userdata to `T` if it is the correct type; otherwise returns an error.
pub fn cast_userdata<T: 'static + LuaUserData + Clone>(
    lua: &Lua,
    value: &LuaValue<'_>,
) -> LuaResult<T> {
    match value.as_userdata().and_then(|d| d.borrow::<T>().ok()) {
        Some(value) => Ok(value.clone()),
        None => lua_convert_err(value, lua_userdata_type_name::<T>(lua)?),
    }
}

/// Lua wrapper around a `&'static str`.
///
/// This is useful for storing the type name for userdata.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) struct LuaStaticStr(&'static str);
impl LuaUserData for LuaStaticStr {}
impl<'lua> FromLua<'lua> for LuaStaticStr {
    fn from_lua(value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        value
            .as_userdata()
            .ok_or(LuaError::FromLuaConversionError {
                // Don't use our custom `lua_type_name()` because that could
                // potentially cause infinite recursion!
                from: value.type_name(),
                to: "Rust `&'static str`",
                message: None,
            })?
            .borrow()
            .map(|s| *s)
    }
}

/// Shortcut function to construct the obvious
/// [`LuaError::FromLuaConversionError`].
fn lua_convert_err<T>(value: &LuaValue<'_>, to: &'static str) -> Result<T, LuaError> {
    Err(lua_convert_error(value, to))
}
/// Shortcut function to construct the obvious
/// [`LuaError::FromLuaConversionError`].
fn lua_convert_error(value: &LuaValue<'_>, to: &'static str) -> LuaError {
    LuaError::FromLuaConversionError {
        from: lua_type_name(value),
        to,
        message: None,
    }
}

/// Returns the type name for a custom userdata type.
pub fn lua_userdata_type_name<'lua, T: 'static + LuaUserData>(
    lua: &'lua Lua,
) -> LuaResult<&'static str> {
    Ok(lua_type_name(&LuaValue::UserData(lua.create_proxy::<T>()?)))
}
/// Returns the name of a Lua type.
///
/// For built-in Lua types, this behaves the same as Lua's built-in `type()`
/// function.
///
/// For userdata types defined in this crate, the `"type"` metadata key is used
/// instead, which gives better information to users of the Lua API.
pub fn lua_type_name<'lua>(value: &LuaValue<'lua>) -> &'static str {
    // IIFE to mimic try_block
    match (|| {
        value
            .as_userdata()?
            .get_metatable()
            .ok()?
            .get("type")
            .ok()?
    })() {
        Some(LuaStaticStr(s)) => s,
        None => value.type_name(),
    }
}

/// Sets the metatable on `table` to make it read-only.
pub fn seal_table<'lua>(lua: &'lua Lua, table: &LuaTable<'lua>) -> LuaResult<()> {
    lua.globals().get::<_, LuaFunction<'_>>("seal")?.call(table)
}
