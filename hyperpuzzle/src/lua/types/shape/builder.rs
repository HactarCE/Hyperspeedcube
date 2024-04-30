use std::sync::Arc;

use hypermath::{ApproxHashMapKey, TransformByMotor};
use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::builder::ShapeBuilder;

/// Lua handle to a shape under construction.
#[derive(Debug, Clone)]
pub struct LuaShape(pub Arc<Mutex<ShapeBuilder>>);

impl<'lua> FromLua<'lua> for LuaShape {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaUserData for LuaShape {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("shape"));

        fields.add_field_method_get("id", |_lua, this| Ok(this.lock().id.clone()));
        fields.add_field_method_get("space", |_lua, this| {
            Ok(LuaSpace(Arc::clone(&this.lock().space)))
        });
        fields.add_field_method_get("ndim", |_lua, this| Ok(this.lock().ndim()));
        fields.add_field_method_get("colors", |_lua, Self(this)| {
            Ok(LuaColorSystem(Arc::clone(this)))
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            let this = this.lock();
            let ndim = this.ndim();
            if let Some(id) = &this.id {
                Ok(format!("shape({id:?}, ndim={ndim})"))
            } else {
                Ok(format!("shape(ndim={ndim})"))
            }
        });

        methods.add_method("carve", |_lua, this, LuaHyperplane(hyperplane)| {
            let cuts = this.orbit(hyperplane)?;
            let mut this = this.lock();
            for cut in cuts {
                this.carve(None, cut.clone()).into_lua_err()?;
                this.colors.add(vec![cut]).into_lua_err()?;
            }
            Ok(())
        });
        methods.add_method(
            "carve_unstickered",
            |_lua, this, LuaHyperplane(hyperplane)| {
                let cuts = this.orbit(hyperplane)?;
                let mut this = this.lock();
                for cut in cuts {
                    this.carve(None, cut).into_lua_err()?;
                }
                Ok(())
            },
        );
        methods.add_method("slice", |_lua, this, LuaHyperplane(hyperplane)| {
            let cuts = this.orbit(hyperplane)?;
            let mut this = this.lock();
            for cut in cuts {
                this.slice(None, cut).into_lua_err()?;
            }
            Ok(())
        });
    }
}

impl LuaShape {
    /// Returns a mutex guard granting temporary access to the underlying
    /// [`ShapeBuilder`].
    pub fn lock(&self) -> MutexGuard<'_, ShapeBuilder> {
        self.0.lock()
    }

    /// Returns a list of the elements in the orbit of `obj` under the shape's
    /// symmetry.
    fn orbit<T: ApproxHashMapKey + TransformByMotor + Clone>(&self, obj: T) -> LuaResult<Vec<T>> {
        match &self.lock().symmetry {
            Some(sym) => Ok(sym
                .orbit(obj, false)
                .into_iter()
                .map(|(_transform, obj)| obj)
                .collect()),
            None => Ok(vec![obj]),
        }
    }
}
