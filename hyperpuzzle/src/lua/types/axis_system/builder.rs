use std::borrow::Cow;
use std::sync::Arc;

use hypermath::Hyperplane;
use itertools::Itertools;
use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::builder::{AxisLayerBuilder, AxisSystemBuilder, CustomOrdering, NamingScheme};
use crate::lua::lua_warn_fn;
use crate::puzzle::Axis;

/// Lua handle for an axis system under construction.
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

        methods.add_method_mut("autoname", |lua, this, ()| {
            let autonames = crate::util::iter_uppercase_letter_names();
            let mut this = this.lock();
            let len = this.len();
            this.names.autoname(len, autonames, lua_warn_fn(lua));
            Ok(())
        });
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
                self.vector_to_id(v).map(Ok)
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
    /// Returns a mutex guard granting temporary access to the underlying
    /// [`AxisSystemBuilder`].
    pub fn lock(&self) -> MutexGuard<'_, AxisSystemBuilder> {
        self.0.lock()
    }

    /// Adds a new symmetric set of twist axes and returns a table containing
    /// them in sequence.
    pub fn add<'lua>(
        &self,
        lua: &'lua Lua,
        vectors: LuaSymmetricSet<LuaVector>,
        extra: Option<LuaTable<'lua>>,
    ) -> LuaResult<Vec<LuaAxis>> {
        let depths: Vec<hypermath::Float>;
        if let Some(t) = extra {
            let layers: LuaTable<'lua>;
            if t.len()? > 0 {
                layers = t;
            } else {
                unpack_table!(lua.unpack(t { layers }));
            }
            depths = layers.sequence_values().try_collect()?;
        } else {
            depths = vec![];
        }

        // Check that layers are monotonic. This check happens later too, but we
        // can give a better error here with a nice message and line number.
        // This is an especially easy mistake to make, so it's important to have
        // a good error message for it.
        for (a, b) in depths.iter().tuple_windows() {
            if a < b {
                return Err(LuaError::external(
                    "layers must be sorted from shallowest to deepest",
                ));
            }
        }

        let mut this = self.lock();
        let mut new_ids = vec![];
        for (name, LuaVector(v)) in vectors.to_vec(lua)? {
            let id = this.add(v.clone()).into_lua_err()?;
            this.names.set(id, name, lua_warn_fn(lua));
            new_ids.push(this.wrap_id(id));

            let axis = this.get_mut(id).into_lua_err()?;
            for &depth in &depths {
                let layer_plane = Hyperplane::new(&v, depth)
                    .ok_or("axis vector cannot be zero")
                    .into_lua_err()?;
                // Flip the bottom plane so that it faces up.
                let layer = AxisLayerBuilder {
                    bottom: layer_plane.flip(),
                    top: None,
                };
                axis.layers.push(layer).into_lua_err()?;
            }
        }
        Ok(new_ids)
    }
}
