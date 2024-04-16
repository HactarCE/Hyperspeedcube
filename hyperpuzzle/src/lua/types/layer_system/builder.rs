use std::sync::Arc;

use hypermath::IndexNewtype;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};

use crate::builder::AxisLayerBuilder;
use crate::puzzle::{Layer, PerLayer};

use super::*;

#[derive(Debug, Clone)]
pub struct LuaLayerSystem {
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
            let space = this.space();
            let this = this.lock()?;
            let manifold_to_lua = |manifold| LuaManifold {
                manifold,
                space: Arc::clone(&space),
            };
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
            let expected_space = this.space();

            let LuaManifold { manifold, space } = bottom;
            if !Arc::ptr_eq(&expected_space, &space) {
                return Err(LuaError::external(
                    "cannot mix manifolds from different spaces",
                ));
            }
            let bottom = -manifold;

            let top = match top {
                Some(LuaManifold { manifold, space }) => {
                    if !Arc::ptr_eq(&expected_space, &space) {
                        return Err(LuaError::external(
                            "cannot mix manifolds from different spaces",
                        ));
                    }
                    Some(manifold)
                }
                None => None,
            };

            this.lock()?.push(AxisLayerBuilder { bottom, top });

            Ok(())
        });
    }
}

impl LuaLayerSystem {
    fn lock(&self) -> LuaResult<MappedMutexGuard<'_, PerLayer<AxisLayerBuilder>>> {
        MutexGuard::try_map(self.axis.db.lock(), |db| {
            Some(&mut db.get_mut(self.axis.id).ok()?.layers)
        })
        .map_err(|_| LuaError::external("error fetching layer system"))
    }

    fn space(&self) -> Arc<Mutex<Space>> {
        Arc::clone(&self.axis.db.lock().space)
    }
}
