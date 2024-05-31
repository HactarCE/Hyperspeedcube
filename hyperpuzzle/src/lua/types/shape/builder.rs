use std::sync::Arc;

use itertools::Itertools;
use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::{builder::ShapeBuilder, lua::lua_warn_fn};

/// Lua handle to a shape under construction.
#[derive(Debug, Clone)]
pub struct LuaShape(pub Arc<Mutex<ShapeBuilder>>);

impl<'lua> FromLua<'lua> for LuaShape {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaUserData for LuaShape {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("shape"));

        fields.add_field_method_get("id", |_lua, this| Ok(this.lock().id.clone()));
        fields.add_field_method_get("space", |_lua, this| {
            Ok(LuaSpace(Arc::clone(&this.lock().space)))
        });
        fields.add_field_method_get("ndim", |_lua, this| Ok(this.lock().ndim()));
        fields.add_field_method_get("colors", |_lua, Self(this)| {
            Ok(LuaColorSystem(Arc::clone(this)))
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            let this = this.lock();
            let ndim = this.ndim();
            if let Some(id) = &this.id {
                Ok(format!("shape({id:?}, ndim={ndim})"))
            } else {
                Ok(format!("shape(ndim={ndim})"))
            }
        });

        methods.add_method("carve", |lua, this, cuts| {
            this.cut(lua, cuts, CutMode::Carve, StickerMode::NewColor)
        });
        methods.add_method("carve_unstickered", |lua, this, cuts| {
            this.cut(lua, cuts, CutMode::Carve, StickerMode::None)
        });
        methods.add_method("slice", |lua, this, cuts| {
            this.cut(lua, cuts, CutMode::Slice, StickerMode::None)
        });
    }
}

impl LuaShape {
    /// Returns a mutex guard granting temporary access to the underlying
    /// [`ShapeBuilder`].
    pub fn lock(&self) -> MutexGuard<'_, ShapeBuilder> {
        self.0.lock()
    }

    /// Cut the puzzle.
    pub fn cut<'lua>(
        &self,
        lua: &'lua Lua,
        cuts: LuaSymmetricSet<LuaHyperplane>,
        cut_mode: CutMode,
        sticker_mode: StickerMode,
    ) -> LuaResult<()> {
        let mut this = self.lock();

        for (name, LuaHyperplane(plane)) in cuts.to_vec(lua)? {
            let color = match sticker_mode {
                StickerMode::NewColor => Some({
                    let c = this.colors.add(vec![plane.clone()]).into_lua_err()?;
                    this.colors.names.set(c, name, lua_warn_fn(lua));
                    c
                }),
                StickerMode::None => None,
            };
            match cut_mode {
                CutMode::Carve => this.carve(None, plane, color).into_lua_err()?,
                CutMode::Slice => this.slice(None, plane, color, color).into_lua_err()?,
            }
        }

        Ok(())
    }
}

/// Which pieces to keep when cutting the shape.
pub enum CutMode {
    /// Delete any pieces outside the cut; keep only pieces inside the cut.
    Carve,
    /// Keep all pieces on both sides of the cut.
    Slice,
}
/// How to sticker new facets created by a cut.
pub enum StickerMode {
    /// Add a new color for each cut and create new stickers with that color on
    /// both sides of the cut.
    NewColor,
    /// Do not add new stickers.
    None,
}

/// Symmetric set of a particular type of object.
#[derive(Debug, Clone)]
pub enum LuaSymmetricSet<T> {
    /// Single object (using the trivial symmetry).
    Single(T),
    /// Symmetric orbit of an object.
    Orbit(LuaOrbit),
}
impl<'lua, T: LuaTypeName + FromLua<'lua>> FromLua<'lua> for LuaSymmetricSet<T> {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if let Ok(orbit) = <_>::from_lua(value.clone(), lua) {
            Ok(Self::Orbit(orbit))
        } else if let Ok(h) = <_>::from_lua(value.clone(), lua) {
            Ok(Self::Single(h))
        } else {
            // This error isn't quite accurate, but it's close enough. The error
            // message will say that we need a value of type `T`, but in fact we
            // accept an orbit of `T` as well.
            lua_convert_err(&value, T::type_name(lua)?)
        }
    }
}
impl<'lua, T: LuaTypeName + FromLua<'lua> + Clone> LuaSymmetricSet<T> {
    /// Returns a list of all the objects in the orbit.
    pub fn to_vec(&self, lua: &'lua Lua) -> LuaResult<Vec<(Option<String>, T)>> {
        match self {
            LuaSymmetricSet::Single(v) => Ok(vec![(None, v.clone())]),
            LuaSymmetricSet::Orbit(orbit) => orbit
                .iter_in_order()
                .map(|(_transform, name, values)| {
                    let v = Self::to_expected_type(lua, values.get(0))?;
                    Ok((name.clone(), v))
                })
                .try_collect(),
        }
    }
    /// Returns the initial object from which the others are generated.
    pub fn first(&self, lua: &'lua Lua) -> LuaResult<T> {
        match self {
            LuaSymmetricSet::Single(v) => Ok(v.clone()),
            LuaSymmetricSet::Orbit(orbit) => Self::to_expected_type(lua, orbit.init().get(0)),
        }
    }

    fn to_expected_type(lua: &'lua Lua, maybe_obj: Option<&Transformable>) -> LuaResult<T> {
        let lua_value =
            maybe_obj
                .and_then(|obj| obj.into_lua(lua))
                .ok_or(LuaError::external(format!(
                    "expected orbit of {}",
                    T::type_name(lua)?,
                )))??;
        T::from_lua(lua_value, lua)
    }
}
