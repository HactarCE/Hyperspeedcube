// TODO: this doesn't correctly handle missing elements (e.g., mising one axis
// but the others should still work)

use std::hash::Hash;
use std::sync::Arc;

use hypermath::collections::approx_hashmap::{
    ApproxHashMapKey, FloatHash, MultivectorHash, VectorHash,
};
use hypermath::{Blade, Float, Isometry, Vector};
use itertools::Itertools;
use parking_lot::Mutex;
use tinyset::Fits64;

use super::*;
use crate::builder::{AxisSystemBuilder, ShapeBuilder, TwistBuilder, TwistSystemBuilder};

#[derive(Debug, Clone)]
pub enum Transformable {
    Axis {
        db: Arc<Mutex<AxisSystemBuilder>>,
        vector: Vector,
    },
    Color {
        db: Arc<Mutex<ShapeBuilder>>,
        blades: Vec<Blade>,
    },
    Manifold(LuaManifold),
    Multivector(LuaMultivector),
    // TODO: piece
    Transform(LuaTransform),
    Twist {
        axis_vector: Vector,
        transform: Isometry,
        db: Arc<Mutex<TwistSystemBuilder>>,
    },
    Vector(LuaVector),

    Error(LuaError),
}
impl<'lua> FromLua<'lua> for Transformable {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        None.or_else(|| lua.unpack(value.clone()).ok().and_then(Self::from_axis))
            .or_else(|| lua.unpack(value.clone()).ok().and_then(Self::from_color))
            .or_else(|| lua.unpack(value.clone()).ok().map(Self::Manifold))
            .or_else(|| lua.unpack(value.clone()).ok().map(Self::Multivector))
            .or_else(|| lua.unpack(value.clone()).ok().map(Self::Transform))
            .or_else(|| lua.unpack(value.clone()).ok().and_then(Self::from_twist))
            .or_else(|| lua.unpack(value.clone()).ok().map(Self::Vector))
            .ok_or_else(|| {
                lua_convert_error(
                    &value,
                    "axis, color, manifold, multivector, transform, twist, \
                     vector, or table containing only those types as values",
                )
            })
    }
}
impl Transformable {
    fn from_axis(axis: LuaAxis) -> Option<Self> {
        let vector = axis.vector().ok()?;
        let db = axis.db;
        Some(Self::Axis { db, vector })
    }
    fn from_color(color: LuaColor) -> Option<Self> {
        let blades = color.blades().ok()?;
        let db = color.db;
        Some(Self::Color { db, blades })
    }
    fn from_twist(twist: LuaTwist) -> Option<Self> {
        let t = twist.get().ok()?;
        let db = twist.db;
        let axis_vector = db.lock().axes.lock().get(t.axis).ok()?.vector().clone();
        let transform = t.transform.clone();
        Some(Self::Twist {
            axis_vector,
            transform,
            db,
        })
    }

