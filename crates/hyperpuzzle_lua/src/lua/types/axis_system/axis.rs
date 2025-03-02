use std::str::FromStr;

use hypermath::Vector;
use hypermath::pga::Motor;
use hyperpuzzle_core::{Axis, LayerMask, LayerMaskUint};
use parking_lot::{MappedMutexGuard, MutexGuard};

use super::*;
use crate::builder::{AxisBuilder, NameSet, PuzzleBuilder};

/// Lua handle for a twist axis in an axis system under construction.
pub type LuaAxis = LuaDbEntry<Axis, PuzzleBuilder>;

impl LuaUserData for LuaAxis {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("axis"));

        LuaNamedIdDatabase::add_named_db_entry_fields(fields);
        LuaOrderedIdDatabase::add_ordered_db_entry_fields(fields);

        fields.add_field_method_get("vector", |_lua, this| {
            let puz = this.db.lock();
            let axes = &puz.twists.axes;
            let v = axes.get(this.id).into_lua_err()?.vector();
            Ok(LuaVector(v.clone()))
        });

        fields.add_field_method_get("layers", |_lua, this| {
            Ok(LuaLayerSystem { axis: this.clone() })
        });

        fields.add_field_method_get("opposite", |_lua, this| {
            let puz = this.db.lock();
            let axes = &puz.twists.axes;
            let v = this.vector()?;
            Ok(axes.vector_to_id(-v).map(|id| Self {
                db: this.db.clone(),
                id,
            }))
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        LuaNamedIdDatabase::add_named_db_entry_methods(methods);

        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            this.lua_into_string()
        });

        methods.add_meta_method(LuaMetaMethod::Call, |lua, this, args: LuaMultiValue| {
            let layer_count = this.layers().len()?;
            let validate_layer = |LuaIndex(n)| {
                if n < LayerMaskUint::BITS as usize {
                    Ok(n as u8)
                } else {
                    Err(LuaError::external("layer index out of range"))
                }
            };

            let layer_mask = if args.len() == 1 {
                let arg: LuaValue = lua.unpack_multi(args)?;
                if let LuaValue::Table(t) = arg {
                    // list of individual layers
                    let mut ret = LayerMask::EMPTY;
                    for v in t.sequence_values() {
                        ret |= LayerMask::from(validate_layer(lua.unpack(v?)?)?);
                    }
                    ret
                } else if let Ok(n) = lua.unpack::<LuaIndex>(arg.clone()) {
                    // single layer
                    LayerMask::from(validate_layer(n)?)
                } else if let Ok(s) = lua.unpack::<String>(arg.clone()) {
                    if s.trim() == "*" {
                        // all layers
                        LayerMask::all_layers(layer_count as u8)
                    } else {
                        // layer mask string
                        LayerMask::from_str(&s).into_lua_err()?
                    }
                } else {
                    return lua_convert_err(
                        &arg,
                        "layer mask string, layer index number, \
                         or list of layer index numbers",
                    );
                }
            } else {
                // layer range
                let (lo, hi) = lua.unpack_multi(args)?;
                LayerMask::from(validate_layer(lo)?..=validate_layer(hi)?)
            };

            let puz = this.db.lock();
            match puz
                .twists
                .axes
                .get(this.id)
                .into_lua_err()?
                .plane_bounded_regions(layer_mask)
            {
                Ok(plane_bounded_regions) => Ok(LuaRegion::Or(
                    plane_bounded_regions
                        .into_iter()
                        .map(|layer_region| {
                            let half_spaces = layer_region.into_iter().map(LuaRegion::HalfSpace);
                            LuaRegion::And(half_spaces.collect())
                        })
                        .collect(),
                )),
                Err(e) => {
                    lua.warning(format!("error computing region: {e}"), false);
                    Ok(LuaRegion::Nothing)
                }
            }
        });

        methods.add_meta_method(LuaMetaMethod::Eq, |_lua, lhs, rhs: Self| Ok(lhs == &rhs));
    }
}

impl LuaAxis {
    /// Returns a mutex guard granting temporary access to the underlying axis.
    pub fn lock(&self) -> LuaResult<MappedMutexGuard<'_, AxisBuilder>> {
        MutexGuard::try_map(self.db.lock(), |puz| puz.twists.axes.get_mut(self.id).ok())
            .map_err(|_| LuaError::external("error fetching axis"))
    }

    /// Returns the vector of the axis.
    pub fn vector(&self) -> LuaResult<Vector> {
        let puz = self.db.lock();
        let axes = &puz.twists.axes;
        Ok(axes.get(self.id).into_lua_err()?.vector().clone())
    }
    /// Returns the name of the axis, or `None` if one has not been assigned.
    pub fn name(&self) -> Option<NameSet> {
        let puz = self.db.lock();
        let axes = &puz.twists.axes;
        axes.names.get(self.id).cloned()
    }
    /// Returns the layer system of the axis.
    pub fn layers(&self) -> LuaLayerSystem {
        LuaLayerSystem { axis: self.clone() }
    }

    /// Returns the expected result of calling the Lua `tostring` function with
    /// `self`.
    pub fn lua_into_string(&self) -> LuaResult<String> {
        if let Some(name) = self.name() {
            Ok(format!("axis({name:?})"))
        } else {
            Ok(format!("axis({})", self.id))
        }
    }

    /// Returns the axis that has an equivalent vector to this one, but
    /// transformed by `t`, or returns `None` if one does not exist.
    pub fn transform_by(&self, m: &Motor) -> LuaResult<Option<Self>> {
        let puz = self.db.lock();
        let v = m.transform_vector(self.vector()?);
        Ok(puz.twists.axes.vector_to_id(v).map(|id| puz.wrap_id(id)))
    }
}
