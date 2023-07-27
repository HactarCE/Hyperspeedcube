use rlua::prelude::*;

use super::types::LuaNdim;
use crate::geometry::Manifold;
use crate::math::cga::Blade;
use crate::math::{Vector, VectorRef};

pub fn lua_construct_plane_manifold<'lua>(
    lua: LuaContext<'lua>,
    arg: LuaTable<'lua>,
) -> LuaResult<Blade> {
    let LuaNdim(ndim) = super::types::lua_get_ndim(lua, None)?;
    let pole = arg.get::<_, Vector>("pole")?;
    let normal = pole
        .normalize()
        .ok_or_else(|| LuaError::external("zero vector is not a valid pole"))?;
    Ok(Manifold::new_hyperplane(normal, pole.mag(), ndim)
        .opns()
        .clone())
}
