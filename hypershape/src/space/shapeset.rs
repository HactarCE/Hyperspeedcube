use super::*;

/// Set of shapes in a space.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct ShapeSet(pub Set64<ShapeRef>);

impl fmt::Display for ShapeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.iter().join(", "))
    }
}

impl fmt::Debug for ShapeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self).finish()
    }
}

impl IntoIterator for ShapeSet {
    type Item = ShapeRef;

    type IntoIter = tinyset::set64::IntoIter<ShapeRef>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl IntoIterator for &ShapeSet {
    type Item = ShapeRef;

    type IntoIter = tinyset::set64::IntoIter<ShapeRef>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.clone().into_iter()
    }
}

impl Neg for ShapeSet {
    type Output = Self;

    fn neg(self) -> Self::Output {
        ShapeSet(self.into_iter().map(|shape_ref| -shape_ref).collect())
    }
}

impl BitOr for ShapeSet {
    type Output = Self;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self.0.extend(rhs);
        self
    }
}

impl From<ShapeRef> for ShapeSet {
    fn from(value: ShapeRef) -> Self {
        ShapeSet::from_iter([value])
    }
}

impl FromIterator<ShapeRef> for ShapeSet {
    fn from_iter<T: IntoIterator<Item = ShapeRef>>(iter: T) -> Self {
        ShapeSet(Set64::from_iter(iter))
    }
}

impl ShapeSet {
    /// Constructs a new empty set.
    pub fn new() -> Self {
        ShapeSet(Set64::new())
    }

    /// Returns whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Returns the number of shapes in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Adds a shape to the set. Duplicates are allowed as long as they have
    /// different signs.
    pub fn insert(&mut self, shape_ref: ShapeRef) {
        self.0.insert(shape_ref);
    }
    /// Adds every shape from an iterator to the set.
    pub fn extend(&mut self, iter: impl IntoIterator<Item = ShapeRef>) {
        self.0.extend(iter);
    }

    /// Iterates over the shapes in the set.
    pub fn iter(&self) -> tinyset::set64::IntoIter<ShapeRef> {
        self.into_iter()
    }
}

hypermath::impl_mul_sign!(impl Mul<Sign> for ShapeSet);
hypermath::impl_mulassign_sign!(impl MulAssign<Sign> for ShapeSet);
