use std::ops::{BitAnd, BitOr, BitXor, Not};

use hypermath::{Hyperplane, TransformByMotor, VectorRef};

use super::*;

/// Region of space, typically defined by intersections, unions, and complements
/// of grips.
#[derive(Debug, Default, Clone)]
pub enum LuaRegion {
    /// Region containing nothing.
    #[default]
    Nothing,
    /// Region containing all of space.
    Everything,
    /// Region bounded by a hyperplane.
    HalfSpace(Hyperplane),
    /// Intersection of regions.
    And(Vec<LuaRegion>),
    /// Union of regions.
    Or(Vec<LuaRegion>),
    /// Symmetric difference of regions.
    Xor(Vec<LuaRegion>),
    /// Complement of a region.
    Not(Box<LuaRegion>),
}

impl FromLua for LuaRegion {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaUserData for LuaRegion {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("region"));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            this.lua_into_string()
        });

        methods.add_meta_method(LuaMetaMethod::BAnd, |_lua, lhs, rhs: Self| {
            Ok(lhs.clone() & rhs)
        });
        methods.add_meta_method(LuaMetaMethod::BOr, |_lua, lhs, rhs: Self| {
            Ok(lhs.clone() | rhs)
        });
        methods.add_meta_method(LuaMetaMethod::BXor, |_lua, lhs, rhs: Self| {
            Ok(lhs.clone() ^ rhs.clone())
        });
        methods.add_meta_method(LuaMetaMethod::BNot, |_lua, this, ()| Ok(!this.clone()));

        methods.add_method("contains", |_lua, this, LuaPoint(point)| {
            Ok(this.contains_point(&point))
        });
    }
}

impl LuaRegion {
    /// Returns the expected result of calling the Lua `tostring` function with
    /// `self`.
    pub fn lua_into_string(&self) -> LuaResult<String> {
        fn joined_string(regions: &[LuaRegion], op: char) -> LuaResult<String> {
            let mut ret = "(".to_string();
            let mut is_first = true;
            for r in regions {
                if is_first {
                    is_first = false;
                } else {
                    ret.push(' ');
                    ret.push(op);
                    ret.push(' ');
                }
                ret += &r.lua_into_string()?;
            }
            ret.push(')');
            Ok(ret)
        }

        Ok(match self {
            LuaRegion::Nothing => "'nothing' region".to_string(),
            LuaRegion::Everything => "'everything' region".to_string(),
            // LuaRegion::Grip { axis, layer_mask } => {
            //     let axis_string = axis.name().unwrap_or_else(|| axis.id.to_string());
            //     format!("{layer_mask}{axis_string}")
            // }
            LuaRegion::HalfSpace(boundary) => {
                format!("({boundary}).region")
            }
            LuaRegion::And(regions) => joined_string(regions, '&')?,
            LuaRegion::Or(regions) => joined_string(regions, '|')?,
            LuaRegion::Xor(regions) => joined_string(regions, '~')?,
            LuaRegion::Not(region) => format!("~{}", region.lua_into_string()?),
        })
    }

    /// Returns whether the region contains a point. If the point is
    /// approximately on the region boundary, it is considered inside the
    /// region.
    pub fn contains_point(&self, point: impl Copy + VectorRef) -> bool {
        match self {
            LuaRegion::Nothing => false,
            LuaRegion::Everything => true,
            LuaRegion::HalfSpace(h) => match h.location_of_point(point) {
                hypermath::PointWhichSide::On => true,
                hypermath::PointWhichSide::Inside => true,
                hypermath::PointWhichSide::Outside => false,
            },
            LuaRegion::And(xs) => xs.iter().all(|x| x.contains_point(point)),
            LuaRegion::Or(xs) => xs.iter().any(|x| x.contains_point(point)),
            LuaRegion::Xor(xs) => xs.iter().filter(|x| x.contains_point(point)).count() % 2 == 1,
            LuaRegion::Not(x) => !x.contains_point(point),
        }
    }
}

impl TransformByMotor for LuaRegion {
    fn transform_by(&self, m: &hypermath::pga::Motor) -> Self {
        match self {
            Self::Nothing => Self::Nothing,
            Self::Everything => Self::Everything,
            Self::HalfSpace(h) => Self::HalfSpace(m.transform(h)),
            Self::And(xs) => Self::And(xs.iter().map(|x| m.transform(x)).collect()),
            Self::Or(xs) => Self::Or(xs.iter().map(|x| m.transform(x)).collect()),
            Self::Xor(xs) => Self::Xor(xs.iter().map(|x| m.transform(x)).collect()),
            Self::Not(x) => Self::Not(Box::new(m.transform(x))),
        }
    }
}

impl BitAnd for LuaRegion {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Nothing, _) | (_, Self::Nothing) => Self::Nothing,
            (Self::Everything, other) | (other, Self::Everything) => other,
            (Self::And(mut xs), Self::And(ys)) => {
                xs.extend(ys);
                Self::And(xs)
            }
            (Self::And(mut xs), y) | (y, Self::And(mut xs)) => {
                xs.push(y);
                Self::And(xs)
            }
            (x, y) => Self::And(vec![x, y]),
        }
    }
}
impl BitOr for LuaRegion {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Everything, _) | (_, Self::Everything) => Self::Everything,
            (Self::Nothing, other) | (other, Self::Nothing) => other,
            (Self::Or(mut xs), Self::Or(ys)) => {
                xs.extend(ys);
                Self::Or(xs)
            }
            (Self::Or(mut xs), y) | (y, Self::Or(mut xs)) => {
                xs.push(y);
                Self::Or(xs)
            }
            (x, y) => Self::Or(vec![x, y]),
        }
    }
}
impl BitXor for LuaRegion {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (LuaRegion::Nothing, x) | (x, LuaRegion::Nothing) => x,
            (LuaRegion::Everything, x) | (x, LuaRegion::Everything) => !x,
            (LuaRegion::Xor(mut xs), LuaRegion::Xor(ys)) => {
                xs.extend(ys);
                Self::Xor(xs)
            }
            (LuaRegion::Xor(mut xs), x) | (x, LuaRegion::Xor(mut xs)) => {
                xs.push(x);
                Self::Xor(xs)
            }
            (x, y) => LuaRegion::Xor(vec![x, y]),
        }
    }
}
impl Not for LuaRegion {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Nothing => Self::Everything,
            Self::Everything => Self::Nothing,
            Self::Not(inner) => *inner,
            other => Self::Not(Box::new(other)),
        }
    }
}
