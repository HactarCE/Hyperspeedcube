use hypermath::{Hyperplane, IndexNewtype};
use parking_lot::MappedMutexGuard;

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
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("layersystem"));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, move |_lua, this, ()| {
            Ok(this.axis.lua_into_string()? + ".layers")
        });

        methods.add_meta_method(LuaMetaMethod::Index, move |lua, this, LuaIndex(index)| {
            let this = this.lock()?;
            match this.get(Layer::try_from_usize(index).into_lua_err()?) {
                Ok(layer) => {
                    let bottom = Some(LuaHyperplane(layer.bottom.clone())).into_lua(lua)?;
                    let top = layer.top.clone().map(LuaHyperplane).into_lua(lua)?;
                    lua.create_table_from([("bottom", bottom), ("top", top)])?
                        .into_lua(lua)
                }
                Err(_) => Ok(LuaNil),
            }
        });
        methods.add_meta_method(LuaMetaMethod::Len, move |_lua, this, ()| {
            Ok(this.lock()?.len())
        });

        methods.add_method("add", |_lua, this, (bottom, top)| {
            // Flip the bottom plane so that it faces up.
            let LuaHyperplane(bottom) = bottom;
            let bottom = bottom.flip();

            // Leave the top plane as-is.
            let top: Option<LuaHyperplane> = top;
            let top = top.map(|LuaHyperplane(m)| m);

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
        Ok(MappedMutexGuard::map(self.axis.lock()?, |axis| {
            &mut axis.layers
        }))
    }

    /// Returns all cuts in the layer.
    pub fn cuts(&self) -> LuaResult<Vec<Hyperplane>> {
        Ok(self
            .lock()?
            .iter_values()
            .flat_map(|layer| itertools::chain([layer.bottom.clone()], layer.top.clone()))
            .collect())
    }

    /// Returns whether there are no layers in the layer system.
    pub fn is_empty(&self) -> LuaResult<bool> {
        Ok(self.lock()?.is_empty())
    }
    /// Returns the number of layers.
    pub fn len(&self) -> LuaResult<usize> {
        Ok(self.lock()?.len())
    }
}
