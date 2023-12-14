/// Set of conformally convex shapes in a space.
///
/// (The shapes are convex, not the set.)
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct AtomicPolytopeSet(pub Set64<AtomicPolytopeRef>);

impl fmt::Display for AtomicPolytopeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.iter().join(", "))
    }
}

impl fmt::Debug for AtomicPolytopeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self).finish()
    }
}

impl IntoIterator for AtomicPolytopeSet {
    type Item = AtomicPolytopeRef;

    type IntoIter = tinyset::set64::IntoIter<AtomicPolytopeRef>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl IntoIterator for &AtomicPolytopeSet {
    type Item = AtomicPolytopeRef;

    type IntoIter = tinyset::set64::IntoIter<AtomicPolytopeRef>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.clone().into_iter()
    }
}

impl Neg for AtomicPolytopeSet {
    type Output = Self;

    fn neg(self) -> Self::Output {
        AtomicPolytopeSet(self.into_iter().map(|shape_ref| -shape_ref).collect())
    }
}

impl BitOr for AtomicPolytopeSet {
    type Output = Self;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self.0.extend(rhs);
        self
    }
}

impl<T: Into<AtomicPolytopeRef>> From<T> for AtomicPolytopeSet {
    fn from(value: T) -> Self {
        AtomicPolytopeSet([value.into()].into_iter().collect())
    }
}

impl FromIterator<AtomicPolytopeRef> for AtomicPolytopeSet {
    fn from_iter<T: IntoIterator<Item = AtomicPolytopeRef>>(iter: T) -> Self {
        AtomicPolytopeSet(Set64::from_iter(iter))
    }
}

impl AtomicPolytopeSet {
    /// Constructs a new empty set.
    pub fn new() -> Self {
        AtomicPolytopeSet(Set64::new())
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
    pub fn insert(&mut self, shape_ref: AtomicPolytopeRef) {
        self.0.insert(shape_ref);
    }
    /// Adds every shape from an iterator to the set.
    pub fn extend(&mut self, iter: impl IntoIterator<Item = AtomicPolytopeRef>) {
        self.0.extend(iter);
    }

    /// Iterates over the shapes in the set.
    pub fn iter(&self) -> tinyset::set64::IntoIter<AtomicPolytopeRef> {
        self.into_iter()
    }
}

hypermath::impl_mul_sign!(impl Mul<Sign> for AtomicPolytopeSet);
hypermath::impl_mulassign_sign!(impl MulAssign<Sign> for AtomicPolytopeSet);
