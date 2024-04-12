use std::sync::Arc;

use hypershape::{ManifoldRef, ManifoldSet};
use parking_lot::{Mutex, MutexGuard};

use crate::builder::ShapeBuilder;

use super::*;

#[derive(Debug, Clone)]
pub struct LuaShape(pub Arc<Mutex<ShapeBuilder>>);

impl<'lua> FromLua<'lua> for LuaShape {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaShape {
    fn lock(&self) -> MutexGuard<'_, ShapeBuilder> {
        self.0.lock()
    }

    fn symmetry_expand_manifold(&self, manifold: ManifoldRef) -> LuaResult<Vec<ManifoldRef>> {
        let shape = self.lock();
        let mut space = shape.space.lock();
        match &shape.symmetry {
            Some(sym) => sym
                .expand(space.blade_of(manifold), |t, b| t.transform_blade(b))
                .into_iter()
                .map(|(_transform, blade)| space.add_manifold(blade).into_lua_err())
                .collect(),
            None => Ok(vec![manifold]),
        }
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
        methods.add_method("carve", |lua, this, LuaManifold { manifold, .. }| {
            let cuts = this.symmetry_expand_manifold(manifold)?;
            let mut this = this.lock();
            for cut in cuts {
                this.carve(None, dbg!(cut)).into_lua_err()?;
                this.colors
                    .add(ManifoldSet::from_iter([cut]))
                    .into_lua_err()?;
            }
            Ok(())
        });
        methods.add_method(
            "carve_unstickered",
            |lua, this, LuaManifold { manifold, .. }| {
                let cuts = this.symmetry_expand_manifold(manifold)?;
                let mut this = this.lock();
                for cut in cuts {
                    this.carve(None, cut).into_lua_err()?;
                }
                Ok(())
            },
        );
        methods.add_method("slice", |lua, this, LuaManifold { manifold, .. }| {
            let cuts = this.symmetry_expand_manifold(manifold)?;
            let mut this = this.lock();
            for cut in cuts {
                this.slice(None, cut).into_lua_err()?;
            }
            Ok(())
        });
    }
}
