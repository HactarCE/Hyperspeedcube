use smallvec::{smallvec, SmallVec};
use std::ops::Index;

use crate::math::*;

#[derive(Debug, Clone)]
pub struct SymmetryGroup {
    generators: Vec<Rotoreflector>,
    elements: Vec<(Rotor, SmallVec<[Generator; 16]>)>,
}
impl Default for SymmetryGroup {
    fn default() -> Self {
        Self {
            generators: vec![],
            elements: vec![(Rotor::ident(), smallvec![])],
        }
    }
}
impl Index<Generator> for SymmetryGroup {
    type Output = Rotoreflector;

    fn index(&self, index: Generator) -> &Self::Output {
        &self.generators[index.0 as usize]
    }
}
impl SymmetryGroup {
    pub fn generator_count(&self) -> usize {
        self.generators.len()
    }
    pub fn nearest_orientation(&self, target: &Rotor) -> &(Rotor, SmallVec<[Generator; 16]>) {
        let reverse_target = target.reverse();
        util::min_by_f32_key(&self.elements, |(transform, _generator_sequence)| {
            // The scalar part of a rotor is the cosine of half the angle of
            // rotation. So we can use the absolute value of that quantity to
            // compare whether one rotor is a larger rotation than another.
            (transform * &reverse_target).s().abs()
        })
        .expect("symmetry group is empty")
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Generator(u8);

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct GroupElement(pub usize);
