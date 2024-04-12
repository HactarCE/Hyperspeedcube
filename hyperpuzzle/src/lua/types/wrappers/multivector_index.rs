use std::str::FromStr;

use hypermath::prelude::*;

use super::*;

/// Conversion wrapper for a string or integer specifying a component of a
/// multivector.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LuaMultivectorIndex {
    /// Which of nₒ or ∞ to the beginning of the axes, if either.
    pub nino: Option<NiNo>,

    pub axes: Axes,
    pub sign: Sign,

    pub string: String,
}

impl LuaMultivectorIndex {
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

impl<'lua> FromLua<'lua> for LuaMultivectorIndex {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if let Ok(LuaVectorIndex(i)) = LuaVectorIndex::from_lua(lua_value.clone(), lua) {
            Ok(LuaMultivectorIndex {
                nino: None,
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

                _ => return Err(format!("unknown axis {c:?}")),
            };
            sign *= axes * new_axis;
            axes ^= new_axis;
        }

        if use_true_basis && use_null_vector_basis {
            return Err("cannot mix true basis (e₋ e₊) with null vector basis (o ∞)".to_string());
        }

        if zeroed {
            return Err(format!("component '{string}' is always zero",));
        }

        let mut nino = None;
        if use_null_vector_basis {
            // We stored nₒ in `E_MINUS` and ∞ in `E_PLUS`.
            // nₒ and ∞ are each allowed, but not at the same time.
            if axes.contains(Axes::E_PLANE) {
                return Err(format!("cannot access component {string:?}",));
            }
            if axes.contains(Axes::E_MINUS) {
                nino = Some(NiNo::No);
            } else if axes.contains(Axes::E_PLUS) {
                nino = Some(NiNo::Ni);
            }
            axes.remove(Axes::E_PLANE);
        }

        let sign = Sign::from(sign);

        Ok(LuaMultivectorIndex {
            nino,
            axes,
            sign,
            string: string.to_owned(),
        })
    }
}

/// ∞ or nₒ.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NiNo {
    // nₒ
    No,
    /// ∞
    Ni,
}
