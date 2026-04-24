use std::sync::Arc;

use hypermath::num::Euclid;
use hypuz_util::ti::TypedIndex;

use crate::{Group, GroupAction, GroupElementId, GroupResult, PerGenerator, PerGroupElement};

/// Subgroup of a [`GroupAction`].
///
/// This type is reference-counted and thus cheap to clone.
///
/// This mainly exists for use with [`crate::SubgroupConstraintSolver`].
#[derive(Debug, Clone)]
pub struct SubgroupAction<P> {
    overgroup_action: GroupAction<P>,
    pub(crate) subgroup_action: GroupAction<P>,
    factor_stride: u32,
    overgroup_factor_elem_count: u32,
    subgroup_factor_elem_count: u32,
    factor_overgroup_to_subgroup: Arc<PerGroupElement<Option<GroupElementId>>>,
    factor_subgroup_to_overgroup: Arc<PerGroupElement<GroupElementId>>,
}

impl<P: TypedIndex> SubgroupAction<P> {
    /// Constructs the trivial subgroup action.
    pub fn trivial() -> Self {
        Self {
            overgroup_action: GroupAction::trivial(),
            subgroup_action: GroupAction::trivial(),
            factor_stride: 1,
            overgroup_factor_elem_count: 1,
            subgroup_factor_elem_count: 1,
            factor_overgroup_to_subgroup: Arc::new(PerGroupElement::from_iter([Some(
                GroupElementId::IDENTITY,
            )])),
            factor_subgroup_to_overgroup: Arc::new(PerGroupElement::from_iter([
                GroupElementId::IDENTITY,
            ])),
        }
    }

    /// Constructs a subgroup action from subgroup generators.
    pub fn new(
        overgroup_action: &GroupAction<P>,
        subgroup_generators: &PerGenerator<GroupElementId>,
    ) -> GroupResult<Self> {
        const IDENT: GroupElementId = GroupElementId::IDENTITY;

        let overgroup = overgroup_action.group();

        let mut factor_overgroup_to_subgroup =
            PerGroupElement::new_with_len(overgroup.element_count());
        let mut factor_subgroup_to_overgroup = PerGroupElement::new();
        factor_overgroup_to_subgroup[IDENT] = Some(IDENT);
        factor_subgroup_to_overgroup.push(IDENT)?;

        let subgroup_label = format!("subgroup of {}", overgroup.label());
        let subgroup =
            Group::from_compose_fn(subgroup_label, subgroup_generators.len(), |e, g| {
                let product =
                    overgroup.compose(factor_subgroup_to_overgroup[e], subgroup_generators[g]);
                Ok(*match &mut factor_overgroup_to_subgroup[product] {
                    Some(product_in_subgroup) => product_in_subgroup,
                    product_in_subgroup @ None => {
                        product_in_subgroup.insert(factor_subgroup_to_overgroup.push(product)?)
                    }
                })
            })?;

        let subgroup_action = subgroup.action(overgroup_action.points().len(), |g, p| {
            Ok(overgroup_action.act(subgroup_generators[g], p))
        })?;

        Ok(Self {
            overgroup_action: overgroup_action.clone(),
            subgroup_action,
            factor_stride: 1,
            overgroup_factor_elem_count: overgroup.element_count() as u32,
            subgroup_factor_elem_count: subgroup.element_count() as u32,
            factor_overgroup_to_subgroup: Arc::new(factor_overgroup_to_subgroup),
            factor_subgroup_to_overgroup: Arc::new(factor_subgroup_to_overgroup),
        })
    }

    /// Constructs a subgroup action from a predicate function that returns
    /// whether an element is in the subgroup.
    pub fn from_subgroup_predicate(
        overgroup_action: &GroupAction<P>,
        mut is_elem_in_subgroup: impl FnMut(GroupElementId) -> bool,
    ) -> GroupResult<Self> {
        let mut subgroup_generators = PerGenerator::new();
        let mut ret = Self::new(overgroup_action, &subgroup_generators)?;
        for elem in overgroup_action.group().elements() {
            if ret.overgroup_to_subgroup(elem).is_none() && is_elem_in_subgroup(elem) {
                subgroup_generators.push(elem)?;
                ret = Self::new(overgroup_action, &subgroup_generators)?;
            }
        }
        Ok(ret)
    }

    pub(crate) fn overgroup_to_subgroup(
        &self,
        elem_in_overgroup: GroupElementId,
    ) -> Option<GroupElementId> {
        let (below, within, above) = factor_indices(
            elem_in_overgroup.0,
            self.factor_stride,
            self.overgroup_factor_elem_count,
        );
        let within = self.factor_overgroup_to_subgroup[GroupElementId(within)]?;
        Some(GroupElementId(unfactor_indices(
            (below, within.0, above),
            self.factor_stride,
            self.subgroup_factor_elem_count,
        )))
    }
    pub(crate) fn subgroup_to_overgroup(&self, elem_in_subgroup: GroupElementId) -> GroupElementId {
        let (below, within, above) = factor_indices(
            elem_in_subgroup.0,
            self.factor_stride,
            self.subgroup_factor_elem_count,
        );
        let within = self.factor_subgroup_to_overgroup[GroupElementId(within)];
        GroupElementId(unfactor_indices(
            (below, within.0, above),
            self.factor_stride,
            self.overgroup_factor_elem_count,
        ))
    }

    /// Returns the overgroup, which is the original group that this is a
    /// subgroup of.
    pub fn overgroup(&self) -> &Group {
        self.overgroup_action.group()
    }
    /// Returns the action of the overgroup.
    pub fn overgroup_action(&self) -> &GroupAction<P> {
        &self.overgroup_action
    }

