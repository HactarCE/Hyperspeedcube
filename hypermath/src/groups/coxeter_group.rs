//! Data structures and algorithms for finite groups, specifically Coxeter
//! groups.

use std::fmt;

use itertools::Itertools;

use crate::{IndexNewtype, Matrix};

use super::{AbstractGroup, ElementId, GeneratorId, GroupBuilder, GroupResult};

/// [Finite Coxeter group](https://w.wiki/7PLd).
///
/// See also: [Coxeter-Dynkin diagram](https://w.wiki/7PLe)
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CoxeterGroup {
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
    I(u8),
}
impl CoxeterGroup {
    /// Returns the minimum number of generators for the group.
    pub fn generator_count(self) -> u8 {
        match self {
            CoxeterGroup::A(n) => n,
            CoxeterGroup::B(n) => n,
            CoxeterGroup::D(n) => n,
            CoxeterGroup::E6 => 6,
            CoxeterGroup::E7 => 7,
            CoxeterGroup::E8 => 8,
            CoxeterGroup::F4 => 4,
            CoxeterGroup::G2 => 2,
            CoxeterGroup::H2 => 2,
            CoxeterGroup::H3 => 3,
            CoxeterGroup::H4 => 4,
            CoxeterGroup::I(_) => 2,
        }
    }
    /// Returns an element of the group's [Coxeter matrix](https://w.wiki/7SNw).
    pub fn coxeter_matrix_element(self, mut i: u8, mut j: u8) -> u8 {
        // Ensure i<j
        if j < i {
            std::mem::swap(&mut i, &mut j);
        }

        let n = self.generator_count();
        if j >= n {
            panic!("index out of range");
        }

        // The diagonals of the matrix are always 1
        if i == j {
            return 1;
        }

        match self {
            CoxeterGroup::A(_) if i + 1 == j => 3,

            CoxeterGroup::B(n) if i + 1 == j => 3 + (j + 1 == n) as u8,

            CoxeterGroup::D(n) if i + 1 == j && j + 1 < n => 3,
            CoxeterGroup::D(n) if i + 3 == n => 3,

            CoxeterGroup::E6 | CoxeterGroup::E7 | CoxeterGroup::E8 if i == 2 && j + 1 == n => 3,
            CoxeterGroup::E6 | CoxeterGroup::E7 | CoxeterGroup::E8 if i + 1 == j && j + 1 < n => 3,

            CoxeterGroup::F4 if i == 1 && j == 2 => 4,
            CoxeterGroup::F4 if i + 1 == j => 3,

            CoxeterGroup::G2 => 6,

            CoxeterGroup::H2 => 5,

            CoxeterGroup::H3 if j == 1 => 5, // (i, j) = (0, 1)
            CoxeterGroup::H3 if i == 1 => 3, // (i, j) = (1, 2)

            CoxeterGroup::H4 if j == 1 => 5, // (i, j) = (0, 1)
            CoxeterGroup::H4 if i + 1 == j => 3,

            CoxeterGroup::I(n) => n,

            _ => 2, // no edge
        }
    }
    /// Returns the group's [Coxeter matrix](https://w.wiki/7SNw).
    pub fn coxeter_matrix(self) -> Matrix {
        Matrix::from_fn(self.generator_count(), |i, j| {
            self.coxeter_matrix_element(i, j) as f64
        })
    }

    /// Constructs the full group structure using an _O(n)_ algorithm, where _n_
    /// is order of the group.
    pub fn group(self) -> GroupResult<AbstractGroup> {
        let n = self.generator_count() as usize;
        let mut g = GroupBuilder::new(n)?;

        let mut relation_tables = vec![];
        for (j, b) in GeneratorId::iter(n).enumerate() {
            for (i, a) in GeneratorId::iter(j).enumerate() {
                let coxeter_matrix_element = self.coxeter_matrix_element(i as u8, j as u8) as usize;
                let mut t = RelationTable::new([a, b].repeat(coxeter_matrix_element));
                t.add_element(ElementId::IDENTITY); // Add the identity.
                relation_tables.push(t);
            }
        }

        let mut element_id = 0;

        while element_id < g.element_count() {
            let element = ElementId::try_from_usize(element_id)?;

            for gen in GeneratorId::iter(n) {
                // If we already know the result of `element * gen` then skip
                // this iteration.
                if g.successor(element, gen).is_some() {
                    continue;
                }

                // We've discovered a new element!
                let new_element = g.add_successor(element, gen)?;
                g.set_successor(new_element, gen, element); // Generators are self-inverse.
                for t in &mut relation_tables {
                    t.add_element(new_element);
                }

                // Continue updating tables until there are no more facts.
                'update_tables: loop {
                    for table in &mut relation_tables {
                        let new_facts = table.fill(&g);
                        for fact in &new_facts {
                            g.set_successor(fact.element, fact.generator, fact.result);
                        }
                        if !new_facts.is_empty() {
                            continue 'update_tables;
                        }
                    }
                    break;
                }
            }

            element_id += 1;
        }

