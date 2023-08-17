use super::*;
use crate::{Object, ObjectData};

impl<'lua> FromLua<'lua> for Object {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        let t = LuaTable::from_lua(lua_value, lua)?;

        let name: String = t.get("name")?;
        let id: String = t.get("id")?;

        let data = match t.get::<_, String>("type")?.as_str() {
            "puzzle" => ObjectData::Puzzle {
                ndim: t
                    .get("ndim")
                    .map_err(|e| LuaError::external(format!("{id:?} has bad `ndim`: {e}")))?,
            },
            type_string => {
                return Err(LuaError::external(format!(
                    "invalid object type: {type_string:?}"
                )));
            }
        };

        Ok(Object { name, id, data })
    }
}
