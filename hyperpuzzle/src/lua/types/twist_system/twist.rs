use std::sync::Arc;

use hypermath::Isometry;
use parking_lot::MutexGuard;

use crate::builder::{TwistBuilder, TwistSystemBuilder};
use crate::puzzle::Twist;

use super::*;

pub type LuaTwist = LuaDbEntry<Twist, TwistSystemBuilder>;

impl LuaUserData for LuaTwist {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("twist"));

        TwistSystemBuilder::add_named_db_entry_fields(fields);

        fields.add_field_method_get("axis", |_lua, this| {
            Ok(LuaAxis {
                id: this.get()?.axis,
                db: Arc::clone(&this.db.lock().axes),
            })
        });
        fields.add_field_method_get("transform", |_lua, this| {
            Ok(LuaTransform(this.get()?.transform))
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            if let Some(name) = this.db.lock().names.get(this.id) {
                Ok(format!("twist({name:?})"))
            } else {
                Ok(format!("twist({})", this.id))
            }
        });
    }
}

impl LuaTwist {
    /// Returns a copy of the twist builder.
    pub fn get(&self) -> LuaResult<TwistBuilder> {
        self.db.lock().get(self.id).into_lua_err().cloned()
    }

    pub fn transform(&self, t: &Isometry) -> LuaResult<Option<Self>> {
        let db = self.db.lock();

        let TwistBuilder { axis, transform } = db.get(self.id).into_lua_err()?;

        let axis = LuaAxis {
            id: *axis,
            db: Arc::clone(&db.axes),
        };
        let Some(transformed_axis) = axis.transform(t)? else {
            return Ok(None);
        };

        let transformed_transform = t.transform_isometry(transform); // TODO: maybe transform uninverted?

        let transformed_twist_data = TwistBuilder {
            axis: transformed_axis.id,
            transform: transformed_transform,
        };

        Ok(db
            .data_to_id()
            .get(&transformed_twist_data)
            .map(|&id| db.wrap_id(id)))
    }
}
