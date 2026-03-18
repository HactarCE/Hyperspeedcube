//! Data structures and algorithms for finite groups, specifically Coxeter
//! groups.

use std::{fmt, sync::Arc};

use crate::{AbstractGroupLut, FactorGroupIsometries, Group, GroupResult, IsometryGroup};

use super::CoxeterMatrix;

/// [Finite Coxeter group](https://w.wiki/7PLd).
///
/// See also: [Coxeter-Dynkin diagram](https://w.wiki/7PLe)
#[expect(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Coxeter {
    A(u8),
    B(u8),
    D(u8),
    E6,
    E7,
    E8,
    F4,
    G2,
    H2,
    H3,
    H4,
    I(u16),
}

impl fmt::Display for Coxeter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Coxeter::A(n) => write!(f, "A{n}"),
            Coxeter::B(n) => write!(f, "B{n}"),
            Coxeter::D(n) => write!(f, "D{n}"),
            Coxeter::E6 => write!(f, "E6"),
            Coxeter::E7 => write!(f, "E7"),
            Coxeter::E8 => write!(f, "E8"),
            Coxeter::F4 => write!(f, "F4"),
            Coxeter::G2 => write!(f, "G2"),
            Coxeter::H2 => write!(f, "H2"),
            Coxeter::H3 => write!(f, "H3"),
            Coxeter::H4 => write!(f, "H4"),
            Coxeter::I(n) => write!(f, "I{n}"),
        }
    }
}

impl Coxeter {
    /// Returns the minimum number of generators for the group.
    pub fn generator_count(self) -> u8 {
        match self {
            Coxeter::A(n) => n,
            Coxeter::B(n) => n,
            Coxeter::D(n) => n,
            Coxeter::E6 => 6,
            Coxeter::E7 => 7,
            Coxeter::E8 => 8,
            Coxeter::F4 => 4,
            Coxeter::G2 => 2,
            Coxeter::H2 => 2,
            Coxeter::H3 => 3,
            Coxeter::H4 => 4,
            Coxeter::I(_) => 2,
        }
    }

    /// Returns an element of the group's [Coxeter matrix](https://w.wiki/7SNw).
    pub fn coxeter_matrix_element(self, mut i: u8, mut j: u8) -> u16 {
        // Ensure i <= j.
        if j < i {
            std::mem::swap(&mut i, &mut j);
        }

        assert!(j < self.generator_count(), "index out of range");

        // The diagonals of the matrix are always 1.
        if i == j {
            return 1;
        }

        match self {
            // Linear diagrams
            Coxeter::A(_) if j == 1 => 3, // (i, j) = (0, 1)
            Coxeter::B(_) if j == 1 => 4, // (i, j) = (0, 1)
            Coxeter::H2 | Coxeter::H3 | Coxeter::H4 if j == 1 => 5, /* (i, j) = (0, 1) */

            // Branched diagrams
            Coxeter::D(_) if i == 0 && j == 2 => 3,
            Coxeter::E6 | Coxeter::E7 | Coxeter::E8 if i == 0 && j == 3 => 3,

            // F4 (unique)
            Coxeter::F4 if j == 1 => 3, // (i, j) = (0, 1)
            Coxeter::F4 if i == 1 && j == 2 => 4,

            // 2D diagrams
            Coxeter::G2 => 6,
            Coxeter::I(n) => n,

            _ if i > 0 && i + 1 == j => 3,
            _ => 2, // no edge
        }
    }

    /// Returns the group's [Coxeter matrix](https://w.wiki/7SNw).
    pub fn matrix(self) -> CoxeterMatrix {
        CoxeterMatrix::from_matrix_index_fn(self.generator_count(), |i, j| {
            self.coxeter_matrix_element(i, j)
        })
        .expect("error constructing coxeter matrix")
    }

    fn abstract_group_lut(&self) -> GroupResult<AbstractGroupLut> {
        super::todd_coxeter::construct_group(self.to_string(), &self.matrix())
    }

    /// Constructs the full group structure.
    pub fn group(self) -> GroupResult<Group> {
        self.abstract_group_lut()?.try_into()
    }

    /// Constructs the full group structure with isometries.
    pub fn isometry_group(&self) -> GroupResult<IsometryGroup> {
        let group = self.abstract_group_lut()?;
        let mirror_generators = self.matrix().spherical_mirror_generators()?;
        let isometries =
            FactorGroupIsometries::from_generators_unchecked(&group, &mirror_generators);
        IsometryGroup::from_factors([(Arc::new(group), Arc::new(isometries))])
    }

    /// Constructs the full chiral group structure with isometries using an
    /// _O(n)_ algorithm, where _n_ is the order of the group.
    pub fn chiral_isometry_group(&self) -> GroupResult<IsometryGroup> {
        // TODO: add chiral_group()
        IsometryGroup::from_generators(
            format!("chiral {self}"),
            self.matrix().spherical_chiral_generators()?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todd_coxeter_algorithm() {
        #[track_caller]
        fn assert_group_order(g: Coxeter, expected_order: usize) {
            assert_eq!(g.group().unwrap().element_count(), expected_order);
        }

        assert_group_order(Coxeter::A(1), 2);
        assert_group_order(Coxeter::A(2), 6);
        assert_group_order(Coxeter::A(3), 24);
        assert_group_order(Coxeter::A(4), 120);
        assert_group_order(Coxeter::A(5), 720);

        assert_group_order(Coxeter::B(2), 8);
        assert_group_order(Coxeter::B(3), 48);
        assert_group_order(Coxeter::B(4), 384);
        // assert_group_order(CoxeterGroup::B(5), 3840); // 5D hypercube

        assert_group_order(Coxeter::D(4), 192);
        assert_group_order(Coxeter::D(5), 1920);

        assert_group_order(Coxeter::F4, 1152);

        assert_group_order(Coxeter::G2, 12);

        assert_group_order(Coxeter::H2, 10);
        assert_group_order(Coxeter::H3, 120);
    }
}
