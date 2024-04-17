use std::{borrow::Cow, sync::Arc};

use parking_lot::Mutex;

use crate::builder::{NamingScheme, TwistBuilder, TwistSystemBuilder};
use crate::puzzle::Twist;

use super::*;

#[derive(Debug, Clone)]
pub struct LuaTwistSystem(pub Arc<Mutex<TwistSystemBuilder>>);

impl LuaUserData for LuaTwistSystem {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("twistsystem"));

        fields.add_field_method_get("axes", |_lua, this| {
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

        methods.add_method("add", |lua, this, data| this.add(lua, data));
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

impl LuaTwistSystem {
    fn add<'lua>(&self, lua: &'lua Lua, data: LuaTable<'lua>) -> LuaResult<LuaTwist> {
        let axis_prefix: Option<bool>;
        let name: Option<String>;
        let inv_name: Option<String>;
        let inverse: Option<bool>;
        let axis: LuaAxis;
        let transform: LuaTransform;

        unpack_table!(lua.unpack(data {
            axis_prefix,
            name,
            inv_name,
            inverse,
            axis,
            transform,
        }));

        let mut name = name.unwrap_or_default();
        if axis_prefix.unwrap_or(true) {
            match axis.name() {
                Some(axis_name) => name.insert_str(0, &axis_name),
                None => {
                    lua.warning("cannot name twist without having named axis", true);
                    lua.warning("consider calling axes:autoname()", true);
                    lua.warning("or use axis_prefix=false", false);
                }
            }
        }

        let mut twists = self.0.lock();
        let id = twists
            .add(TwistBuilder {
                axis: axis.id,
                transform: transform.0,
            })
            .into_lua_err()?;
        twists.names.set(id, Some(name)).into_lua_err()?;

        Ok(twists.wrap_id(id))
    }
}
