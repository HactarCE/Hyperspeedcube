//! Data structures and algorithms for finite groups, specifically Coxeter
//! groups.

use std::fmt;

use hypermath::prelude::*;
use hypuz_util::ti::TypedIndex;

use super::{
    AbstractGroup, CoxeterGroup, EggTable, GeneratorId, GroupBuilder, GroupElementId, GroupResult,
};

/// [Finite Coxeter group](https://w.wiki/7PLd).
///
/// See also: [Coxeter-Dynkin diagram](https://w.wiki/7PLe)
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FiniteCoxeterGroup {
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
    I(u8), // Unless someone wants 256-agons, `u8` is enough.
}
impl FiniteCoxeterGroup {
    /// Returns the minimum number of generators for the group.
    pub fn generator_count(self) -> u8 {
        match self {
            FiniteCoxeterGroup::A(n) => n,
            FiniteCoxeterGroup::B(n) => n,
            FiniteCoxeterGroup::D(n) => n,
            FiniteCoxeterGroup::E6 => 6,
            FiniteCoxeterGroup::E7 => 7,
            FiniteCoxeterGroup::E8 => 8,
            FiniteCoxeterGroup::F4 => 4,
            FiniteCoxeterGroup::G2 => 2,
            FiniteCoxeterGroup::H2 => 2,
            FiniteCoxeterGroup::H3 => 3,
            FiniteCoxeterGroup::H4 => 4,
            FiniteCoxeterGroup::I(_) => 2,
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
            // Linear diagrams
            FiniteCoxeterGroup::A(_) if j == 1 => 3, // (i, j) = (0, 1)
            FiniteCoxeterGroup::B(_) if j == 1 => 4, // (i, j) = (0, 1)
            FiniteCoxeterGroup::H2 | FiniteCoxeterGroup::H3 | FiniteCoxeterGroup::H4 if j == 1 => 5, /* (i, j) = (0, 1) */

            // Branched diagrams
            FiniteCoxeterGroup::D(_) if i == 0 && j == 2 => 3,
            FiniteCoxeterGroup::E6 | FiniteCoxeterGroup::E7 | FiniteCoxeterGroup::E8
                if i == 0 && j == 3 =>
            {
                3
            }

            // F4 (unique)
            FiniteCoxeterGroup::F4 if j == 1 => 3, // (i, j) = (0, 1)
            FiniteCoxeterGroup::F4 if i == 1 && j == 2 => 4,

            // 2D diagrams
            FiniteCoxeterGroup::G2 => 6,
            FiniteCoxeterGroup::I(n) => n,

            _ if i > 0 && i + 1 == j => 3,
            _ => 2, // no edge
        }
    }
    /// Returns the group's [Coxeter matrix](https://w.wiki/7SNw).
    pub fn coxeter_matrix(self) -> Matrix {
        Matrix::from_fn(self.generator_count(), |i, j| {
            self.coxeter_matrix_element(i, j) as f64
        })
    }
    /// Returns the corresponding [`CoxeterGroup`]
    pub fn coxeter_group(self, basis: Option<Vec<Vector>>) -> GroupResult<CoxeterGroup> {
        CoxeterGroup::from_matrix_index_fn(
            self.generator_count(),
            |i, j| self.coxeter_matrix_element(i, j) as _,
            basis,
        )
    }

    /// Constructs the full group structure using an _O(n)_ algorithm, where _n_
    /// is order of the group.
    pub fn group(self) -> GroupResult<AbstractGroup> {
        // I first understood this algorithm using the interactive demo on this
        // page: https://syntopia.github.io/Polytopia/polytopes.html (search for
        // the word "demo")
        //
        // Even after implementing this algorithm, I don't really know what they
        // mean by the "subgroup table," but the leftmost one in that demo is
        // the coset table (which we'll call the "successor table") and the
        // others we'll call "relation tables."
        //
        // We don't actually need to keep track of the whole relation tables --
        // just the elements to the left and right of the gap in each row. Also,
        // we don't care which table the row comes from; just its header and
        // contents.
        //
        // See [`RelationTables`] for more info about how that's structured and
        // why.

        let n = self.generator_count() as usize;
        let mut g = GroupBuilder::new(n)?;

        let mut relation_table_headers = vec![];
        // Set up a relation table for each possible pair of generators.
        for (j, b) in GeneratorId::iter(n).enumerate() {
            for (i, a) in GeneratorId::iter(j).enumerate() {
                let ab_order = self.coxeter_matrix_element(i as u8, j as u8);
                relation_table_headers.push(RelationTableHeader { a, b, ab_order });
            }
        }
        // Add a row with the identity for each possible pair of generators.
        let mut relation_tables = RelationTables::new(n);
        for h in &relation_table_headers {
            relation_tables.add_row(h.new_row(GroupElementId::IDENTITY));
        }

        let mut element_id = 0;

        while element_id < g.element_count() {
            let element = GroupElementId::try_from_index(element_id)?;

            for generator in GeneratorId::iter(n) {
                // If we already know the result of `element * generator` then
                // skip this iteration.
                if g.successor(element, generator).is_some() {
                    continue;
                }

                // We've discovered a new element!
                let new_element = g.add_successor(element, generator)?;
                g.set_successor(new_element, generator, element); // Generators are self-inverse.

                // Add a row to the relation tables for each possible pair of
                // generators.
                relation_tables.add_element()?;
                for h in &relation_table_headers {
                    relation_tables.add_row(h.new_row(new_element));
                }

                let mut facts = vec![SuccessorRelation {
                    element,
                    generator,
                    result: new_element,
                }];
                // Continue updating tables until there are no more new facts.
                while let Some(fact) = facts.pop() {
                    let new_facts = relation_tables.update_with_fact(fact, &mut g);
                    facts.extend(new_facts);
                }
            }

            element_id += 1;
        }

        g.build()
    }
}

