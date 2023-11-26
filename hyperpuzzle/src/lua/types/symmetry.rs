use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;

pub use super::*;

lua_userdata_value_conversion_wrapper! {
    #[name = "symmetry", convert_str = "symmetry or schlafli symbol"]
    pub struct LuaSymmetry(SchlafliSymbol) = |_lua| {
        <LuaTable<'_>>(t) => Ok(LuaSymmetry::construct_from_table(t)?),
    }
}

impl LuaSymmetry {
    fn construct_from_table(t: LuaTable<'_>) -> LuaResult<SchlafliSymbol> {
        t.sequence_values()
            .into_iter()
            .try_collect()
            .map(SchlafliSymbol::from_indices)
    }

    pub fn construct_from_schlafli_table(
        _lua: LuaContext<'_>,
        t: LuaTable<'_>,
    ) -> LuaResult<LuaSymmetry> {
        Self::construct_from_table(t).map(LuaSymmetry)
    }
}

impl LuaUserData for LuaNamedUserData<SchlafliSymbol> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method("ndim", |_lua, Self(this), ()| Ok(this.ndim()));

        methods.add_method("vec", |_lua, Self(this), LuaVector(v)| {
            let mat = Matrix::from_cols(this.mirrors().into_iter().map(|Mirror(m)| m));
            Ok(LuaVector(mat * v))
        });
    }

    fn get_uvalues_count(&self) -> std::os::raw::c_int {
        1
    }
}
