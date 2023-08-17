use super::*;
use crate::PieceSet;

lua_userdata_value_conversion_wrapper! {
    #[name = "pieceset"]
    pub struct LuaPieceSet(PieceSet) ;
}

impl LuaUserData for LuaNamedUserData<PieceSet> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method("carve", |lua, Self(this), LuaManifold(m)| {
            LuaSpace::with(lua, |space| {
                Ok(LuaPieceSet(todo!("carve piece set")))
                // let result = space.carve(this, m);
                // Ok(LuaPieceSet(result.map_err(|e| {
                //     LuaError::external(e.context("cutting piece"))
                // })?))
            })
        });
    }
}
