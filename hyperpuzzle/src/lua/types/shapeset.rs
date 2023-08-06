use hypershape::prelude::*;

use super::*;

lua_userdata_value_conversion_wrapper! {
    #[name = "shapeset", convert_str = "shapeset or manifold"]
    pub struct LuaShapeSet(ShapeSet) = |lua| {
        <_>(LuaManifold(m)) => Ok(construct_shapeset_from_manifold(lua, m)?),
    }
}

fn construct_shapeset_from_manifold(lua: LuaContext<'_>, m: ManifoldRef) -> LuaResult<ShapeSet> {
    LuaSpace::with(lua, |space| match space.add_shape_without_boundary(m) {
        Ok(shape) => Ok(ShapeSet::from_iter([shape])),
        Err(e) => Err(LuaError::external(e)),
    })
}

impl LuaUserData for LuaNamedUserData<ShapeSet> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method("carve", |lua, Self(this), LuaManifold(m)| {
            LuaSpace::with(lua, |space| {
                let result = space.carve(this, m);
                Ok(LuaShapeSet(result.map_err(|e| {
                    LuaError::external(e.context("cutting shape"))
                })?))
            })
        });
    }
}
