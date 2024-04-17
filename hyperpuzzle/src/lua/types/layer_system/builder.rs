use std::sync::Arc;

use hypermath::IndexNewtype;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};

use super::*;
use crate::builder::AxisLayerBuilder;
use crate::puzzle::{Layer, PerLayer};

/// Lua handle to the layer system of an axis in an axis system.
#[derive(Debug, Clone)]
pub struct LuaLayerSystem {
    /// Axis.
    pub axis: LuaAxis,
}

impl LuaUserData for LuaLayerSystem {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("layersystem"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, move |_lua, this, ()| {
            Ok(this.axis.lua_into_string()? + ".layers")
        });

        methods.add_meta_method(LuaMetaMethod::Index, move |lua, this, LuaIndex(index)| {
            let space_mutex = this.space();
            let space = space_mutex.lock();
            let this = this.lock()?;
            let manifold_to_lua = |manifold| LuaManifold(space.blade_of(manifold));
            match this.get(Layer::try_from_usize(index).into_lua_err()?) {
                Ok(layer) => Ok(LuaMultiValue::from_vec(vec![
                    Some(manifold_to_lua(layer.bottom)).into_lua(lua)?,
                    layer.top.map(manifold_to_lua).into_lua(lua)?,
                ])),
                Err(_) => LuaNil.into_lua_multi(lua),
            }
        });
        methods.add_meta_method(LuaMetaMethod::Len, move |_lua, this, ()| {
            Ok(this.lock()?.len())
        });

        methods.add_method("add", |_lua, this, (bottom, top)| {
            let space_mutex = this.space();
            let mut space = space_mutex.lock();

            // Reverse the bottom manifold so that it faces up.
            let LuaManifold(bottom) = bottom;
            let bottom = space.add_manifold(-bottom).into_lua_err()?;

            // Leave the top manifold as-is.
            let top = match top {
                Some(LuaManifold(m)) => Some(space.add_manifold(m).into_lua_err()?),
                None => None,
            };

            this.lock()?
                .push(AxisLayerBuilder { bottom, top })
                .into_lua_err()?;

            Ok(())
        });
    }
}

impl LuaLayerSystem {
    /// Returns a mutex guard granting temporary access to the underlying layer
    /// list.
    pub fn lock(&self) -> LuaResult<MappedMutexGuard<'_, PerLayer<AxisLayerBuilder>>> {
        MutexGuard::try_map(self.axis.db.lock(), |db| {
            Some(&mut db.get_mut(self.axis.id).ok()?.layers)
        })
        .map_err(|_| LuaError::external("error fetching layer system"))
    }

    /// Returns the space in which the layer system is constructed.
    fn space(&self) -> Arc<Mutex<Space>> {
        Arc::clone(&self.axis.db.lock().space)
    }
}
