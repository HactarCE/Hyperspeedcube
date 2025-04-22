use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

use hypershape::GeneratorSequence;
use itertools::Itertools;

use super::*;
use crate::NameSpec;

/// Orbit used to generate element of the puzzle, exposed to end users to help
/// with puzzle development.
#[derive(Debug, Clone)]
pub enum AnyOrbit {
    /// Orbit of twist axes.
    Axes(Orbit<Axis>),
    /// Orbit of colors.
    Colors(Orbit<Color>),
}
impl AnyOrbit {
    /// Returns a human-readable description for the orbit.
    pub fn description(&self) -> String {
        match self {
            AnyOrbit::Axes(orbit) => orbit.description(),
            AnyOrbit::Colors(orbit) => orbit.description(),
        }
    }
    /// Returns the index and name of each element, sorted by ID. The ID is not
    /// returned.
    pub fn sorted_ids_and_names(&self, puz: &Puzzle) -> Vec<(usize, String)> {
        match self {
            AnyOrbit::Axes(orbit) => orbit.sorted_ids_and_names(puz),
            AnyOrbit::Colors(orbit) => orbit.sorted_ids_and_names(puz),
        }
    }

    /// Returns the Lua source code to generate the given naming and ordering.
    pub fn lua_code(&self, new_names_and_order: &[(usize, String)], compact: bool) -> String {
        match self {
            AnyOrbit::Axes(orbit) => orbit.lua_code(new_names_and_order, compact),
            AnyOrbit::Colors(orbit) => orbit.lua_code(new_names_and_order, compact),
        }
    }
}

/// Element of a puzzle that can appear in a [`DevOrbit`].
pub trait PuzzleElement: fmt::Debug + Copy + Eq + Hash + Ord {
    /// Human-friendly plural noun for the element.
    const ELEMENT_STRING_PLURAL: &'static str = "axes";

    /// Returns the name specification of the element on the given puzzle.
    fn name(self, puzzle: &Puzzle) -> Option<&NameSpec>;
}
impl PuzzleElement for Axis {
    const ELEMENT_STRING_PLURAL: &'static str = "axes";

    fn name(self, puzzle: &Puzzle) -> Option<&NameSpec> {
        puzzle.axes().names.get(self).ok()
    }
}
impl PuzzleElement for Color {
    const ELEMENT_STRING_PLURAL: &'static str = "colors";

    fn name(self, puzzle: &Puzzle) -> Option<&NameSpec> {
        puzzle.colors.names.get(self).ok()
    }
}

/// Information about an orbit of puzzle elements.
///
/// This type is cheap to clone.
#[derive(Debug, Clone)]
pub struct Orbit<T> {
    /// Each puzzle element, in the order they were generated in the orbit. Some
    /// elements may be missing.
    pub elements: Arc<Vec<Option<T>>>,
    /// Generator sequence for each element in the orbit, in the order they were
    /// generated, including missing ones.
    pub generator_sequences: Arc<Vec<GeneratorSequence>>,
}
impl<T> Default for Orbit<T> {
    fn default() -> Self {
        Self {
            elements: Arc::new(vec![]),
            generator_sequences: Arc::new(vec![]),
        }
    }
}
impl<T: PuzzleElement> Orbit<T> {
    /// Applies a function to every element in the orbit.
    #[must_use]
    pub fn map<U>(&self, mut f: impl FnMut(T) -> Option<U>) -> Orbit<U> {
        Orbit {
            elements: Arc::new(self.elements.iter().map(|&t| f(t?)).collect()),
            generator_sequences: self.generator_sequences.clone(),
        }
    }

    /// Returns a human-readable description for the orbit.
    pub fn description(&self) -> String {
        let len = self.elements.len();
        let count = self.elements.iter().filter(|e| e.is_some()).count();
        if count == len {
            format!("{count} {}", T::ELEMENT_STRING_PLURAL)
        } else {
            format!("{count}/{len} {}", T::ELEMENT_STRING_PLURAL)
        }
    }

    /// Returns the index and name of each element, sorted by ID. The ID is not
    /// returned.
    pub fn sorted_ids_and_names(&self, puz: &Puzzle) -> Vec<(usize, String)> {
        self.elements
            .iter()
            .enumerate()
            .sorted_by_key(|(_, elem)| **elem)
            .filter_map(|(i, elem)| Some((i, elem.as_ref()?.name(&puz)?.spec.clone())))
            .collect()
    }

    /// Returns whether the orbit is completely empty. This is only really
    /// useful when using `DevOrbit::default()` to stand in for an empty value.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns the Lua source code to generate the given naming and ordering.
    pub fn lua_code(&self, new_names_and_order: &[(usize, String)], compact: bool) -> String {
        let mut new_element_names = vec![None; self.elements.len()];
        for (i, new_name) in new_names_and_order {
            if *i < new_element_names.len() {
                new_element_names[*i] = Some(new_name);
            }
        }

        let mut s = ":named({\n".to_owned();
        for (i, new_name) in new_names_and_order {
            s += "  ";
            s += &*crate::util::escape_lua_table_key(new_name);
            s += " = {";
            let mut is_first = true;
            let mut elem_index = *i;
            while let Some(gen_seq) = self.generator_sequences.get(elem_index) {
                for g in &gen_seq.generators {
                    if is_first {
                        is_first = false;
                    } else {
                        s += ", ";
                    }
                    s += &format!("{}", g + 1); // 1-indexed
                }
                let Some(next) = gen_seq.end else { break };
                elem_index = next;
                if compact {
                    if let Some(Some(other_name)) = new_element_names.get(elem_index) {
                        s += &format!(", {}", crate::util::lua_string_literal(other_name));
                        break;
                    }
                }
            }
            s += "},\n";
        }
        s += "})";
        s
    }
}
