use tinyset::Set64;

use super::*;

use crate::geometry::{CutParams, Manifold, ShapeRef};
use crate::math::cga::Blade;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct LuaShapeSet(pub Set64<ShapeRef>);

impl LuaUserData for LuaShapeSet {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method_mut("carve", |lua, this, blade: Blade| {
            let LuaNdim(ndim) = super::lua_get_ndim(lua, None)?;
            let space = lua.globals().get::<_, LuaSpace>("SPACE")?;
            space
                .0
                .lock()
                .cut(CutParams {
                    cut: Manifold::from_opns(blade, ndim).map_err(|e| {
                        LuaError::external(e.context("error constructing manifold"))
                    })?,
                    inside: crate::geometry::CutOp::Keep(None),
                    outside: crate::geometry::CutOp::Remove,
                })
                .map_err(|e| LuaError::external(e.context("cutting shape")))?;
            this.0 = space.0.lock().roots().iter().map(|&id| id.into()).collect();
            Ok(())
        });
    }
}
