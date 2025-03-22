use std::sync::Arc;

use float_ord::FloatOrd;
use hypermath::{Hyperplane, VectorRef};
use hyperpuzzle_core::prelude::*;
use itertools::Itertools;
use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::builder::{AxisLayerBuilder, NamingScheme, PuzzleBuilder};
use crate::lua::lua_warn_fn;

/// Lua handle for an axis system under construction.
#[derive(Debug, Clone)]
pub struct LuaAxisSystem(pub Arc<Mutex<PuzzleBuilder>>);

impl LuaUserData for LuaAxisSystem {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("axissystem"));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        LuaIdDatabase::<Axis>::add_db_metamethods(methods, |Self(puz)| puz);
        LuaNamedIdDatabase::<Axis>::add_named_db_methods(methods, |Self(puz)| puz);

        methods.add_method("autoname", |lua, this, ()| {
            let autonames = hyperpuzzle_core::util::iter_uppercase_letter_names();
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

impl LuaIdDatabase<Axis> for PuzzleBuilder {
    const ELEMENT_NAME_SINGULAR: &'static str = "axis";
    const ELEMENT_NAME_PLURAL: &'static str = "axes";

    fn value_to_id(&self, lua: &Lua, value: LuaValue) -> LuaResult<Axis> {
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
}

impl LuaNamedIdDatabase<Axis> for PuzzleBuilder {
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
    pub fn add(
        &self,
        lua: &Lua,
        vectors: LuaSymmetricSet<LuaVector>,
        extra: Option<LuaTable>,
    ) -> LuaResult<LuaSymmetricSet<LuaAxis>> {
        let depths: Vec<hypermath::Float>;
        let slice: Option<bool>;
        if let Some(t) = extra {
            let layers: LuaTable;
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
        let mut sorted_depths = depths.clone();
        sorted_depths.sort_by_key(|d| FloatOrd(-d));
        if depths != sorted_depths {
            lua.warning("layers should be sorted from shallowest to deepest", false);
        }
        let depths = sorted_depths;

        let mut gen_seqs = vec![];
        let mut new_axes = vec![];
        let ret = vectors.map(lua, |gen_seq, name, LuaVector(v)| {
            let v = v
                .normalize()
                .ok_or_else(|| LuaError::external("axis vector cannot be zero"))?;

            gen_seqs.push(gen_seq);

            let mut puz = self.lock();

            let id = puz.twists.axes.add(v.clone()).into_lua_err()?;
            puz.twists.axes.names.set_name(id, name, lua_warn_fn(lua));
            let new_axis = puz.wrap_id(id);
            new_axes.push(new_axis.clone());

            let axis = puz.twists.axes.get_mut(id).into_lua_err()?;
            for (&top, &bottom) in depths.iter().tuple_windows() {
                let layer = AxisLayerBuilder { bottom, top };
                axis.layers.push(layer).into_lua_err()?;
            }
            if slice {
                // Do the shallowest cut first to optimize piece ID usage.
                for &depth in depths
                    .iter()
                    .filter(|d| d.is_finite())
                    .sorted_by_key(|d| FloatOrd(-d.abs()))
                {
                    let cut_plane = Hyperplane::new(&v, depth)
                        .ok_or("axis vector cannot be zero")
                        .into_lua_err()?;
                    puz.shape
                        .slice(None, cut_plane, None)
                        .map_err(|e| LuaError::external(format!("{e:#}")))?;
                }
            }

            Ok(new_axis)
        })?;

        self.lock().twists.axes.axis_orbits.push(DevOrbit {
            kind: "axes",
            elements: new_axes.iter().map(|ax| Some(ax.id)).collect(),
            generator_sequences: gen_seqs,
        });

        Ok(ret)
    }
}
