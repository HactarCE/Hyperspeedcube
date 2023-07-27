use super::*;
use crate::math::{cga::*, *};

impl LuaUserData for Isometry {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_meta_method(LuaMetaMethod::Mul, |lua, this, rhs: LuaValue<'_>| {
            lua_convert!(match (lua, &rhs, "vector, manifold, or transform") {
                <Vector>(v) => Ok(this.transform_vector(v).to_lua(lua)?),
                // <Manifold>(m) => Ok(this.transform(m).to_lua(lua)?),
                <Isometry>(rhs) => Ok((this * rhs).to_lua(lua)?),
            })
        });
    }
}
