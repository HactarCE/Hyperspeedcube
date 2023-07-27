use rlua::prelude::*;

macro_rules! lua_fn {
    ($closure:expr) => {
        $crate::library::lua::macros::LuaFn(|lua, args| {
            $closure(lua, lua.unpack_multi(args)?)?.to_lua_multi(lua)
        })
    };
}

pub struct LuaFn<F>(pub F)
where
    F: for<'lua> Fn(LuaContext<'lua>, LuaMultiValue<'lua>) -> LuaResult<LuaMultiValue<'lua>>;

impl<F> LuaUserData for LuaFn<F>
where
    F: for<'lua> Fn(LuaContext<'lua>, LuaMultiValue<'lua>) -> LuaResult<LuaMultiValue<'lua>>,
{
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_meta_method(LuaMetaMethod::Call, |lua, this, args| (this.0)(lua, args))
    }
}
