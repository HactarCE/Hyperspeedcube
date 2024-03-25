pub trait LuaUserDataConvertWrap {
    const TYPE_NAME: &'static str;
    type Inner;
}

macro_rules! lua_userdata_value_conversion_wrapper {
    (
        #[name = $type_name_str:literal]
        $vis:vis struct $wrapper_name:ident($inner:ty) $(;)?
    ) => {
        lua_userdata_value_conversion_wrapper! {
            #[name = $type_name_str, convert_str = $type_name_str]
            $vis struct $wrapper_name($inner) = |_lua| {}
        }
    };

    (
        #[name = $type_name_str:literal, convert_str = $convert_str:literal]
        $vis:vis struct $wrapper_name:ident($inner:ty) = |$lua_ctx:ident| {
            $($tok:tt)*
        }
    ) => {
        #[derive(Debug, Clone)]
        $vis struct $wrapper_name(pub $inner);

        impl $crate::lua::types::wrappers::LuaUserDataConvertWrap for $wrapper_name {
            const TYPE_NAME: &'static str = $type_name_str;
            type Inner = $inner;
        }

        impl<'lua> ::rlua::FromLua<'lua> for $wrapper_name {
            fn from_lua(lua_value: ::rlua::Value<'lua>, lua: ::rlua::Context<'lua>) -> rlua::Result<Self> {
                let $lua_ctx = lua;
                Ok(Self(lua_convert!(match (lua, &lua_value, $convert_str) {
                    <$crate::lua::types::wrappers::LuaNamedUserData<$inner>>(userdata) => Ok(userdata.0),
                    $($tok)*
                })?))
            }
        }

        impl<'lua> ToLua<'lua> for $wrapper_name {
            fn to_lua(self, lua: LuaContext<'lua>) -> LuaResult<LuaValue<'lua>> {
                $crate::lua::types::wrappers::LuaNamedUserData(self.0).to_lua(lua)
            }
        }
    };
}
macro_rules! lua_userdata_multivalue_conversion_wrapper {
    ($vis:vis struct $wrapper_name:ident($inner:ty) = $func:expr) => {
        #[derive(Debug)]
        $vis struct $wrapper_name(pub $inner);

        impl<'lua> ::rlua::FromLuaMulti<'lua> for $wrapper_name {
            fn from_lua_multi(values: LuaMultiValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
                match lua.unpack_multi::<$crate::lua::types::wrappers::LuaNamedUserData<$inner>>(
                    values.clone(),
                ) {
                    Ok(ret) => Ok(Self(ret.0)),
                    Err(_) => Ok(Self($func(lua, values)?)),
                }
            }
        }
    };
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct LuaNamedUserData<T>(pub T);

macro_rules! lua_convert {
    // Entry point
    (
        match ($lua:expr, $lua_value:expr, $result_type_name:literal) {
            $($tok:tt)*
        }
    ) => {{
        #[allow(unused)]
        let lua = $lua;
        let val: &::rlua::Value<'_> = $lua_value;
        lua_convert!(@ lua, val; $($tok)*).map_err(|message| {
            ::rlua::Error::FromLuaConversionError {
                from: $crate::lua::lua_type_name(val),
                to: $result_type_name,
                message,
            }
        })
    }};

    // Fallback case
    (
        @ $lua:ident, $val:ident;
    ) => {
        Err(None)
    };

    // Convert from specific userdata type
    (
        @ $lua:ident, $val:ident;
        < $userdata_type:ty >($unerased_value:pat) => $ret:expr
        $(, $($rest:tt)*)?
    ) => {
        if let Ok($unerased_value) = <$userdata_type>::from_lua($val.clone(), $lua) {
            let ret: Result<_, String> = $ret;
            #[allow(unreachable_code)]
            ret.map_err(Some)
        } else {
            lua_convert!(
                @ $lua, $val;
                $($($rest)*)?
            )
        }
    };

    // Convert from Lua built-in type
    (
        @ $lua:ident, $val:ident;
        $pattern:pat => $ret:expr
        $(, $($rest:tt)*)?
    ) => {
        if let $pattern = $val.clone() {
            let ret: Result<_, String> = $ret;
            #[allow(unreachable_code)]
            ret.map_err(Some)
        } else {
            lua_convert!(
                @ $lua, $val;
                $($($rest)*)?
            )
        }
    };
}
