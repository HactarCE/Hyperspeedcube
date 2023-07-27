use itertools::Itertools;

use super::*;
use crate::math::*;

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
                from: lua_type_name(val),
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

macro_rules! lua_wrapper_struct {
    ($vis:vis struct $wrapper_name:ident($inner_vis:vis $inner_type:ty), $type_name:literal) => {
        $vis struct $wrapper_name($inner_vis $inner_type);
        impl<'lua> ::rlua::FromLua<'lua> for $wrapper_name {
            fn from_lua(lua_value: ::rlua::Value<'lua>, lua: ::rlua::Context<'lua>) -> ::rlua::Result<Self> {
                lua_convert!(match (lua, &lua_value, $type_name) {
                    <$inner_type>(v) => Ok(Self(v)),
                })
            }
        }
    };
}

macro_rules! lua_wrapper_enum {
    (
        #[name = $wrapper_name:literal]
        $vis:vis enum $wrapper:ident {
            $(
                #[name = $variant_name:literal]
                $variant:ident($inner_type:ty)
            ),* $(,)?
        }
    ) => {
        $vis enum $wrapper {
            $( $variant($inner_type), )*
        }
        impl<'lua> FromLua<'lua> for $wrapper {
            fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
                lua_convert!(match (lua, &lua_value, $wrapper_name) {
                    $( <$inner_type>(value) => Ok(Self::$variant(value)), )*
                })
            }
        }
        impl $wrapper {
            pub fn type_name(&self) -> &'static str {
                match self {
                    $( Self::$variant(_) => $variant_name, )*
                }
            }
        }
    };
}

pub struct LuaNdim(pub u8);
impl<'lua> FromLua<'lua> for LuaNdim {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        lua_convert!(match (lua, &lua_value, "number of dimensions") {
            <u8>(i) => if 1 <= i &&  i <= crate::MAX_NDIM {
                Ok(LuaNdim(i))
            } else {
                Err("out of range".to_owned())
            },
        })
    }
}

pub struct LuaOptNdim(pub u8);
impl<'lua> FromLua<'lua> for LuaOptNdim {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        lua_get_ndim(lua, Option::<LuaNdim>::from_lua(lua_value, lua)?)
            .map(|LuaNdim(ndim)| LuaOptNdim(ndim))
    }
}

pub fn lua_get_ndim(lua: LuaContext<'_>, fallback: Option<LuaNdim>) -> LuaResult<LuaNdim> {
    match fallback {
        Some(ndim) => Ok(ndim),
        None => match lua.globals().get("NDIM")? {
            LuaNil => Err(LuaError::external(
                "unknown number of dimensions; set global \
                 `NDIM` variable or pass NDIM as argument",
            )),
            other_value => LuaNdim::from_lua(other_value, lua),
        },
    }
}

pub struct LuaAxisName(pub u8);
impl<'lua> FromLua<'lua> for LuaAxisName {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        lua_convert!(match (lua, &lua_value, "axis name") {
            <String>(s) => {
                match s.chars().exactly_one() {
                    Ok(c) => match crate::AXIS_NAMES.find(c.to_ascii_uppercase()) {
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

    pub axes: cga::Axes,
    pub sign: Sign,

    pub string: String,
}
impl LuaAxesString {
    pub fn to_multivector(&self) -> cga::Multivector {
        let t = cga::Term {
            coef: self.sign.to_float(),
            axes: self.axes,
        };
        match self.nino {
            None => t.into(),
            Some(NiNo::No) => cga::Multivector::NO * t,
            Some(NiNo::Ni) => cga::Multivector::NI * t,
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
        let mut axes = cga::Axes::empty();
        let mut sign = 1.0;
        let mut zeroed = false;
        for c in string.chars() {
            let new_axis: cga::Axes = match c {
                // Remember, Lua is 1-indexed so the X axis is 1.
                '1' | 'x' | 'X' => cga::Axes::X,
                '2' | 'y' | 'Y' => cga::Axes::Y,
                '3' | 'z' | 'Z' => cga::Axes::Z,
                '4' | 'w' | 'W' => cga::Axes::W,
                '5' | 'u' | 'U' => cga::Axes::U,
                '6' | 'v' | 'V' => cga::Axes::V,
                '7' | 'r' | 'R' => cga::Axes::R,
                '8' | 's' | 'S' => cga::Axes::S,

                // Ignore these characters.
                'e' | 'n' | ' ' => continue,

                // Store nₒ in `E_MINUS` for now.
                'o' => {
                    use_null_vector_basis = true;
                    zeroed |= axes.contains(cga::Axes::E_MINUS); // get nullvector'd lmao
                    cga::Axes::E_MINUS
                }
                // Store ∞ in `E_PLUS` for now.
                'i' => {
                    use_null_vector_basis = true;
                    zeroed |= axes.contains(cga::Axes::E_PLUS); // get nullvector'd lmao
                    cga::Axes::E_PLUS
                }

                '-' => {
                    use_true_basis = true;
                    cga::Axes::E_MINUS
                }
                '+' => {
                    use_true_basis = true;
                    cga::Axes::E_PLUS
                }
                'E' => {
                    use_true_basis = true;
                    cga::Axes::E_PLANE
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
            if axes.contains(cga::Axes::E_PLANE) {
                return Err(LuaError::external(format!(
                    "cannot access component {string:?}",
                )));
            }
            if axes.contains(cga::Axes::E_MINUS) {
                nino = Some(NiNo::No);
            } else if axes.contains(cga::Axes::E_PLUS) {
                nino = Some(NiNo::Ni);
            }
            axes.remove(cga::Axes::E_PLANE);
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
impl<'lua> ToLua<'lua> for cga::Axes {
    fn to_lua(self, lua: LuaContext<'lua>) -> LuaResult<LuaValue<'lua>> {
        self.to_string().to_lua(lua)
    }
}

pub struct LuaMultivector(pub cga::Multivector);
impl<'lua> FromLua<'lua> for LuaMultivector {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> ::rlua::Result<Self> {
        lua_convert!(match (lua, &lua_value, "multivector") {
            <Float>(x) => Ok(Self(cga::Multivector::scalar(x))),
            <Vector>(v) => Ok(Self(v.into())),
            <cga::Multivector>(m) => Ok(Self(m)),
        })
    }
}

lua_wrapper_struct!(pub struct LuaVector(pub Vector), "vector");
lua_wrapper_struct!(pub struct LuaBlade(pub cga::Blade), "blade (multivector with consistent grade)");

lua_wrapper_enum!(
    #[name = "vector or number"]
    pub enum LuaVectorOrNumber {
        #[name = "vector"]
        Vec(Vector),
        #[name = "number"]
        Num(Float),
    }
);
