use std::{borrow::Cow, sync::Arc};

use parking_lot::Mutex;

use crate::builder::{NamingScheme, TwistSystemBuilder};
use crate::puzzle::Twist;

use super::*;

#[derive(Debug, Clone)]
pub struct LuaTwistSystem(pub Arc<Mutex<TwistSystemBuilder>>);

impl LuaUserData for LuaTwistSystem {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("twistsystem"));

        fields.add_field_method_get("axes", |lua, this| {
            Ok(LuaAxisSystem(Arc::clone(&this.0.lock().axes)))
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            let this = this.lock();
            let len = this.len();
            if let Some(id) = &this.id {
                Ok(format!("twistsystem({id:?}, len={len})"))
            } else {
                Ok(format!("twistsystem(len={len})"))
            }
        });

        TwistSystemBuilder::add_db_metamethods(methods, |Self(shape)| shape.lock());
        TwistSystemBuilder::add_named_db_methods(methods, |Self(shape)| shape.lock());
    }
}

impl<'lua> LuaIdDatabase<'lua, Twist> for TwistSystemBuilder {
    const ELEMENT_NAME_SINGULAR: &'static str = "twist";
    const ELEMENT_NAME_PLURAL: &'static str = "twists";

    fn value_to_id(&self, lua: &'lua Lua, value: LuaValue<'lua>) -> LuaResult<Twist> {
        // TODO: lookup by axis vector
        self.value_to_id_by_userdata(lua, &value)
            .or_else(|| self.value_to_id_by_name(lua, &value))
            .unwrap_or_else(|| lua_convert_err(&value, "axis, string, or integer index"))
    }

    fn db_arc(&self) -> Arc<Mutex<Self>> {
        self.arc()
    }
    fn db_len(&self) -> usize {
        self.len()
    }
    fn ids_in_order(&self) -> Cow<'_, [Twist]> {
        Cow::Owned(self.alphabetized())
    }
}

impl<'lua> LuaNamedIdDatabase<'lua, Twist> for TwistSystemBuilder {
    fn names(&self) -> &NamingScheme<Twist> {
        &self.names
    }
    fn names_mut(&mut self) -> &mut NamingScheme<Twist> {
        &mut self.names
    }
}