/// See the comment at the start of [`CoxeterGroup::group()`] for an explanation
/// of the role this structure takes in the Todd-Coxeter algorithm.
///
/// The most important thing about each row is the element+generator composition
/// that must be known in order to fill in more of that row. Each row has two
/// such pairs: the one that will shrink the gap from the left, and the one that
/// will shrink the gap from the right. So we arrange the rows into an array
/// indexed by an element+generator pair. Each value in the table consists of a
/// list of rows with that pair on the left side of its gap, and a list of rows
/// with that pair on the right side of its gap.
#[derive(Debug, Clone)]
struct RelationTables(EggTable<[Vec<RelationTableRow>; 2]>);
impl fmt::Display for RelationTables {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "RelationTables {{")?;
        for ((element, generator), [left, right]) in self.0.iter() {
            let generator = GroupElementId::from(generator);
            writeln!(f, "    ({element} * {generator}): {{")?;
            writeln!(f, "        left: [")?;
            for row in left {
                writeln!(f, "        {row},")?;
            }
            writeln!(f, "        ],")?;
            writeln!(f, "        right: [")?;
            for row in right {
                writeln!(f, "        {row},")?;
            }
            writeln!(f, "        ],")?;
            writeln!(f, "    }}")?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}
impl RelationTables {
    /// Constructs a new structure for storing relation tables.
    fn new(generator_count: usize) -> Self {
        RelationTables(EggTable::new(generator_count))
    }

    /// Makes the table aware of another element.
    fn add_element(&mut self) -> GroupResult<GroupElementId> {
        self.0.add_element([vec![], vec![]])
    }
    /// Adds a table row, indexed by both the left and right sides of its gap.
    fn add_row(&mut self, row: RelationTableRow) {
        self.add_row_left(row);
        self.add_row_right(row);
    }
    /// Adds a table row, indexed by the left side of its gap.
    fn add_row_left(&mut self, row: RelationTableRow) {
        self.0.get_mut(row.left_element, row.left_generator())[0].push(row);
    }
    /// Adds a table row, indexed by the right side of its gap.
    fn add_row_right(&mut self, row: RelationTableRow) {
        self.0.get_mut(row.right_element, row.right_generator())[1].push(row);
    }

    /// Makes the table aware of a new fact, updating any rows that were waiting
    /// for it.
    fn update_with_fact(
        &mut self,
        fact: SuccessorRelation,
        g: &mut GroupBuilder,
    ) -> Vec<SuccessorRelation> {
        let mut new_successor_relations = vec![];

        // Iterate over every row that was waiting on the fact for either the
        // left or right side of its gap.
        #[rustfmt::skip] enum SideOfGap { Left, Right }
        let [left_rows, right_rows] = std::mem::take(self.0.get_mut(fact.element, fact.generator));
        let left_rows = left_rows.into_iter().map(|row| (SideOfGap::Left, row));
        let right_rows = right_rows.into_iter().map(|row| (SideOfGap::Right, row));

        for (side, row) in itertools::chain!(left_rows, right_rows) {
            match row.fill(g) {
                // The row is now complete and we have discovered a
                // (potentially) new relation!
                Ok(new_relation) => {
                    for r in [new_relation, new_relation.inverse()] {
                        // Record this new relation in the group structure.
                        if g.set_successor(r.element, r.generator, r.result) {
                            // Also tell the caller about it so they can update
                            // the tables again with the new fact.
                            new_successor_relations.push(r);
                        }
                    }
                }

                // The row is incomplete and must be added back.
                Err(new_row) => match side {
                    SideOfGap::Left => self.add_row_left(new_row),
                    SideOfGap::Right => self.add_row_right(new_row),
                },
            }
        }

        new_successor_relations
    }
}

/// For each group element, the elements along the relations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct RelationTableHeader {
    a: GeneratorId,
    b: GeneratorId,
    ab_order: u8,
}

