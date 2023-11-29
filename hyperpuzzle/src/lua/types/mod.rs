use rlua::prelude::*;

#[macro_use]
mod wrappers;
mod manifold;
mod multivector;
mod ndim;
mod pieceset;
mod puzzle;
mod space;
mod symmetry;
mod vector;

pub use manifold::LuaManifold;
pub use multivector::{LuaConstructMultivector, LuaMultivector};
pub use ndim::LuaNdim;
pub use pieceset::LuaPieceSet;
pub use puzzle::LuaPuzzleBuilder;
pub use space::LuaSpace;
pub use symmetry::LuaSymmetry;
pub use vector::{LuaConstructVector, LuaVector};
pub use wrappers::*;

pub fn lua_type_name(lua_value: &LuaValue<'_>) -> &'static str {
    if let LuaValue::UserData(userdata) = lua_value {
        macro_rules! return_name_if_type {
            ($userdata:ident, $wrapper_type:ty) => {
                if $userdata
                    .is::<LuaNamedUserData<<$wrapper_type as LuaUserDataConvertWrap>::Inner>>()
                {
                    return <$wrapper_type as LuaUserDataConvertWrap>::TYPE_NAME;
                }
            };
        }
        return_name_if_type!(userdata, LuaManifold);
        return_name_if_type!(userdata, LuaMultivector);
        return_name_if_type!(userdata, LuaPuzzleBuilder);
        return_name_if_type!(userdata, LuaPieceSet);
        return_name_if_type!(userdata, LuaSpace);
        return_name_if_type!(userdata, LuaSymmetry);
        return_name_if_type!(userdata, LuaVector);
    }
    lua_value.type_name()
}

#[derive(Debug, Clone)]
pub struct LuaLogLine {
    pub msg: String,
    pub file: String,
    pub level: String,
}
impl<'lua> From<LuaTable<'lua>> for LuaLogLine {
    fn from(value: LuaTable<'lua>) -> Self {
        LuaLogLine {
            msg: value.get("msg").unwrap_or_else(|_| "nil".to_string()),
            file: value.get("file").unwrap_or_else(|_| "?.lua".to_string()),
            level: value.get("level").unwrap_or_else(|_| "????".to_string()),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum LuaObjectLoadError {
    #[error("missing dependencies: {0:?}")]
    MissingDependencies(Vec<String>),
    #[error("error: {0}")]
    UserError(LuaError),
    #[error("internal error: {0}")]
    InternalError(LuaError),
}