        g.build()
    }
}

/// For each group element, the elements along the relations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RelationTable {
    generator_sequence: Vec<GeneratorId>,
    rows: Vec<RelationTableRow>,
}
impl fmt::Display for RelationTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "RelationTable {{")?;

        let generator_sequence_string = self
            .generator_sequence
            .iter()
            .map(|gen| gen.to_string())
            .join(", ");
        writeln!(f, "    generator_sequence: [{generator_sequence_string}]")?;

        writeln!(f, "    rows: [")?;
        for row in &self.rows {
            let mut row_values = vec!["--".to_string(); self.generator_sequence.len()];
            if let Some(i) = row.left_index.checked_sub(1) {
                row_values[i as usize] = row.left_element.to_string();
            }
            row_values[row.right_index as usize] = row.right_element.to_string();
            writeln!(f, "        [{}],", row_values.join(", "))?;
        }
        writeln!(f, "    ]")?;

        writeln!(f, "}}")?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct RelationTableRow {
    left_index: u16,
    right_index: u16,
    left_element: ElementId,
    right_element: ElementId,
}
impl RelationTableRow {
    fn new(len: usize, element: ElementId) -> Self {
        RelationTableRow {
            left_index: 0,
            right_index: len as u16 - 1,
            left_element: element,
            right_element: element,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Fact {
    element: ElementId,
    generator: GeneratorId,
    result: ElementId,
}

impl RelationTable {
    fn new(sequence: Vec<GeneratorId>) -> Self {
        RelationTable {
            generator_sequence: sequence,
            rows: vec![],
        }
    }
    fn add_element(&mut self, element: ElementId) {
        self.rows.push(RelationTableRow::new(
            self.generator_sequence.len(),
            element,
        ));
    }
    fn fill(&mut self, g: &GroupBuilder) -> Vec<Fact> {
        let mut new_facts = vec![];

        self.rows.retain_mut(|row| {
            while row.left_index < row.right_index {
                let left_generator = self.generator_sequence[row.left_index as usize];
                if let Some(succ) = g.successor(row.left_element, left_generator) {
                    row.left_index += 1;
                    row.left_element = succ;
                } else {
                    break;
                }
            }

            while row.left_index < row.right_index {
                let right_generator = self.generator_sequence[row.right_index as usize];
                if let Some(pred) = g.predecessor(row.right_element, right_generator) {
                    row.right_index -= 1;
                    row.right_element = pred;
                } else {
                    break;
                }
            }

            if row.left_index < row.right_index {
                true
            } else {
                let index = row.left_index; // same as `row.right_index`
                new_facts.push(Fact {
                    element: row.left_element,
                    generator: self.generator_sequence[index as usize],
                    result: row.right_element,
                });
                false
            }
        });

        new_facts
    }
}

#[cfg(test)]
mod tests {
    use crate::groups::Group;

    use super::CoxeterGroup;

    #[test]
    fn test_todd_coxeter_algorithm() {
        #[track_caller]
        fn assert_group_order(g: CoxeterGroup, expected_order: usize) {
            assert_eq!(g.group().unwrap().element_count(), expected_order);
        }

        assert_group_order(CoxeterGroup::A(1), 2);
        assert_group_order(CoxeterGroup::A(2), 6);
        assert_group_order(CoxeterGroup::A(3), 24);
        assert_group_order(CoxeterGroup::A(4), 120);
        assert_group_order(CoxeterGroup::A(5), 720);

        assert_group_order(CoxeterGroup::B(2), 8);
        assert_group_order(CoxeterGroup::B(3), 48);
        assert_group_order(CoxeterGroup::B(4), 384);
        // assert_group_order(CoxeterGroup::B(5), 3840); // 5D hypercube

        assert_group_order(CoxeterGroup::D(4), 192);
        assert_group_order(CoxeterGroup::D(5), 1920);

        assert_group_order(CoxeterGroup::F4, 1152);

        assert_group_order(CoxeterGroup::G2, 12);

        assert_group_order(CoxeterGroup::H2, 10);
        assert_group_order(CoxeterGroup::H3, 120);
    }
}
