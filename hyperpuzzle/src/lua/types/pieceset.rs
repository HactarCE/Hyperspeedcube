use super::*;
use crate::PieceSet;

lua_userdata_value_conversion_wrapper! {
    #[name = "pieceset"]
    pub struct LuaPieceSet(PieceSet) ;
}

impl LuaUserData for LuaNamedUserData<PieceSet> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method("copy", |_lua, this, ()| Ok(this.clone()));

        methods.add_method_mut("carve", |lua, Self(this), LuaManifold(m)| {
            *this = LuaPuzzleBuilder::with(lua, |puzzle| puzzle.carve(this, m))?;
            Ok(())
        });

        methods.add_method_mut("slice", |lua, Self(this), LuaManifold(m)| {
            *this = LuaPuzzleBuilder::with(lua, |puzzle| puzzle.slice(this, m))?;
            Ok(())
        });
    }
}
