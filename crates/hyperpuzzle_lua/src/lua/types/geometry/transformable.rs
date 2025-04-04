// TODO: this doesn't correctly handle missing elements (e.g., missing one axis
// but the others should still work)

use std::hash::Hash;
use std::sync::Arc;

use hypermath::collections::approx_hashmap::{FloatHash, MultivectorHash, VectorHash};
use hypermath::pga::Motor;
use hypermath::prelude::*;
use parking_lot::Mutex;

use super::*;
use crate::builder::{PuzzleBuilder, TwistKey};

/// Lua value that can be transformed by an isometry.
///
/// This abstraction is useful for the method defined on `LuaSymmetry` that
/// computes objects' orbits.
#[derive(Debug, Clone)]
pub enum Transformable {
    /// Axis in an axis system.
    Axis {
        /// Puzzle containing the axis system.
        db: Arc<Mutex<PuzzleBuilder>>,
        /// Vector of the axis.
        vector: Vector,
    },
    /// Blade.
    Blade(LuaBlade),
    /// Color in the color system of a shape.
    Color(Option<LuaColor>),
    /// Region.
    Region(LuaRegion),
    /// Transform (isometry).
    Transform(LuaTransform),
    /// Twist in a twist system.
    Twist {
        /// Puzzle containing the twist system.
        db: Arc<Mutex<PuzzleBuilder>>,
        /// Vector of the axis of the twist.
        axis_vector: Vector,
        /// Transform of the twist.
        transform: Motor,
    },
}

impl FromLua for Transformable {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        if value.is_nil() {
            None
        } else {
            // Be careful with the order here so that we don't accidentally
            // coerce things in the wrong way.
            None.or_else(|| cast_userdata(lua, &value).and_then(Self::from_axis).ok())
                .or_else(|| cast_userdata(lua, &value).and_then(Self::from_color).ok())
                .or_else(|| cast_userdata(lua, &value).and_then(Self::from_twist).ok())
                .or_else(|| cast_userdata(lua, &value).map(Self::Blade).ok())
                .or_else(|| cast_userdata(lua, &value).map(Self::Transform).ok())
                .or_else(|| cast_userdata(lua, &value).map(Self::Region).ok())
        }
        .ok_or_else(|| {
            lua_convert_error(&value, "axis, color, region, transform, twist, or vector")
        })
    }
}

impl Transformable {
    /// Converts a Lua value into an object that can be transformed arbitrarily.
    pub fn from_vector(v: impl VectorRef) -> LuaResult<Self> {
        Ok(Self::Blade(LuaBlade(pga::Blade::from_vector(v))))
    }

    fn from_axis(axis: LuaAxis) -> LuaResult<Self> {
        let vector = axis.vector()?;
        let db = axis.db;
        Ok(Self::Axis { db, vector })
    }
    fn from_color(color: LuaColor) -> LuaResult<Self> {
        Ok(Self::Color(Some(color)))
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

    /// Converts the object back into a Lua value. Returns `None` if there is no
    /// such transformed object (e.g., no such twist axis after transformation).
    pub fn into_lua(&self, lua: &Lua) -> Option<LuaResult<LuaValue>> {
        match self {
            Self::Axis { db, vector } => {
                let db = Arc::clone(db);
                let id = db.lock().twists.axes.vector_to_id(vector)?;
                Some(LuaAxis { id, db }.into_lua(lua))
            }
            Self::Blade(b) => Some(b.clone().into_lua(lua)),
            Self::Color(c) => Some(c.clone().into_lua(lua)),
            Self::Region(r) => Some(r.clone().into_lua(lua)),
            Self::Transform(t) => Some(t.clone().into_lua(lua)),
            Self::Twist {
                db,
                axis_vector,
                transform,
            } => {
                let db = Arc::clone(db);
                let puz = db.lock();
                let axis = puz.twists.axes.vector_to_id(axis_vector)?;
                let twist_key = TwistKey::new(axis, transform).ok()?;
                let id = puz.twists.data_to_id(&twist_key)?;
                drop(puz);
                Some(LuaTwist { id, db }.into_lua(lua))
            }
        }
    }

    /// Converts the object back into a Lua value. Returns `Ok(LuaNil)` if there
    /// is no such transformed object (e.g., no such twist axis after
    /// transformation).
    pub fn into_nillable_lua(&self, lua: &Lua) -> LuaResult<LuaValue> {
        self.into_lua(lua).transpose()?.into_lua(lua)
    }
}

impl TransformByMotor for Transformable {
    /// Transforms the object by the motor `m`.
    ///
    /// Generally if the object is a pure mathematical object (vector,
    /// multivector, etc.) then it is transformed directly, and if it is a
    /// puzzle element (twist axis, color, etc.) then the nearest equivalent one
    /// is returned. See the `transform()` method on individual Lua wrapper
    /// types to learn how each one is transformed.
    fn transform_by(&self, m: &Motor) -> Self {
        match self {
            Self::Axis { db, vector } => Self::Axis {
                db: Arc::clone(db),
                vector: m.transform_vector(vector),
            },
            Self::Blade(b) => Self::Blade(m.transform(b)),
            Self::Color(_) => Self::Color(None), // TODO: support transforming colors
            Self::Region(r) => Self::Region(m.transform(r)),
            Self::Transform(LuaTransform(t)) => Self::Transform(LuaTransform(m.transform(t))),
            Self::Twist {
                db,
                axis_vector,
                transform,
            } => Self::Twist {
                db: Arc::clone(db),
                axis_vector: m.transform_vector(axis_vector),
                transform: m.transform(transform),
            },
        }
    }
}

impl ApproxHashMapKey for Transformable {
    type Hash = TransformableHash;

