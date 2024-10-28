use std::str::FromStr;

use hypermath::pga::*;
use hypermath::prelude::*;

use super::*;

/// Conversion wrapper for a string or integer specifying a component of a
/// multivector.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LuaMultivectorIndex {
    /// Set of axes.
    pub axes: Axes,
    /// Sign to apply when reading/writing this term.
    pub sign: Sign,

    /// String description of the index.
    pub string: String,
}

impl LuaMultivectorIndex {
    /// Constructs a blade with a coefficient of 1 at this index, and no other
    /// values.
    pub fn to_multivector(&self, ndim: u8) -> Blade {
        Blade::from_term(
            ndim,
            Term {
                coef: self.sign.to_num(),
                axes: self.axes,
            },
        )
    }
}

impl FromLua for LuaMultivectorIndex {
    fn from_lua(lua_value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        if let Ok(LuaVectorIndex(i)) = LuaVectorIndex::from_lua(lua_value.clone(), lua) {
            Ok(LuaMultivectorIndex {
                axes: Axes::euclidean(i),
                sign: Sign::Pos,
                string: match lua.coerce_string(lua_value)? {
                    Some(s) => s.to_str()?.to_string(),
                    None => String::new(),
                },
            })
        } else {
            String::from_lua(lua_value, lua)?.parse()
        }
        .into_lua_err()
    }
}

impl FromStr for LuaMultivectorIndex {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let mut axes = Axes::empty();
        let mut sign = Sign::Pos;
        for c in string.chars() {
            let new_axis: Axes = match c {
                // Remember, Lua is 1-indexed so the X axis is 1.
                '0' => Axes::E0,
                '1' | 'x' | 'X' => Axes::X,
                '2' | 'y' | 'Y' => Axes::Y,
                '3' | 'z' | 'Z' => Axes::Z,
                '4' | 'w' | 'W' => Axes::W,
                '5' | 'v' | 'V' => Axes::V,
                '6' | 'u' | 'U' => Axes::U,
                '7' | 't' | 'T' => Axes::T,

                // Ignore these characters.
                's' | 'e' | 'n' | '_' | ' ' => continue,

                _ => return Err(format!("unknown axis {c:?}")),
            };
            let Some(new_sign) = Axes::sign_of_geometric_product(axes, new_axis) else {
                return Err(format!("component '{string}' is always zero"));
            };
            sign *= new_sign;
            axes ^= new_axis;
        }
        let string = string.to_owned();
        Ok(LuaMultivectorIndex { axes, sign, string })
    }
}
