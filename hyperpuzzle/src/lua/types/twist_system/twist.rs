use std::sync::Arc;

use hypermath::pga::Motor;

use super::*;
use crate::builder::{PuzzleBuilder, TwistBuilder, TwistKey};
use crate::puzzle::Twist;

/// Lua handle to a twist in a twist system under construction.
pub type LuaTwist = LuaDbEntry<Twist, PuzzleBuilder>;

impl LuaUserData for LuaTwist {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("twist"));

        LuaNamedIdDatabase::<Twist>::add_named_db_entry_fields(fields);

        fields.add_field_method_get("axis", |_lua, this| this.axis());
        fields.add_field_method_get("transform", |_lua, this| {
            Ok(LuaTransform(this.get()?.transform))
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            if let Some(name) = this.db.lock().twists.names.get(this.id) {
                Ok(format!("twist({name:?})"))
            } else {
                Ok(format!("twist({})", this.id))
            }
        });

        methods.add_meta_method(LuaMetaMethod::Mul, |_lua, this, other: Self| {
            if !Arc::ptr_eq(&this.db, &other.db) {
                return Err(LuaError::external(
                    "cannot compose twists from different twist systems",
                ));
            }
            if this.id != other.id {
                return Err(LuaError::external(
                    "cannot compose twists with different axes",
                ));
            }
            let puz = this.db.lock();

            let lhs = puz.twists.get(this.id).into_lua_err()?;
            let rhs = puz.twists.get(other.id).into_lua_err()?;
            let combined_transform = &lhs.transform * &rhs.transform;
            let twist_key = TwistKey::new(lhs.axis, &combined_transform).into_lua_err()?;
            Ok(puz
                .twists
                .data_to_id(&twist_key)
                .map(|id| Self { id, db: puz.arc() }))
        });

        methods.add_meta_method(LuaMetaMethod::Pow, |lua, this, power: i16| {
            let ndim = LuaNdim::get(lua)?;

            let puz = this.db.lock();
            let this = puz.twists.get(this.id).into_lua_err()?;
            // Convert to `i64` to guard against overflow.
            let mut transform = (0..(power as i64).abs())
                .map(|_| &this.transform)
                .fold(Motor::ident(ndim), |a, b| b * a);
            if power < 0 {
                transform = transform.reverse();
            }
            Ok(puz
                .twists
                .data_to_id(&TwistKey::new(this.axis, &transform).into_lua_err()?)
                .map(|id| LuaTwist { id, db: puz.arc() }))
        });
    }
}

impl LuaTwist {
    /// Returns a copy of the twist builder.
    pub fn get(&self) -> LuaResult<TwistBuilder> {
        self.db.lock().twists.get(self.id).into_lua_err().cloned()
    }

    /// Returns the twist that contains an equivalent axis and transform to this
    /// twist, but transformed by `t`.
    pub fn transform_by(&self, m: &Motor) -> LuaResult<Option<Self>> {
        let db = self.db.lock();

        let TwistBuilder {
            axis, transform, ..
        } = db.twists.get(self.id).into_lua_err()?;

        let axis = db.wrap_id(*axis);
        let Some(transformed_axis) = axis.transform_by(m)? else {
            return Ok(None);
        };

        let transformed_transform = m.transform(transform); // TODO: maybe transform uninverted?

        Ok(db
            .twists
            .data_to_id(&TwistKey::new(transformed_axis.id, &transformed_transform).into_lua_err()?)
            .map(|id| db.wrap_id(id)))
    }

    /// Returns the axis of the twist.
    pub fn axis(&self) -> LuaResult<LuaAxis> {
        Ok(LuaAxis {
            id: self.get()?.axis,
            db: Arc::clone(&self.db),
        })
    }
}
