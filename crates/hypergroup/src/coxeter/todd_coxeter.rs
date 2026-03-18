//! Implementation of the [Todd-Coxeter algorithm][algorithm].
//!
//! [algorithm]: https://en.wikipedia.org/wiki/Todd%E2%80%93Coxeter_algorithm
//!
//! This is a special-cased implementation of the Todd-Coxeter algorithm that
//! simply enumerates the elements of the group rather than enumerating cosets.
//!
//! # Implementation
//!
//! I first understood this algorithm by reading ["Building 4D Polytopes" by
//! Mikael Hvidtfeldt Christensen][building 4d polytopes]. (Search for the word "demo" in that
//! page.)
//!
//! [building 4d polytopes]: https://syntopia.github.io/Polytopia/polytopes.html
//!
//! Even after implementing this algorithm, I don't really know what they mean
//! by the "subgroup table," but the leftmost one in that demo is the coset
//! table (which we'll call the "successor table") and the others we'll call
//! "relation tables."
//!
//! We don't actually need to keep track of the whole relation tables -- just
//! the elements to the left and right of the gap in each row. Also, we don't
//! care which table the row comes from; just its header and contents.
//!
//! See [`RelationTables`] for more info about how that's structured and why.

use std::{borrow::Cow, fmt};

use hypuz_util::ti::TypedIndex;

use crate::{
    AbstractGroupLut, AbstractGroupLutBuilder, CoxeterMatrix, EggTable, GeneratorId,
    GroupElementId, GroupResult,
};

// TODO: support relations using techniques from https://github.com/Sonicpineapple/Discrete/blob/master/src/todd_coxeter.rs

pub fn construct_group(
    name: impl Into<Cow<'static, str>>,
    coxeter_matrix: &CoxeterMatrix,
) -> GroupResult<AbstractGroupLut> {
    let n = coxeter_matrix.mirror_count() as usize;
    let mut g = AbstractGroupLutBuilder::new(name, n)?;

    let mut relation_table_headers = vec![];
    // Set up a relation table for each possible pair of generators.
    for (j, b) in GeneratorId::iter(n).enumerate() {
        for (i, a) in GeneratorId::iter(j).enumerate() {
            let ab_order = coxeter_matrix.entries()[i][j];
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

/// See the comment at the start of [`FiniteCoxeterGroup::group()`] for an explanation
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
        self.0.add_element()
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
        g: &mut AbstractGroupLutBuilder,
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
    ab_order: u16,
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
    left_index: u16,
    /// Column of the element on the right side of the gap.
    right_index: u16,
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
    fn generator(self, index: u16) -> GeneratorId {
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
    fn fill(mut self, g: &AbstractGroupLutBuilder) -> Result<SuccessorRelation, Self> {
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
