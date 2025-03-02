use super::*;

/// Oriented memoized object in a [`Space`].
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct SignedRef<I> {
    /// Unoriented ID.
    pub id: I,
    /// Orientation.
    pub sign: Sign,
}
impl<I: fmt::Debug> fmt::Debug for SignedRef<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let SignedRef { id, sign } = self;
        write!(f, "{sign}{id:?}")
    }
}
impl<I: fmt::Display> fmt::Display for SignedRef<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let SignedRef { id, sign } = self;
        write!(f, "{sign}{id}")
    }
}
impl<I> From<I> for SignedRef<I> {
    fn from(id: I) -> Self {
        SignedRef {
            id,
            sign: Sign::Pos,
        }
    }
}
impl<I: Clone> From<&I> for SignedRef<I> {
    fn from(id: &I) -> Self {
        SignedRef {
            id: id.clone(),
            sign: Sign::Pos,
        }
    }
}
impl<I: Fits64> Fits64 for SignedRef<I> {
    unsafe fn from_u64(x: u64) -> Self {
        Self {
            // SAFETY: this is inverse of `to_u64()` and caller ensures that any
            //         inputs here come from `to_u64()`
            id: unsafe { I::from_u64(x >> 1) },
            sign: if x & 1 == 0 { Sign::Pos } else { Sign::Neg },
        }
    }

    fn to_u64(self) -> u64 {
        (self.id.to_u64() << 1) | self.sign as u64
    }
}
impl<I> Neg for SignedRef<I> {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        self.sign = -self.sign;
        self
    }
}
impl<I: Eq + Fits64> PartialOrd for SignedRef<I> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<I: Eq + Fits64> Ord for SignedRef<I> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_u64().cmp(&other.to_u64())
    }
}
hypermath::impl_mul_sign!(impl<I> Mul<Sign> for SignedRef<I>);
hypermath::impl_mulassign_sign!(impl<I: Clone> MulAssign<Sign> for SignedRef<I>);