    pub fn into_lua<'lua>(&self, lua: &'lua Lua) -> Option<LuaResult<LuaValue<'lua>>> {
        match self {
            Self::Axis { db, vector } => {
                let db = Arc::clone(&db);
                let id = *db.lock().vector_to_id().get(vector)?;
                Some(LuaAxis { id, db }.into_lua(lua))
            }
            Self::Color { db, blades } => {
                let db = Arc::clone(&db);
                let db_guard = db.lock();
                let mut space = db_guard.space.lock();
                let manifold_set = match blades
                    .iter()
                    .map(|blade| space.add_manifold(blade.clone()))
                    .try_collect()
                {
                    Ok(set) => set,
                    Err(e) => return Some(Err(e.into_lua_err())),
                };
                let id = *db.lock().colors.manifold_set_to_id().get(&manifold_set)?;
                drop(space);
                drop(db_guard);
                Some(LuaColor { id, db }.into_lua(lua))
            }
            Self::Manifold(m) => Some(m.clone().into_lua(lua)),
            Self::Multivector(m) => Some(m.clone().into_lua(lua)),
            Self::Transform(t) => Some(t.clone().into_lua(lua)),
            Self::Twist {
                db,
                axis_vector,
                transform,
            } => {
                let db = Arc::clone(&db);
                let db_guard = db.lock();
                let id = *db_guard.data_to_id().get(&TwistBuilder {
                    axis: *db_guard.axes.lock().vector_to_id().get(axis_vector)?,
                    transform: transform.clone(),
                })?;
                drop(db_guard);
                Some(LuaTwist { id, db }.into_lua(lua))
            }
            Self::Vector(v) => Some(v.clone().into_lua(lua)),

            Self::Error(e) => Some(Err(e.clone())),
        }
    }

    pub fn transform(&self, t: &Isometry) -> LuaResult<Self> {
        match self {
            Self::Axis { db, vector } => Ok(Self::Axis {
                db: Arc::clone(db),
                vector: t.transform_vector(vector),
            }),
            Self::Color { db, blades } => Ok(Self::Color {
                db: Arc::clone(db),
                blades: blades.into_iter().map(|b| t.transform_blade(b)).collect(),
            }),
            Self::Manifold(m) => Ok(Self::Manifold(m.transform(t)?)),
            Self::Multivector(LuaMultivector(m)) => {
                Ok(Self::Multivector(LuaMultivector(t.transform(m))))
            }
            Self::Transform(LuaTransform(t2)) => {
                Ok(Self::Transform(LuaTransform(t.transform_isometry(t2))))
            }
            Self::Twist {
                axis_vector,
                transform,
                db,
            } => Ok(Self::Twist {
                axis_vector: t.transform_vector(axis_vector),
                transform: t.transform_isometry(transform),
                db: Arc::clone(db),
            }),
            Self::Vector(v) => Ok(Self::Vector(v.transform(t)?)),

            Self::Error(e) => Err(e.clone()),
        }
    }
}
impl ApproxHashMapKey for Transformable {
    type Hash = TransformableHash;

    fn approx_hash(&self, mut float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        match self {
            Self::Axis { db: _, vector } => vector.approx_hash(float_hash_fn).into(),
            Self::Color { db: _, blades } => blades
                .iter()
                .map(|blade| blade.approx_hash(&mut float_hash_fn).into())
                .collect(),
            Self::Manifold(LuaManifold { manifold, .. }) => manifold.to_u64().into(),
            Self::Multivector(LuaMultivector(m)) => m.approx_hash(float_hash_fn).into(),
            Self::Transform(LuaTransform(t)) => t.approx_hash(float_hash_fn).into(),
            Self::Twist {
                axis_vector,
                transform,
                db: _,
            } => [
                axis_vector.approx_hash(&mut float_hash_fn).into(),
                transform.approx_hash(&mut float_hash_fn).into(),
            ]
            .into_iter()
            .collect(),
            Self::Vector(LuaVector(v)) => v.approx_hash(float_hash_fn).into(),

            Self::Error(_) => TransformableHash::Nil,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransformableHash {
    Nil,
    Id(u64),
    Vector(VectorHash),
    Multivector(MultivectorHash),
    Vec(Vec<TransformableHash>),
}
impl From<u64> for TransformableHash {
    fn from(value: u64) -> Self {
        Self::Id(value)
    }
}
impl From<VectorHash> for TransformableHash {
    fn from(value: VectorHash) -> Self {
        Self::Vector(value)
    }
}
impl From<MultivectorHash> for TransformableHash {
    fn from(value: MultivectorHash) -> Self {
        Self::Multivector(value)
    }
}
impl FromIterator<TransformableHash> for TransformableHash {
    fn from_iter<T: IntoIterator<Item = TransformableHash>>(iter: T) -> Self {
        Self::Vec(iter.into_iter().collect())
    }
}
