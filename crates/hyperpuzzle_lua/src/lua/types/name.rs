use super::*;
use crate::builder::NameSet;

/// Lua name object.
#[derive(Debug, Clone)]
pub struct LuaNameSet(pub NameSet);

impl FromLua for LuaNameSet {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        match &value {
            LuaValue::Integer(i) => Ok(Self(NameSet::from(i.to_string()))),
            LuaValue::Number(n) => Ok(Self(NameSet::from(n.to_string()))),
            LuaValue::String(s) => Ok(Self(NameSet::from(s.to_string_lossy()))),
            LuaValue::UserData(userdata) => {
                if let Ok(this) = userdata.borrow::<Self>() {
                    Ok(this.clone())
                } else if let Ok(this) = userdata.get("name") {
                    Ok(this)
                } else {
                    // easy way to get a good error message
                    cast_userdata(lua, &value)
                }
            }
            _ => cast_userdata(lua, &value), // easy way to get a good error message
        }
    }
}

impl LuaUserData for LuaNameSet {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("name"));

        fields.add_field_method_get("list", |_lua, this| {
            this.0.string_set().map_err(LuaError::external)
        });

        fields.add_field_method_get("canonical", |_lua, this| Ok(this.0.canonical_name()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Len, |_lua, this, ()| {
            Ok(this.0.name_set_len())
        });

        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            Ok(this.0.canonical_name())
        });

        methods.add_meta_function(
            LuaMetaMethod::Concat,
            |_lua, (LuaNameSet(lhs), LuaNameSet(rhs))| Ok(LuaNameSet(NameSet::new_seq([lhs, rhs]))),
        );
    }
}
