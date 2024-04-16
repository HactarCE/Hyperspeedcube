use std::{borrow::Cow, sync::Arc};

use parking_lot::{Mutex, MutexGuard};

use crate::builder::{AxisSystemBuilder, CustomOrdering, NamingScheme};
use crate::puzzle::Axis;

use super::*;

#[derive(Debug, Clone)]
pub struct LuaAxisSystem(pub Arc<Mutex<AxisSystemBuilder>>);

impl LuaUserData for LuaAxisSystem {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("axissystem"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        AxisSystemBuilder::add_db_metamethods(methods, |Self(shape)| shape.lock());
        AxisSystemBuilder::add_named_db_methods(methods, |Self(shape)| shape.lock());
        AxisSystemBuilder::add_ordered_db_methods(methods, |Self(shape)| shape.lock());

        methods.add_method("add", |lua, this, data| this.add(lua, data));

        methods.add_method_mut("autoname", |_lua, this, ()| {
            let autonames = crate::util::iter_uppercase_letter_names();
            let mut this = this.lock();
            let len = this.len();
            this.names.autoname(len, autonames).into_lua_err()
        })
    }
}

impl<'lua> LuaIdDatabase<'lua, Axis> for AxisSystemBuilder {
    const ELEMENT_NAME_SINGULAR: &'static str = "axis";
    const ELEMENT_NAME_PLURAL: &'static str = "axes";

    fn value_to_id(&self, lua: &'lua Lua, value: LuaValue<'lua>) -> LuaResult<Axis> {
        self.value_to_id_by_userdata(lua, &value)
            .or_else(|| self.value_to_id_by_name(lua, &value))
            .or_else(|| self.value_to_id_by_index(lua, &value))
            .or_else(|| {
                let LuaVector(v) = lua.unpack(value.clone()).ok()?;
                self.vector_to_id().get(&v).copied().map(Ok)
            })
            .unwrap_or_else(|| lua_convert_err(&value, "axis, string, or integer index"))
    }

    fn db_arc(&self) -> Arc<Mutex<Self>> {
        self.arc()
    }
    fn db_len(&self) -> usize {
        self.len()
    }
    fn ids_in_order(&self) -> Cow<'_, [Axis]> {
        Cow::Borrowed(self.ordering.ids_in_order())
    }
}
impl<'lua> LuaOrderedIdDatabase<'lua, Axis> for AxisSystemBuilder {
    fn ordering(&self) -> &CustomOrdering<Axis> {
        &self.ordering
    }
    fn ordering_mut(&mut self) -> &mut CustomOrdering<Axis> {
        &mut self.ordering
    }
}
impl<'lua> LuaNamedIdDatabase<'lua, Axis> for AxisSystemBuilder {
    fn names(&self) -> &NamingScheme<Axis> {
        &self.names
    }
    fn names_mut(&mut self) -> &mut NamingScheme<Axis> {
        &mut self.names
    }
}

impl LuaAxisSystem {
    fn lock(&self) -> MutexGuard<'_, AxisSystemBuilder> {
        self.0.lock()
    }

    fn add<'lua>(&self, lua: &'lua Lua, data: LuaValue<'lua>) -> LuaResult<LuaValue<'lua>> {
        let name: Option<String>;
        let vector: LuaVector;
        if let Ok(v) = lua.unpack(data.clone()) {
            name = None;
            vector = v;
        } else if let LuaValue::Table(t) = data {
            unpack_table!(lua.unpack(t { name, vector }));
        } else {
            return lua_convert_err(&data, "vector or table");
        };

        let LuaVector(vector) = vector;

        let mut this = self.lock();

        match &this.symmetry {
            Some(sym) => {
                if name.is_some() {
                    return Err(LuaError::external(
                        "`name` is invalid when symmetry-expanding vector",
                    ));
                }
                sym.expand(vector, |t, v| t.transform_vector(v))
                    .into_iter()
                    .enumerate()
                    .map(|(i, (_transform, v))| {
                        let id = this.add(v).into_lua_err()?;
                        Ok((i, this.wrap_id(id)))
                    })
                    .collect::<LuaResult<Vec<_>>>()
                    .and_then(|new_axes| lua.create_table_from(new_axes))
                    .map(LuaValue::Table)
            }
            None => {
                let id = this.add(vector).into_lua_err()?;
                this.names.set(id, name).into_lua_err()?;
                this.wrap_id(id).into_lua(lua)
            }
        }
    }
}
