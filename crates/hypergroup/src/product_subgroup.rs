use super::*;

/// Product of subgroups. There is exactly one subgroup for each factor
/// overgroup.
pub struct ProductSubgroup<'a> {
    pub(super) factors: PerFactorGroup<&'a Subgroup>,
}

impl<'a> ProductSubgroup<'a> {
    pub fn is_trivial(&self) -> bool {
        self.factors.iter_values().all(|f| f.is_trivial())
    }

    pub fn factors(&self) -> &PerFactorGroup<&'a Subgroup> {
        &self.factors
    }

    pub fn element_count(&self) -> usize {
        self.factors
            .iter_values()
            .map(|f| f.element_count())
            .product()
    }
}
