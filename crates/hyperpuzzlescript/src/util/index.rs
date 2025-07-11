use crate::{FromValueRef, Result, Type, Value};

/// Integer indexing from the front or back of a collection.
pub enum FrontBackIndex {
    /// Index from the front of a collection (positive).
    Front(usize),
    /// Index from the back of a collection (negative).
    Back(usize),
}
impl_ty!(FrontBackIndex = Type::Int);
impl<'a> FromValueRef<'a> for FrontBackIndex {
    fn from_value_ref(value: &'a Value) -> Result<Self> {
        Ok(Self::from(value.ref_to::<i64>()?))
    }
}
impl From<i64> for FrontBackIndex {
    fn from(value: i64) -> Self {
        match value {
            i @ 0.. => Self::Front(usize::try_from(i).unwrap_or(usize::MAX)),
            i @ ..0 => Self::Back(usize::try_from(-1 - i).unwrap_or(usize::MAX)),
        }
    }
}
impl FrontBackIndex {
    /// Returns the bounds for a collection with length `len` that allows
    /// negative indexing.
    pub(crate) fn bounds(len: usize) -> Option<(i64, i64)> {
        let max = len.checked_sub(1)? as i64;
        let min = (-1_i64).saturating_sub(max);
        Some((min, max))
    }
}
