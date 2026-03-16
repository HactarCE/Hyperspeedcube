use hypermath::num::Euclid;
use hypuz_util::{ti::TiVec, typed_index_struct};

use super::*;

typed_index_struct! {
    /// Factor group that makes up a [`ProductGroup`].
    pub(super) struct FactorGroup(u8);
}

pub(super) type PerFactorGroup<T> = TiVec<FactorGroup, T>;

pub struct ProductGroup {
    /// Number of elements in the product group.
    element_count: usize,
    /// Factor groups.
    factors: PerFactorGroup<Box<dyn Send + Sync + Group>>,
    /// For each factor group: how much to multiply its [`GroupElementId`]s by
    /// to get the corresponding [`GroupElementId`]s in the product group.
    strides: PerFactorGroup<u32>,
    /// For each generator: the index of its factor group, and the
    /// [`GeneratorId`] within that group.
    generators_within_groups: PerGenerator<(FactorGroup, GeneratorId)>,
    /// For each geneartor: the [`GroupElementId`] in the product group.
    generators: PerGenerator<GroupElementId>,
}

impl ProductGroup {
    pub fn new(groups: impl IntoIterator<Item = Box<dyn Send + Sync + Group>>) -> Self {
        let factors = PerFactorGroup::from_iter(groups);

        let element_count = factors.iter_values().map(|g| g.element_count()).product();

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

        Self {
            element_count,
            factors,
            strides,
            generators_within_groups,
            generators,
        }
    }

    pub fn element_into_factors(
        &self,
        mut element: GroupElementId,
    ) -> impl Iterator<Item = GroupElementId> {
        self.factors.iter_values().map(move |g| {
            let (quotient, remainder) = element.0.div_rem_euclid(&(g.element_count() as u32));
            element.0 = quotient;
            GroupElementId(remainder)
        })
    }

    pub fn element_from_factors(
        &self,
        elements: impl IntoIterator<Item = GroupElementId>,
    ) -> GroupElementId {
        GroupElementId(
            std::iter::zip(elements, self.strides.iter_values())
                .map(|(e, stride)| e.0 * stride)
                .sum(),
        )
    }

    pub fn element_in_factor(
        &self,
        factor: FactorGroup,
        element: GroupElementId,
    ) -> GroupElementId {
        let group = &self.factors[factor];
        let group_stride = self.strides[factor];
        GroupElementId((element.0 / group_stride) % group.element_count() as u32)
    }

    pub fn replace_factor_element(
        &self,
        factor: FactorGroup,
        element: GroupElementId,
        modify_element_within_factor: impl FnOnce(GroupElementId) -> GroupElementId,
    ) -> GroupElementId {
        let group_stride = self.strides[factor];
        let old_factor_element = self.element_in_factor(factor, element);
        let new_factor_element = modify_element_within_factor(old_factor_element);
        GroupElementId(
            element.0 - (old_factor_element.0 * group_stride)
                + (new_factor_element.0 * group_stride),
        )
    }
}

impl Group for ProductGroup {
    fn element_count(&self) -> usize {
        self.element_count
    }
    fn generators(&self) -> &PerGenerator<GroupElementId> {
        &self.generators
    }

    fn factorization(&self, element: GroupElementId) -> Factorization<'_> {
        std::iter::zip(
            self.factors.iter_values(),
            self.element_into_factors(element),
        )
        .map(|(g, e)| g.factorization(e))
        .collect()
    }
    fn inverse(&self, element: GroupElementId) -> GroupElementId {
        self.element_from_factors(
            std::iter::zip(
                self.factors.iter_values(),
                self.element_into_factors(element),
            )
            .map(|(g, e)| g.inverse(e)),
        )
    }
    fn successor(&self, element: GroupElementId, generator: GeneratorId) -> GroupElementId {
        let (factor, generator_within_factor) = self.generators_within_groups[generator];
        self.replace_factor_element(factor, element, |element_within_factor| {
            self.factors[factor].successor(element_within_factor, generator_within_factor)
        })
    }
    fn predecessor(&self, element: GroupElementId, generator: GeneratorId) -> GroupElementId {
        let (factor, generator_within_factor) = self.generators_within_groups[generator];
        self.replace_factor_element(factor, element, |element_within_factor| {
            self.factors[factor].predecessor(element_within_factor, generator_within_factor)
        })
    }
    fn compose(&self, a: GroupElementId, b: GroupElementId) -> GroupElementId {
        self.element_from_factors(
            itertools::izip!(
                self.factors.iter_values(),
                self.element_into_factors(a),
                self.element_into_factors(b),
            )
            .map(|(factor_group, a_factor, b_factor)| factor_group.compose(a_factor, b_factor)),
        )
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    lazy_static::lazy_static! {
        static ref BIG_PRODUCT_GROUP: ProductGroup = {
            let g1 = FiniteCoxeterGroup::B(3).group().unwrap(); // cube (3D)
            let g2 = FiniteCoxeterGroup::I(10).group().unwrap(); // 10-gon (2D)
            let g3 = FiniteCoxeterGroup::A(5).group().unwrap(); // 5-simplex (5D)
            ProductGroup::new([g1, g2, g3].map(|g| Box::new(g) as Box<dyn Send + Sync + Group>))
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
            assert_eq!(a, g.element_from_factors(g.element_into_factors(a)));
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
