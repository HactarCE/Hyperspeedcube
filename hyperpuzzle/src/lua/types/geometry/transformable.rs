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

use super::*;
use crate::builder::{AxisSystemBuilder, ShapeBuilder, TwistSystemBuilder};

/// Lua value that can be transformed by an isometry.
///
/// This abstraction is useful for the method defined on `LuaSymmetry` that
/// computes objects' orbits.
#[derive(Debug, Clone)]
pub enum Transformable {
    /// Axis in an axis system.
    Axis {
        /// Axis system.
        db: Arc<Mutex<AxisSystemBuilder>>,
        /// Vector of the axis.
        vector: Vector,
    },
    /// Color in the color system of a shape.
    Color {
        /// Shape.
        db: Arc<Mutex<ShapeBuilder>>,
        /// Manifolds of the color.
        blades: Vec<Blade>,
    },
    /// Manifold.
    Manifold(LuaManifold),
    /// Multivector.
    Multivector(LuaMultivector),
    // TODO: piece
    /// Transform (isometry).
    Transform(LuaTransform),
    /// Twist in a twist system.
    Twist {
        /// Twist system.
        db: Arc<Mutex<TwistSystemBuilder>>,
        /// Vector of the axis of the twist.
        axis_vector: Vector,
        /// Transform of the twist.
        transform: Isometry,
    },
    /// Vector.
    Vector(LuaVector),
}
impl<'lua> FromLua<'lua> for Transformable {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if value.is_nil() {
            None
        } else {
            None.or_else(|| lua.unpack(value.clone()).and_then(Self::from_axis).ok())
                .or_else(|| lua.unpack(value.clone()).and_then(Self::from_color).ok())
                .or_else(|| lua.unpack(value.clone()).map(Self::Manifold).ok())
                .or_else(|| lua.unpack(value.clone()).map(Self::Multivector).ok())
                .or_else(|| lua.unpack(value.clone()).map(Self::Transform).ok())
                .or_else(|| lua.unpack(value.clone()).and_then(Self::from_twist).ok())
                .or_else(|| lua.unpack(value.clone()).map(Self::Vector).ok())
        }
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
    fn from_axis(axis: LuaAxis) -> LuaResult<Self> {
        let vector = axis.vector()?;
        let db = axis.db;
        Ok(Self::Axis { db, vector })
    }
    fn from_color(color: LuaColor) -> LuaResult<Self> {
        let blades = color.blades()?;
        let db = color.db;
        Ok(Self::Color { db, blades })
    }
    fn from_twist(twist: LuaTwist) -> LuaResult<Self> {
        let t = twist.get()?;
        let axis_vector = twist.axis()?.vector()?;
        let transform = t.transform.clone();
        let db = twist.db;
        Ok(Self::Twist {
            db,
            axis_vector,
            transform,
        })
    }

    /// Converts
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
                let axis = *db_guard.axes.lock().vector_to_id().get(axis_vector)?;
                let id = db_guard.data_to_id(axis, transform)?;
                drop(db_guard);
                Some(LuaTwist { id, db }.into_lua(lua))
            }
            Self::Vector(v) => Some(v.clone().into_lua(lua)),
        }
    }

    /// Transforms the object by `t`.
    ///
    /// Generally if the object is a pure mathematical object (vector,
    /// multivector, etc.) then it is transformed directly, and if it is a
    /// puzzle element (twist axis, color, etc.) then the nearest equivalent one
    /// is returned. See the `transform()` method on individual Lua wrapper
    /// types to learn how each one is transformed.
    pub fn transform(&self, t: &Isometry) -> Self {
        match self {
            Self::Axis { db, vector } => Self::Axis {
                db: Arc::clone(db),
                vector: t.transform_vector(vector),
            },
            Self::Color { db, blades } => Self::Color {
                db: Arc::clone(db),
                blades: blades.into_iter().map(|b| t.transform_blade(b)).collect(),
            },
            Self::Manifold(m) => Self::Manifold(m.transform(t)),
            Self::Multivector(LuaMultivector(m)) => {
                Self::Multivector(LuaMultivector(t.transform(m)))
            }
            Self::Transform(LuaTransform(t2)) => {
                Self::Transform(LuaTransform(t.transform_isometry(t2)))
            }
            Self::Twist {
                db,
                axis_vector,
                transform,
            } => Self::Twist {
                db: Arc::clone(db),
                axis_vector: t.transform_vector(axis_vector),
                transform: t.transform_isometry(transform),
            },
            Self::Vector(v) => Self::Vector(v.transform(t)),
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
            Self::Manifold(LuaManifold(b)) => b.approx_hash(float_hash_fn).into(),
            Self::Multivector(LuaMultivector(m)) => m.approx_hash(float_hash_fn).into(),
            Self::Transform(LuaTransform(t)) => t.approx_hash(float_hash_fn).into(),
            Self::Twist {
                db: _,
                axis_vector,
                transform,
            } => [
                axis_vector.approx_hash(&mut float_hash_fn).into(),
                transform.approx_hash(&mut float_hash_fn).into(),
            ]
            .into_iter()
            .collect(),
            Self::Vector(LuaVector(v)) => v.approx_hash(float_hash_fn).into(),
        }
    }
}

/// Hash of a [`Transformable`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransformableHash {
    /// Hash of a vector.
    Vector(VectorHash),
    /// Hash of a multivector or blade.
    Multivector(MultivectorHash),
    /// Hash of multiple vectors, multivectors, or blades.
    Vec(Vec<TransformableHash>),
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