    /// Returns the direct product of a group action with a subgroup action.
    ///
    /// Mathematically, this is just a direct product. There are two separate
    /// methods because [`GroupAction`] and [`SubgroupAction`] are separate
    /// types.
    pub fn direct_product_left(lhs: &GroupAction<P>, rhs: Self) -> GroupResult<Self> {
        Ok(Self {
            overgroup_action: GroupAction::product([lhs, &rhs.overgroup_action])?,
            subgroup_action: GroupAction::product([lhs, &rhs.subgroup_action])?,
            factor_stride: lhs.group().element_count() as u32 * rhs.factor_stride,
            ..rhs
        })
    }
    /// Returns the direct product of a subgroup action with a group action.
    ///
    /// Mathematically, this is just a direct product. There are two separate
    /// methods because [`GroupAction`] and [`SubgroupAction`] are separate
    /// types.
    pub fn direct_product_right(lhs: Self, rhs: &GroupAction<P>) -> GroupResult<Self> {
        Ok(Self {
            overgroup_action: GroupAction::product([&lhs.overgroup_action, rhs])?,
            subgroup_action: GroupAction::product([&lhs.subgroup_action, rhs])?,
            ..lhs
        })
    }
}

fn factor_indices(index: u32, factor_stride: u32, factor_elem_count: u32) -> (u32, u32, u32) {
    let (rest, below_factor) = index.div_rem_euclid(&factor_stride);
    let (above_factor, within_factor) = rest.div_rem_euclid(&factor_elem_count);
    (below_factor, within_factor, above_factor)
}

fn unfactor_indices(indices: (u32, u32, u32), factor_stride: u32, factor_elem_count: u32) -> u32 {
    let (below_factor, within_factor, above_factor) = indices;
    (above_factor * factor_elem_count + within_factor) * factor_stride + below_factor
}

#[cfg(test)]
mod tests {
    use hypermath::point;

    use crate::CoxeterMatrix;
    use crate::tests::{PerTestPoint, TestPoint};

    use super::*;

    #[test]
    fn test_factor_indices() {
        // 3 x 4 x 5
        for c in 0..5 {
            for b in 0..4 {
                for a in 0..3 {
                    let combined = a + b * 3 + c * 12;
                    assert_eq!((a, b, c), factor_indices(combined, 3, 4));
                    assert_eq!(combined, unfactor_indices((a, b, c), 3, 4));
                }
            }
        }
    }

    #[test]
    fn test_factor_subgroup_action() -> GroupResult<()> {
        let line = CoxeterMatrix::A(1)?.isometry_group()?;
        let triangle = CoxeterMatrix::A(2)?.isometry_group()?;
        let cube = CoxeterMatrix::B(3)?.isometry_group()?;

        let line_action =
            line.action_on_points(&PerTestPoint::from_iter([point![1.0], point![-1.0]]))?;
        let rot120 = hypermath::pga::Motor::from_angle_in_axis_plane(0, 1, 120.0_f64.to_radians());
        let triangle_action = triangle.action_on_points(&PerTestPoint::from_iter([
            point![0.0, 1.0],
            rot120.transform(&point![0.0, 1.0]),
            rot120.reverse().transform(&point![0.0, 1.0]),
        ]))?;
        let cube_action = cube.action_on_points(&PerTestPoint::from_iter([
            point![1.0, 0.0, 0.0],
            point![-1.0, 0.0, 0.0],
            point![0.0, 1.0, 0.0],
            point![0.0, -1.0, 0.0],
            point![0.0, 0.0, 1.0],
            point![0.0, 0.0, -1.0],
        ]))?;

        let fixed_point = TestPoint(3); // [0, -1, 0]

        let factor_subgroup_action = SubgroupAction::from_subgroup_predicate(&cube_action, |e| {
            cube_action.act(e, fixed_point) == fixed_point // stabilize
        })?;

        // cube
        assert_factor_subgroup_action_is_correct(
            &cube_action,
            &factor_subgroup_action,
            fixed_point,
        );

        let product_action =
            GroupAction::product([&line_action, &cube_action, &line_action, &triangle_action])?;
        let product_subgroup_action = {
            let cube_x_line =
                SubgroupAction::direct_product_right(factor_subgroup_action, &line_action)?;
            let line_x_cube_x_line =
                SubgroupAction::direct_product_left(&line_action, cube_x_line)?;
            let line_x_cube_x_line_x_triangle =
                SubgroupAction::direct_product_right(line_x_cube_x_line, &triangle_action)?;
            line_x_cube_x_line_x_triangle
        };
        let fixed_point_in_product = TestPoint(line_action.points().len() as u16 + fixed_point.0);

        // line x cube x line x triangle
        assert_eq!(1152, product_action.group().element_count());
        assert_factor_subgroup_action_is_correct(
            &product_action,
            &product_subgroup_action,
            fixed_point_in_product,
        );

        Ok(())
    }

    fn assert_factor_subgroup_action_is_correct(
        product_action: &GroupAction<TestPoint>,
        product_subgroup_action: &SubgroupAction<TestPoint>,
        fixed_point_in_product: TestPoint,
    ) {
        for e in product_action.group().elements() {
            if product_action.act(e, fixed_point_in_product) == fixed_point_in_product {
                let e_in_subgroup = product_subgroup_action.overgroup_to_subgroup(e).unwrap();
                assert_eq!(
                    e,
                    product_subgroup_action.subgroup_to_overgroup(e_in_subgroup)
                );
                assert_eq!(
                    fixed_point_in_product,
                    product_subgroup_action
                        .subgroup_action
                        .act(e_in_subgroup, fixed_point_in_product),
                );
            } else {
                assert_eq!(product_subgroup_action.overgroup_to_subgroup(e), None);
            }
        }
    }
}
