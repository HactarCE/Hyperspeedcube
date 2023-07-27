use super::*;
use crate::math::{cga::*, *};

impl LuaUserData for Blade {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function("pss", |_lua, LuaOptNdim(ndim)| {
            Ok(Blade::pseudoscalar(ndim))
        });
        methods.add_function("inv_pss", |_lua, LuaOptNdim(ndim)| {
            Ok(Blade::inverse_pseudoscalar(ndim))
        });

        methods.add_method("ipns_to_opns", |_lua, this, LuaOptNdim(ndim)| {
            Ok(this.ipns_to_opns(ndim))
        });
        methods.add_method("opns_to_ipns", |_lua, this, LuaOptNdim(ndim)| {
            Ok(this.opns_to_ipns(ndim))
        });
    }
}
