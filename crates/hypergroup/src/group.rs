use std::{
    borrow::Cow,
    fmt,
    ops::Mul,
    sync::{Arc, LazyLock},
};

use hypermath::num::Euclid;
use hypuz_util::ti::{IndexOverflow, TypedIndex, TypedIndexIter};
use itertools::Itertools;

use super::*;

/// [Abstract] [finite group], represented as a product of **factor groups**.
/// Each factor group is represented using lookup tables.
///
/// - Elements are represented using [`GroupElementId`].
/// - Generators are represented using [`GeneratorId`].
///
/// [Abstract]: https://en.wikipedia.org/wiki/Group_theory#Abstract_groups
/// [finite group]: https://en.wikipedia.org/wiki/Finite_group
///
/// This type is reference-counted and thus cheap to clone.
#[derive(Clone)]
pub struct Group {
    inner: Arc<GroupInner>,
}

pub(crate) struct GroupInner {
    /// Number of elements in the product group.
    element_count: usize,
    /// For each generator: the [`GroupElementId`] in the product group.
    generators: PerGenerator<GroupElementId>,

    /// Factor groups.
    factors: PerFactorGroup<Arc<AbstractGroupLut>>,
    /// For each factor group: how much to multiply its [`GroupElementId`]s by
    /// to get the corresponding [`GroupElementId`]s in the product group.
    strides: PerFactorGroup<u32>,
    /// For each generator: the index of its factor group, and the
    /// [`GeneratorId`] within that group.
    generators_within_factors: PerGenerator<(FactorGroup, GeneratorId)>,
}

impl fmt::Debug for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Group")
            .field("label", &self.label())
            .field("element_count", &self.inner.element_count)
            .field("generators", &self.inner.generators)
            .field("factors", &self.inner.factors)
            .finish_non_exhaustive()
    }
}

impl Default for Group {
    fn default() -> Self {
        Self::trivial()
    }
}

impl Mul for &Group {
    type Output = GroupResult<Group>;

    fn mul(self, rhs: Self) -> Self::Output {
        Group::product([self, rhs])
    }
}

impl TryFrom<AbstractGroupLut> for Group {
    type Error = GroupError;

    fn try_from(value: AbstractGroupLut) -> Result<Self, Self::Error> {
        Self::from_factors([Arc::new(value)])
    }
}

impl Group {
    /// Returns the trivial group.
    pub fn trivial() -> Self {
        static TRIVIAL_GROUP: LazyLock<Group> =
            LazyLock::new(|| Group::from_factors([]).expect("error constructing trivial group"));

        TRIVIAL_GROUP.clone()
    }

    /// Constructs a group from factors.
    pub(crate) fn from_factors(
        factor_groups: impl IntoIterator<Item = Arc<AbstractGroupLut>>,
    ) -> GroupResult<Self> {
        let factors = PerFactorGroup::from_iter(factor_groups);

        let element_count = factors
            .iter_values()
            .map(|g| g.element_count())
            .try_fold(1, usize::checked_mul)
            .filter(|n| n.saturating_sub(1) <= GroupElementId::MAX.0 as usize)
            .ok_or(IndexOverflow::new::<GroupElementId>())?;

        let strides: PerFactorGroup<u32> = factors
            .iter_values()
            .scan(1, |stride, group| {
                let this_stride = *stride;
                *stride *= group.element_count() as u32;
                Some(this_stride)
            })
            .collect();

        let generators_within_groups: PerGenerator<(FactorGroup, GeneratorId)> = factors
            .iter()
            .flat_map(|(i, group)| group.generators().iter_keys().map(move |g| (i, g)))
            .collect();

        let generators: PerGenerator<GroupElementId> = factors
            .iter_values()
            .flat_map(|group| group.generators().iter_values().copied())
            .collect();

        Ok(Self {
            inner: Arc::new(GroupInner {
                element_count,
                generators,
                factors,
                strides,
                generators_within_factors: generators_within_groups,
            }),
        })
    }

