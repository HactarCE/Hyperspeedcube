use std::collections::HashMap;

use hypuz_util::ti::{IndexOverflow, TiVec, TypedIndex};

#[derive(Debug, Clone)]
pub struct NameBiMap<I> {
    id_to_name: TiVec<I, String>,
    name_to_id: HashMap<String, I>,
}

impl<I: TypedIndex> NameBiMap<I> {
    pub fn new() -> Self {
        Self {
            id_to_name: TiVec::new(),
            name_to_id: HashMap::new(),
        }
    }

    pub fn concat(a: &Self, b: &Self) -> Self {
        let lift_a = |i: I| i;
        let lift_b = |i: I| I::try_from_index(i.to_index() + a.len()).expect("overflow");
        Self {
            id_to_name: std::iter::chain(a.id_to_name.iter_values(), b.id_to_name.iter_values())
                .cloned()
                .collect(),
            name_to_id: std::iter::chain(
                a.name_to_id
                    .iter()
                    .map(|(a_name, &a_index)| (a_name.clone(), lift_a(a_index))),
                b.name_to_id
                    .iter()
                    .map(|(b_name, &b_index)| (b_name.clone(), lift_b(b_index))),
            )
            .collect(),
        }
    }

    pub fn push(&mut self, name: String) -> Result<I, IndexOverflow> {
        let id = self.id_to_name.push(name.clone())?;
        self.name_to_id.insert(name, id);
        Ok(id)
    }

    pub fn len(&self) -> usize {
        self.id_to_name.len()
    }

    pub fn id_to_name(&self) -> &TiVec<I, String> {
        &self.id_to_name
    }

    pub fn name_to_id(&self, name: &str) -> Option<I> {
        self.name_to_id.get(name).copied()
    }
}
