//! Data structures and algorithms for finite groups, specifically Coxeter
//! groups.

use std::{cell::Cell, collections::HashMap, fmt, rc::Rc};

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
        // contents. Instead we arrange the rows into a hashmap, indexed by the
        // elements on either side of the gap in that row.

        let n = self.generator_count() as usize;
        let mut g = GroupBuilder::new(n)?;

        let mut relation_table_headers = vec![];
        let mut relation_tables = RelationTables::new();
        for (j, b) in GeneratorId::iter(n).enumerate() {
            for (i, a) in GeneratorId::iter(j).enumerate() {
                let ab_order = self.coxeter_matrix_element(i as u8, j as u8);
                relation_table_headers.push(RelationTableHeader { a, b, ab_order });
            }
        }
        for h in &relation_table_headers {
            relation_tables.add_row(h.new_row(ElementId::IDENTITY));
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
                for h in &relation_table_headers {
                    relation_tables.add_row(h.new_row(new_element));
                }

                let mut facts = vec![SuccessorRelation {
                    element,
                    generator: gen,
                    result: new_element,
                }];
                // Continue updating tables until there are no more facts.
                while let Some(fact) = facts.pop() {
                    let new_facts = relation_tables.add_fact(fact, &mut g);
                    facts.extend(new_facts);
                }
            }

            element_id += 1;
        }

        g.build()
    }
}

#[derive(Debug, Default, Clone)]
struct RelationTables(HashMap<(ElementId, GeneratorId), Vec<Rc<Cell<RelationTableRow>>>>);
impl fmt::Display for RelationTables {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "RelationTables {{")?;
        for ((elem, gen), rows) in &self.0 {
            writeln!(f, "    ({elem} * {}): [", ElementId::from(*gen))?;
            for row in rows {
                writeln!(f, "        {},", row.get())?;
            }
            writeln!(f, "    ],")?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}
impl RelationTables {
    fn new() -> Self {
        RelationTables(HashMap::new())
    }

    fn add_row(&mut self, row: RelationTableRow) {
        self.add_existing_row(Rc::new(Cell::new(row)));
    }
    fn add_existing_row(&mut self, row: Rc<Cell<RelationTableRow>>) {
        let row_ref = row.get();
        let left_key = (row_ref.left_element, row_ref.left_generator());
        let right_key = (row_ref.right_element, row_ref.right_generator());
        self.0.entry(left_key).or_default().push(Rc::clone(&row));
        self.0.entry(right_key).or_default().push(row);
    }

    fn add_fact(
        &mut self,
        fact: SuccessorRelation,
        g: &mut GroupBuilder,
    ) -> Vec<SuccessorRelation> {
        let mut new_successor_relations = vec![];
        let key = (fact.element, fact.generator);
        if let Some(rows) = self.0.remove(&key) {
            for cell in rows {
                let mut row = cell.get();
                if row.is_complete() {
                    continue; // The row was already completed; discard it.
                }
                let optional_new_successor_relation = row.fill(g);
                cell.set(row);
                if let Some(new_relation) = optional_new_successor_relation {
                    // The row is now complete and we have discovered a new
                    // relation!
                    for r in [new_relation, new_relation.inverse()] {
                        if g.set_successor(r.element, r.generator, r.result) {
                            new_successor_relations.push(r);
                        }
                    }
                } else {
                    // The row is incomplete and must be added back.
                    self.add_existing_row(cell);
                }
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
    fn new_row(self, element: ElementId) -> RelationTableRow {
        RelationTableRow {
            generator_pair: [self.a, self.b],
            left_index: 0,
            right_index: self.ab_order * 2 - 1,
            left_element: element,
            right_element: element,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct RelationTableRow {
    generator_pair: [GeneratorId; 2],
    left_index: u8,
    right_index: u8,
    left_element: ElementId,
    right_element: ElementId,
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
    fn generator(&self, index: u8) -> GeneratorId {
        self.generator_pair[index as usize & 1]
    }
    fn left_generator(&self) -> GeneratorId {
        self.generator(self.left_index)
    }
    fn right_generator(&self) -> GeneratorId {
        self.generator(self.right_index)
    }

    /// Fills in as much of the table as possible. If the row is completely
    /// filled, returns the successor relation discovered by completing it. If
    /// this method returns `Some`, then the row can be discarded; if it returns
    /// `None`, then the row is incomplete and may have been modified.
    fn fill(&mut self, g: &GroupBuilder) -> Option<SuccessorRelation> {
        while self.left_index < self.right_index {
            if let Some(succ) = g.successor(self.left_element, self.left_generator()) {
                self.left_index += 1;
                self.left_element = succ;
            } else {
                break;
            }
        }

        while self.left_index < self.right_index {
            if let Some(pred) = g.predecessor(self.right_element, self.right_generator()) {
                self.right_index -= 1;
                self.right_element = pred;
            } else {
                break;
            }
        }

        self.is_complete().then(|| {
            let index = self.left_index; // same as `self.right_index`
            SuccessorRelation {
                element: self.left_element,
                generator: self.generator(index),
                result: self.right_element,
            }
        })
    }

    /// Returns whether the row has been completed and thus can be discarded.
    fn is_complete(&self) -> bool {
        self.left_index == self.right_index
    }
}

/// Representation of the fact that `element * generator = result`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct SuccessorRelation {
    element: ElementId,
    generator: GeneratorId,
    result: ElementId,
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
