use rlua::prelude::*;

#[macro_use]
mod wrappers;
mod manifold;
mod multivector;
mod shapeset;
mod space;
mod transform;
mod vector;

pub use multivector::lua_construct_multivector;
pub use shapeset::LuaShapeSet;
pub use space::LuaSpace;
pub use vector::lua_construct_vector;
pub use wrappers::*;

pub fn lua_type_name(lua_value: &LuaValue<'_>) -> &'static str {
    if let LuaValue::UserData(userdata) = lua_value {
        if userdata.is::<crate::math::Vector>() {
            return "vector";
        }
        if userdata.is::<crate::math::cga::Multivector>() {
            return "multivector";
        }
        if userdata.is::<crate::math::cga::Blade>() {
            return "manifold";
        }
        if userdata.is::<crate::math::cga::Isometry>() {
            return "transform";
        }
    }
    lua_value.type_name()
}

#[derive(Debug, Clone)]
pub struct LuaLogLine {
    pub msg: String,
    pub file: String,
    pub level: String,
}
impl<'lua> TryFrom<LuaTable<'lua>> for LuaLogLine {
    type Error = LuaError;

    fn try_from(value: LuaTable<'lua>) -> std::result::Result<Self, Self::Error> {
        Ok(LuaLogLine {
            msg: value.get("msg")?,
            file: value.get("file")?,
            level: value.get("level")?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum LuaFileLoadError {
    MissingDependencies(Vec<String>),
    UserError(LuaError),
    InternalError(LuaError),
}
