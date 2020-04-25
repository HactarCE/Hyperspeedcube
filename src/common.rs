use std::ops::{Add, Mul, Neg};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sign {
    Neg = -1,
    Zero = 0,
    Pos = 1,
}
impl Default for Sign {
    fn default() -> Self {
        Self::Zero
    }
}
impl From<TwistDirection> for Sign {
    fn from(direction: TwistDirection) -> Self {
        match direction {
            TwistDirection::CW => Self::Pos,
            TwistDirection::CCW => Self::Neg,
        }
    }
}
impl Neg for Sign {
    type Output = Self;
    fn neg(self) -> Self {
        match self {
            Self::Neg => Self::Pos,
            Self::Zero => Self::Zero,
            Self::Pos => Self::Neg,
        }
    }
}
impl Mul<Sign> for Sign {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        match self {
            Self::Neg => -rhs,
            Self::Zero => Self::Zero,
            Self::Pos => rhs,
        }
    }
}
impl Add<Sign> for Sign {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        match self {
            Self::Neg => match rhs {
                Self::Neg => panic!("Too negative"),
                Self::Zero => Self::Neg,
                Self::Pos => Self::Zero,
            },
            Self::Zero => rhs,
            Self::Pos => match rhs {
                Self::Neg => Self::Zero,
                Self::Zero => Self::Pos,
                Self::Pos => panic!("Too positive"),
            },
        }
    }
}
impl Sign {
    pub fn int(self) -> isize {
        match self {
            Self::Neg => -1,
            Self::Zero => 0,
            Self::Pos => 1,
        }
    }
    pub fn float(self) -> f32 {
        self.int() as f32
    }
    pub fn abs(self) -> usize {
        match self {
            Self::Neg | Self::Pos => 1,
            Self::Zero => 0,
        }
    }
    pub fn is_zero(self) -> bool {
        self == Self::Zero
    }
    pub fn is_nonzero(self) -> bool {
        self != Self::Zero
    }
    pub fn iter() -> impl Iterator<Item = &'static Self> {
        [Self::Neg, Self::Zero, Self::Pos].iter()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Color(usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TwistDirection {
    CW,
    CCW,
}
impl Default for TwistDirection {
    fn default() -> Self {
        Self::CW
    }
}