    fn approx_hash(&self, mut float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        match self {
            Self::Axis { db: _, vector } => vector.approx_hash(float_hash_fn).into(),
            Self::Blade(LuaBlade(b)) => b.approx_hash(float_hash_fn).into(),
            Self::Color(color) => color.as_ref().map(|c| c.id.0 as u64).into(),
            Self::Region(r) => {
                let mut planes = vec![];
                let mut ast_structure = String::new();
                hash_region(&mut float_hash_fn, &mut planes, &mut ast_structure, r);
                TransformableHash::Region {
                    planes,
                    ast_structure,
                }
            }
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
        }
    }
}

/// Hash of a [`Transformable`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransformableHash {
    /// Hash of a vector.
    Vector(VectorHash),
    /// Hash of a multivector.
    Multivector(MultivectorHash),
    /// Hash of multiple vectors or multivectors.
    Vec(Vec<TransformableHash>),
    /// Hash of an ID.
    Id(Option<u64>),
    /// Hash of a region.
    Region {
        /// Hyperplanes that factor into the region.
        planes: Vec<VectorHash>,
        /// Serialization of the tree of operations to construct the region.
        ast_structure: String,
    },
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
impl From<Option<u64>> for TransformableHash {
    fn from(value: Option<u64>) -> Self {
        Self::Id(value)
    }
}

fn hash_region(
    float_hash_fn: &mut impl FnMut(Float) -> FloatHash,
    planes: &mut Vec<VectorHash>,
    ast_structure: &mut String,
    r: &LuaRegion,
) {
    // The hash needs to be unambigous, but we never have to decode it, so this
    // is essentially a silly little domain-specific language.
    match r {
        LuaRegion::Nothing => ast_structure.push('_'),
        LuaRegion::Everything => ast_structure.push('*'),
        LuaRegion::HalfSpace(h) => {
            ast_structure.push('h');
            planes.push(h.approx_hash(float_hash_fn));
        }
        LuaRegion::And(xs) => {
            // `&XYZ.` = intersection of X, Y, and Z
            ast_structure.push('&');
            for x in xs {
                hash_region(float_hash_fn, planes, ast_structure, x);
            }
            ast_structure.push('.');
        }
        LuaRegion::Or(xs) => {
            // `|XYZ.` = union of X, Y, and Z
            ast_structure.push('|');
            for x in xs {
                hash_region(float_hash_fn, planes, ast_structure, x);
            }
            ast_structure.push('.');
        }
        LuaRegion::Xor(xs) => {
            // `^XYZ.` = symmetric difference of X, Y, and Z
            ast_structure.push('^');
            for x in xs {
                hash_region(float_hash_fn, planes, ast_structure, x);
            }
            ast_structure.push('.');
        }
        LuaRegion::Not(x) => {
            // `~X` = complement of X
            ast_structure.push('~');
            hash_region(float_hash_fn, planes, ast_structure, x);
        }
    }
}
