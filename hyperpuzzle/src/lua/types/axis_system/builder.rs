use std::borrow::Cow;
use std::sync::Arc;

use hypermath::Hyperplane;
use itertools::Itertools;
use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::builder::{AxisLayerBuilder, CustomOrdering, NamingScheme, PuzzleBuilder};
use crate::lua::lua_warn_fn;
use crate::puzzle::Axis;

/// Lua handle for an axis system under construction.
#[derive(Debug, Clone)]
pub struct LuaAxisSystem(pub Arc<Mutex<PuzzleBuilder>>);

impl LuaUserData for LuaAxisSystem {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("axissystem"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        LuaIdDatabase::<Axis>::add_db_metamethods(methods, |Self(puz)| puz.lock());
        LuaNamedIdDatabase::<Axis>::add_named_db_methods(methods, |Self(puz)| puz.lock());
        LuaOrderedIdDatabase::<Axis>::add_ordered_db_methods(methods, |Self(puz)| puz.lock());

        methods.add_method("autoname", |lua, this, ()| {
            let autonames = crate::util::iter_uppercase_letter_names();
            let mut puz = this.lock();
            let axes = &mut puz.twists.axes;
            let len = axes.len();
            axes.names.autoname(len, autonames, lua_warn_fn(lua));
            Ok(())
        });

        methods.add_method("add", |lua, this, (vectors, extra)| {
            this.add(lua, vectors, extra)
        });
    }
}

impl<'lua> LuaIdDatabase<'lua, Axis> for PuzzleBuilder {
    const ELEMENT_NAME_SINGULAR: &'static str = "axis";
    const ELEMENT_NAME_PLURAL: &'static str = "axes";

    fn value_to_id(&self, lua: &'lua Lua, value: LuaValue<'lua>) -> LuaResult<Axis> {
        self.value_to_id_by_userdata(lua, &value)
            .or_else(|| self.value_to_id_by_name(lua, &value))
            .or_else(|| self.value_to_id_by_index(lua, &value))
            .or_else(|| {
                let LuaVector(v) = lua.unpack(value.clone()).ok()?;
                self.twists.axes.vector_to_id(v).map(Ok)
            })
            .unwrap_or_else(|| lua_convert_err(&value, "axis, string, or integer index"))
    }

    fn db_arc(&self) -> Arc<Mutex<Self>> {
        self.arc()
    }
    fn db_len(&self) -> usize {
        self.twists.axes.len()
    }
    fn ids_in_order(&self) -> Cow<'_, [Axis]> {
        Cow::Borrowed(self.twists.axes.ordering.ids_in_order())
    }
}
impl<'lua> LuaOrderedIdDatabase<'lua, Axis> for PuzzleBuilder {
    fn ordering(&self) -> &CustomOrdering<Axis> {
        &self.twists.axes.ordering
    }
    fn ordering_mut(&mut self) -> &mut CustomOrdering<Axis> {
        &mut self.twists.axes.ordering
    }
}
impl<'lua> LuaNamedIdDatabase<'lua, Axis> for PuzzleBuilder {
    fn names(&self) -> &NamingScheme<Axis> {
        &self.twists.axes.names
    }
    fn names_mut(&mut self) -> &mut NamingScheme<Axis> {
        &mut self.twists.axes.names
    }
}

impl LuaAxisSystem {
    /// Returns a mutex guard granting temporary access to the underlying
    /// [`PuzzleBuilder`].
    pub fn lock(&self) -> MutexGuard<'_, PuzzleBuilder> {
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
        let slice: Option<bool>;
        if let Some(t) = extra {
            let layers: LuaTable<'lua>;
            if t.len()? > 0 || t.is_empty() {
                slice = t.get("slice")?;
                layers = t;
            } else {
                unpack_table!(lua.unpack(t { layers, slice }));
            }
            depths = layers.sequence_values().try_collect()?;
        } else {
            depths = vec![];
            slice = None;
        }

        let slice = slice.unwrap_or(true);

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

        let mut puz = self.lock();
        let mut new_axes = vec![];
        for (name, LuaVector(v)) in vectors.to_vec(lua)? {
            let id = puz.twists.axes.add(v.clone()).into_lua_err()?;
            puz.twists.axes.names.set_name(id, name, lua_warn_fn(lua));
            new_axes.push(puz.wrap_id(id));

            let axis = puz.twists.axes.get_mut(id).into_lua_err()?;
            let mut layer_planes = vec![];
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
                layer_planes.push(layer_plane);
            }
            if slice {
                for cut in layer_planes {
                    puz.shape.slice(None, cut, None, None).into_lua_err()?;
                }
            }
        }

        Ok(new_axes)
    }
}