impl fmt::Display for RelationTableHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let generator_sequence = [self.a, self.b].repeat(self.ab_order as usize);
        write!(f, "RelationTableHeader({generator_sequence:?})")
    }
}

impl RelationTableHeader {
    /// Constructs a row in the relation table starting with `element`.
    fn new_row(self, element: GroupElementId) -> RelationTableRow {
        RelationTableRow {
            generator_pair: [self.a, self.b],
            left_index: 0,
            right_index: self.ab_order * 2 - 1,
            left_element: element,
            right_element: element,
        }
    }
}

/// Row in a relation table.
///
/// We don't need to store the whole row, just the elements on either side of
/// the gap.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(align(8))]
struct RelationTableRow {
    /// Pair of generators that alternate along the header of the table.
    generator_pair: [GeneratorId; 2],

    /// Column of the element on the left side of the gap.
    left_index: u8,
    /// Column of the element on the right side of the gap.
    right_index: u8,
    /// Element on the left side of the gap.
    left_element: GroupElementId,
    /// Element on the right side of the gap.
    right_element: GroupElementId,
}

impl fmt::Display for RelationTableRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut row = vec!["--".to_string(); self.right_index as usize + 1];
        row[self.left_index as usize] = self.left_element.to_string();
        row[self.right_index as usize] = self.right_element.to_string();
        write!(f, "{:?} ~ {}", self.generator_pair, row.join(", "))?;
        Ok(())
    }
}

impl RelationTableRow {
    /// Returns the generator for a given column in the table.
    fn generator(self, index: u8) -> GeneratorId {
        self.generator_pair[index as usize & 1]
    }
    /// Returns the generator for the column at the left side of the gap.
    fn left_generator(self) -> GeneratorId {
        self.generator(self.left_index)
    }
    /// Returns the generator for the column at the right side of the gap.
    fn right_generator(self) -> GeneratorId {
        self.generator(self.right_index)
    }

    /// Fills in as much of the row as possible. This method returns `Ok` if the
    /// row has been completely filled and thus can be discarded; it returns
    /// `Err` if the row is incomplete, and therefore must be kept.
    fn fill(mut self, g: &GroupBuilder) -> Result<SuccessorRelation, Self> {
        while self.left_index < self.right_index {
            if let Some(pred) = g.predecessor(self.right_element, self.right_generator()) {
                self.right_index -= 1;
                self.right_element = pred;
            } else {
                break;
            }
        }

        while self.left_index < self.right_index {
            if let Some(succ) = g.successor(self.left_element, self.left_generator()) {
                self.left_index += 1;
                self.left_element = succ;
            } else {
                break;
            }
        }

        if self.is_complete() {
            let index = self.left_index; // same as `self.right_index`
            Ok(SuccessorRelation {
                element: self.left_element,
                generator: self.generator(index),
                result: self.right_element,
            })
        } else {
            Err(self)
        }
    }

    /// Returns whether the row has been completed and thus can be discarded.
    fn is_complete(&self) -> bool {
        self.left_index == self.right_index
    }
}

/// Representation of the fact that `element * generator = result`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct SuccessorRelation {
    element: GroupElementId,
    generator: GeneratorId,
    result: GroupElementId,
}
impl SuccessorRelation {
    /// Returns the inverse of a successor relation, which is true iff
    /// `generator` is self-inverse.
    fn inverse(mut self) -> Self {
        std::mem::swap(&mut self.element, &mut self.result);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::super::Group;
    use super::*;

    #[test]
    fn test_todd_coxeter_algorithm() {
        #[track_caller]
        fn assert_group_order(g: FiniteCoxeterGroup, expected_order: usize) {
            assert_eq!(g.group().unwrap().element_count(), expected_order);
        }

        assert_group_order(FiniteCoxeterGroup::A(1), 2);
        assert_group_order(FiniteCoxeterGroup::A(2), 6);
        assert_group_order(FiniteCoxeterGroup::A(3), 24);
        assert_group_order(FiniteCoxeterGroup::A(4), 120);
        assert_group_order(FiniteCoxeterGroup::A(5), 720);

        assert_group_order(FiniteCoxeterGroup::B(2), 8);
        assert_group_order(FiniteCoxeterGroup::B(3), 48);
        assert_group_order(FiniteCoxeterGroup::B(4), 384);
        // assert_group_order(CoxeterGroup::B(5), 3840); // 5D hypercube

        assert_group_order(FiniteCoxeterGroup::D(4), 192);
        assert_group_order(FiniteCoxeterGroup::D(5), 1920);

        assert_group_order(FiniteCoxeterGroup::F4, 1152);

        assert_group_order(FiniteCoxeterGroup::G2, 12);

        assert_group_order(FiniteCoxeterGroup::H2, 10);
        assert_group_order(FiniteCoxeterGroup::H3, 120);
    }
}
