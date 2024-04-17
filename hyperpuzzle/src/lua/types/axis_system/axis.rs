use hypermath::{Isometry, Vector};

use super::*;
use crate::builder::AxisSystemBuilder;
use crate::puzzle::Axis;

/// Lua handle for a twist axis in an axis system under construction.
pub type LuaAxis = LuaDbEntry<Axis, AxisSystemBuilder>;

impl LuaUserData for LuaAxis {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("axis"));

        AxisSystemBuilder::add_named_db_entry_fields(fields);
        AxisSystemBuilder::add_ordered_db_entry_fields(fields);

        fields.add_field_method_get("vector", |_lua, this| {
            let db = this.db.lock();
            let v = db.get(this.id).into_lua_err()?.vector();
            Ok(LuaVector(v.clone()))
        });

        fields.add_field_method_get("layers", |_lua, this| {
            Ok(LuaLayerSystem { axis: this.clone() })
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            this.lua_into_string()
        });
    }
}

impl LuaAxis {
    /// Returns the vector of the axis.
    pub fn vector(&self) -> LuaResult<Vector> {
        Ok(self.db.lock().get(self.id).into_lua_err()?.vector().clone())
    }
    /// Returns the name of the axis, or `None` if one has not been assigned.
    pub fn name(&self) -> Option<String> {
        self.db.lock().names.get(self.id)
    }

    /// Returns the expected result of calling the Lua `tostring` function with
    /// `self`.
    pub fn lua_into_string(&self) -> LuaResult<String> {
        if let Some(name) = self.name() {
            Ok(format!("axis({name:?}, vector={})", self.vector()?))
        } else {
            Ok(format!("axis({})", self.id))
        }
    }

    /// Returns the axis that has an equivalent vector to this one, but
    /// transformed by `t`.
    pub fn transform(&self, t: &Isometry) -> LuaResult<Option<Self>> {
        let v = t.transform_vector(self.vector()?);
        Ok(self.db.lock().vector_to_id().get(&v).map(|&id| {
            let db = self.db.clone();
            Self { id, db }
        }))
    }
}
