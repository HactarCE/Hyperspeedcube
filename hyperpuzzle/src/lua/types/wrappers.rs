use hypermath::prelude::*;
use itertools::Itertools;

use super::*;

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

#[derive(Debug, Clone)]
pub struct LuaNamedUserData<T>(pub T);

macro_rules! lua_convert {
    // Entry point
    (
        match ($lua:expr, $lua_value:expr, $result_type_name:literal) {
            $($tok:tt)*
        }
    ) => {{
        let lua = $lua;
        let val: &::rlua::Value = $lua_value;
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
        @ $lua:ident, $val:ident, $result_type_name:literal;
        $pattern:pat => $ret:expr
        $(, $($rest:tt)*)?
    ) => {
        if let $pattern = $val {
            let ret = Result<_, String> = $ret;
            ret.map_err(Some)
        } else {
            lua_convert!(
                @ $lua, $val;
                $($($rest)*)?
            )
        }
    };
}

pub struct LuaAxisName(pub u8);
impl<'lua> FromLua<'lua> for LuaAxisName {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        lua_convert!(match (lua, &lua_value, "axis name") {
            <String>(s) => {
                match s.chars().exactly_one() {
                    Ok(c) => match AXIS_NAMES.find(c.to_ascii_uppercase()) {
                        Some(i) => Ok(LuaAxisName(i as u8)),
                        None => Err(format!("no axis named '{c}'")),
                    }
                    Err(_) => Err("axis name must be single character".to_owned()),
                }
            },
        })
    }
}

pub struct LuaVectorIndex(pub u8);
impl<'lua> FromLua<'lua> for LuaVectorIndex {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        lua_convert!(match (lua, &lua_value, "vector index") {
            <LuaNdim>(ndim) => Ok(LuaVectorIndex(ndim.0 - 1)),
            <LuaAxisName>(axis) => Ok(LuaVectorIndex(axis.0)),
        })
    }
}

/// ∞ or nₒ.
pub enum NiNo {
    // nₒ
    No,
    /// ∞
    Ni,
}
pub struct LuaAxesString {
    /// Which of nₒ or ∞ to the beginning of the axes, if either.
    pub nino: Option<NiNo>,

    pub axes: Axes,
    pub sign: Sign,

    pub string: String,
}
impl LuaAxesString {
    pub fn to_multivector(&self) -> Multivector {
        let t = Term {
            coef: self.sign.to_num(),
            axes: self.axes,
        };
        match self.nino {
            None => t.into(),
            Some(NiNo::No) => Multivector::NO * t,
            Some(NiNo::Ni) => Multivector::NI * t,
        }
    }
}
impl<'lua> FromLua<'lua> for LuaAxesString {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        let string = if matches!(lua_value, LuaValue::Nil) {
            String::new()
        } else if let Ok(LuaVectorIndex(i)) = LuaVectorIndex::from_lua(lua_value.clone(), lua) {
            i.to_string()
        } else {
            String::from_lua(lua_value, lua)?
        };

        let mut use_true_basis = false;
        let mut use_null_vector_basis = false;
        let mut axes = Axes::empty();
        let mut sign = 1.0;
        let mut zeroed = false;
        for c in string.chars() {
            let new_axis: Axes = match c {
                // Remember, Lua is 1-indexed so the X axis is 1.
                '1' | 'x' | 'X' => Axes::X,
                '2' | 'y' | 'Y' => Axes::Y,
                '3' | 'z' | 'Z' => Axes::Z,
                '4' | 'w' | 'W' => Axes::W,
                '5' | 'v' | 'V' => Axes::U,
                '6' | 'u' | 'U' => Axes::V,
                '7' => Axes::T,
                '8' => Axes::S,
                '9' => Axes::R,

                // Ignore these characters.
                's' | 'e' | 'n' | '_' | ' ' => continue,

                // Store nₒ in `E_MINUS` for now.
                'o' => {
                    use_null_vector_basis = true;
                    zeroed |= axes.contains(Axes::E_MINUS); // get nullvector'd lmao
                    Axes::E_MINUS
                }
                // Store ∞ in `E_PLUS` for now.
                'i' => {
                    use_null_vector_basis = true;
                    zeroed |= axes.contains(Axes::E_PLUS); // get nullvector'd lmao
                    Axes::E_PLUS
                }

                'm' | '-' => {
                    use_true_basis = true;
                    Axes::E_MINUS
                }
                'p' | '+' => {
                    use_true_basis = true;
                    Axes::E_PLUS
                }
                'E' => {
                    use_true_basis = true;
                    Axes::E_PLANE
                }

                _ => return Err(LuaError::external(format!("unknown axis {c:?}"))),
            };
            sign *= axes * new_axis;
            axes ^= new_axis;
        }

        if use_true_basis && use_null_vector_basis {
            return Err(LuaError::external(
                "cannot mix true basis (e₋ e₊) with null vector basis (o ∞)",
            ));
        }

        if zeroed {
            return Err(LuaError::external(format!(
                "component '{string}' is always zero",
            )));
        }

        let mut nino = None;
        if use_null_vector_basis {
            // We stored nₒ in `E_MINUS` and ∞ in `E_PLUS`.
            // nₒ and ∞ are each allowed, but not at the same time.
            if axes.contains(Axes::E_PLANE) {
                return Err(LuaError::external(format!(
                    "cannot access component {string:?}",
                )));
            }
            if axes.contains(Axes::E_MINUS) {
                nino = Some(NiNo::No);
            } else if axes.contains(Axes::E_PLUS) {
                nino = Some(NiNo::Ni);
            }
            axes.remove(Axes::E_PLANE);
        }

        let sign = Sign::from(sign);

        Ok(LuaAxesString {
            nino,
            axes,
            sign,
            string,
        })
    }
}