    /// Constructs a group from a [direct product].
    ///
    /// [direct product]: https://en.wikipedia.org/wiki/Direct_product_of_groups
    pub fn product<'a>(factors: impl IntoIterator<Item = &'a Self>) -> GroupResult<Self> {
        Self::from_factors(
            factors
                .into_iter()
                .flat_map(|factor| factor.inner.factors.iter_values().cloned()),
        )
    }

    /// Constructs a group from a composition function.
    ///
    /// The composition function is called on every pair of group element and
    /// generator, in order. It must return either an existing element, or an
    /// element that is one more than the largest element that has been returned
    /// previously.
    ///
    /// # Example
    ///
    /// ```
    /// # use hypergroup::*;
    /// // Construct the group of integers modulo 12, using +3 and +4 as the generators
    /// let generators: PerGenerator<u8> = [3, 4].into_iter().collect();
    /// let mut element_values: PerGroupElement<u8> = [0].into_iter().collect(); // start with identity
    /// let mut value_to_element: std::collections::HashMap::<u8, GroupElementId> = element_values
    ///     .iter()
    ///     .map(|(id, &element_value)| (element_value, id))
    ///     .collect();
    /// let group = Group::from_compose_fn("integers modulo 12", 2, |e, g| {
    ///     let new_elem_value = (element_values[e] + generators[g]) % 12;
    ///     Ok(*value_to_element
    ///         .entry(new_elem_value)
    ///         .or_insert_with(|| element_values.push(new_elem_value).unwrap()))
    /// })
    /// .unwrap();
    /// assert_eq!(group.element_count(), 12);
    /// ```
    pub fn from_compose_fn(
        label: impl Into<Cow<'static, str>>,
        generator_count: usize,
        compose: impl FnMut(GroupElementId, GeneratorId) -> GroupResult<GroupElementId>,
    ) -> GroupResult<Self> {
        Self::from_factors([Arc::new(AbstractGroupLut::from_compose_fn(
            label,
            generator_count,
            compose,
        )?)])
    }

    /// Flattens a direct product of factor groups into a single factor group
    /// that represents the same product. [`GeneratorId`]s and
    /// [`GroupElementId`]s are compatible between the original group and the
    /// flattened group.
    ///
    /// Note: This enumerates all elements of the group, which may be very slow.
    pub fn flatten(&self) -> GroupResult<Self> {
        if self.inner.factors.len() <= 1 {
            Ok(self.clone())
        } else {
            Self::from_compose_fn(self.label(), self.generators().len(), |e, g| {
                Ok(self.compose_elem_generator(e, g))
            })
        }
    }

    /// Flattens the group using [`Group::flatten()`] and constructs a group
    /// action from a function that composes a generator with a point. The
    /// number of points must be known ahead of time.
    ///
    /// When possible, prefer constructing the direct product of group actions
    /// instead of the group action of a direct product to avoid flattening
    /// large groups.
    pub fn action<P: TypedIndex>(
        &self,
        point_count: usize,
        act: impl FnMut(GeneratorId, P) -> P,
    ) -> GroupResult<GroupAction<P>> {
        let combined_factor_group = self
            .flatten()?
            .direct_product_decomposition()
            .first()
            .map(Arc::clone)
            .unwrap_or_default();
        GroupAction::from_factors([Arc::new(AbstractGroupActionLut::from_fn(
            combined_factor_group,
            point_count,
            act,
        ))])
    }

    pub fn label(&self) -> String {
        self.inner
            .factors
            .iter_values()
            .map(|f| f.label())
            .join(" x ")
    }

    /// Returns the factor groups that this group is a direct product of.
    pub(crate) fn direct_product_decomposition(&self) -> &PerFactorGroup<Arc<AbstractGroupLut>> {
        &self.inner.factors
    }

    /// Returns the offset in [`GroupElementId`] for each subgroup.
    pub(crate) fn strides(&self) -> &PerFactorGroup<u32> {
        &self.inner.strides
    }

    /// Decomposes an element into a factor element for each factor group.
    ///
    /// This is the inverse of [`Self::element_from_factors()`].
    pub(crate) fn element_to_factors(
        &self,
        mut element: GroupElementId,
    ) -> impl Clone + DoubleEndedIterator<Item = GroupElementId> + ExactSizeIterator {
        self.inner.factors.iter_values().map(move |g| {
            let (quotient, remainder) = element.0.div_rem_euclid(&(g.element_count() as u32));
            element.0 = quotient;
            GroupElementId(remainder)
        })
    }

    /// Composes an element from a factor element for each factor group.
    ///
    /// This is the inverse of [`Self::element_into_factors()`].
    pub(crate) fn element_from_factors(
        &self,
        elements: impl IntoIterator<Item = GroupElementId>,
    ) -> GroupElementId {
        GroupElementId(
            std::iter::zip(elements, self.strides().iter_values())
                .map(|(e, stride)| e.0 * stride)
                .sum(),
        )
    }

    /// Projects `element` into a factor group.
    ///
    /// This is an optimized equivalent to the following code:
    ///
    /// ```ignore
    /// return group.element_into_factors(element).nth(factor.0 as usize);
    /// ```
    pub(crate) fn project_element_to_factor(
        &self,
        factor: FactorGroup,
        element: GroupElementId,
    ) -> GroupElementId {
        let group = &self.inner.factors[factor];
        let group_stride = self.strides()[factor];
        GroupElementId((element.0 / group_stride) % group.element_count() as u32)
    }

    /// Decomposes `element` into a factor element for each factor group,
    /// modifies the factor element corresponding to `factor`, and then composes
    /// the element back together.
    ///
    /// This is an optimized equivalent to the following code:
    ///
    /// ```ignore
    /// let mut factor_elements: PerFactorGroup<GroupElementId> = group.element_into_factors(element).collect();
    /// factor_elements[factor] = modify_element_within_factor(factor_elements[factor]);
    /// return group.element_from_factors(factor_elements);
    /// ```
    pub(crate) fn replace_factor_element(
        &self,
        element: GroupElementId,
        factor: FactorGroup,
        modify_element_within_factor: impl FnOnce(GroupElementId) -> GroupElementId,
    ) -> GroupElementId {
        let group_stride = self.strides()[factor];
        let old_factor_element = self.project_element_to_factor(factor, element);
        let new_factor_element = modify_element_within_factor(old_factor_element);
        GroupElementId(
            element.0 - (old_factor_element.0 * group_stride)
                + (new_factor_element.0 * group_stride),
        )
    }

    /// Returns the list of generators used to generate the group.
    pub fn generators(&self) -> &PerGenerator<GroupElementId> {
        &self.inner.generators
    }

    /// Returns the number of elements in the group.
    pub fn element_count(&self) -> usize {
        self.inner.element_count
    }

    /// Returns an iterator over the elements in the group.
    pub fn elements(&self) -> TypedIndexIter<GroupElementId> {
        GroupElementId::iter(self.element_count())
    }

    /// Returns the shortest factorization of `element` into generators. Ties
    /// are broken by lexicographical ordering.
    pub fn factorization(
        &self,
        element: GroupElementId,
    ) -> impl Clone + DoubleEndedIterator<Item = GeneratorId> {
        std::iter::zip(
            self.inner.factors.iter_values(),
            self.element_to_factors(element),
        )
        .flat_map(|(g, e)| g.factorization(e))
        .copied()
    }

    /// Returns the inverse of `element`.
    pub fn inverse(&self, element: GroupElementId) -> GroupElementId {
        self.element_from_factors(
            std::iter::zip(
                self.inner.factors.iter_values(),
                self.element_to_factors(element),
            )
            .map(|(g, e)| g.inverse(e)),
        )
    }

    /// Returns the factor group containing a generator and its ID within that
    /// factor group.
    pub(crate) fn generator_within_factor(&self, g: GeneratorId) -> (FactorGroup, GeneratorId) {
        self.inner.generators_within_factors[g]
    }

    /// Composes an element and a generator.
    ///
    /// This is an optimized equivalent to `group.compose(e, group.generators()[g])`.
    pub fn compose_elem_generator(&self, e: GroupElementId, g: GeneratorId) -> GroupElementId {
        let (factor, g_in_factor) = self.inner.generators_within_factors[g];
        self.replace_factor_element(e, factor, |e_in_factor| {
            self.inner.factors[factor].compose_elem_generator(e_in_factor, g_in_factor)
        })
    }

    /// Returns the composition `a * b`.
    pub fn compose(&self, a: GroupElementId, b: GroupElementId) -> GroupElementId {
        self.element_from_factors(
            itertools::izip!(
                self.inner.factors.iter_values(),
                self.element_to_factors(a),
                self.element_to_factors(b),
            )
            .map(|(factor_group, a_factor, b_factor)| factor_group.compose(a_factor, b_factor)),
        )
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn test_trivial_group() {
        let g = Group::trivial();
        assert_eq!(1, g.element_count());
        assert!(g.generators().is_empty());
        assert!(Arc::ptr_eq(&g.inner, &Group::trivial().inner))
    }

    lazy_static::lazy_static! {
        static ref BIG_PRODUCT_GROUP: Group = {
            let g1 = Coxeter::B(3).group().unwrap(); // cube (3D)
            let g2 = Coxeter::I(10).group().unwrap(); // 10-gon (2D)
            let g3 = Coxeter::A(5).group().unwrap(); // 5-simplex (5D)
            Group::product(&[g1, g2, g3]).unwrap()
        };
    }

    fn elem_strategy() -> impl Strategy<Value = GroupElementId> {
        (0..BIG_PRODUCT_GROUP.element_count() as u32).prop_map(GroupElementId)
    }

    proptest! {
        #[test]
        fn proptest_product_group_element_factors(
            a in elem_strategy(),
        ) {
            let g = &*BIG_PRODUCT_GROUP;
            assert_eq!(a, g.element_from_factors(g.element_to_factors(a)));
        }

        /// Tests a * (b * c) == (a * b) * c
        #[test]
        fn proptest_product_group_associativity(
            a in elem_strategy(),
            b in elem_strategy(),
            c in elem_strategy(),
        ) {
            let g = &*BIG_PRODUCT_GROUP;
            assert_eq!(
                g.compose(a, g.compose(b, c)),
                g.compose(g.compose(a, b), c),
            )
        }

        /// Tests a == a * ident == ident * a
        #[test]
        fn proptest_product_group_identity(
            a in elem_strategy(),
        ) {
            let g = &*BIG_PRODUCT_GROUP;
            assert_eq!(a, g.compose(a, GroupElementId::IDENTITY));
            assert_eq!(a, g.compose(GroupElementId::IDENTITY, a));
        }

        /// Tests a == (a^-1)^-1
        #[test]
        fn proptest_product_group_inverse(
            a in elem_strategy(),
        ) {
            let g = &*BIG_PRODUCT_GROUP;
            assert_eq!(a, g.inverse(g.inverse(a)));
        }

        /// Tests a^-1 b^-1 == (ba)^-1
        #[test]
        fn proptest_product_group_compose_inverse(
            a in elem_strategy(),
            b in elem_strategy(),
        ) {
            let g = &*BIG_PRODUCT_GROUP;
            assert_eq!(
                g.inverse(g.compose(b, a)),
                g.compose(g.inverse(a), g.inverse(b)),
            );
        }
    }
}
