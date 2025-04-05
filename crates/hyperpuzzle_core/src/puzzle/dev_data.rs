use std::hash::Hash;

use hypershape::GeneratorSequence;

use super::*;
use crate::NameSpec;

/// Extra information about how the puzzle was generated, for puzzle dev
/// purposes.
#[derive(Debug, Default, Clone)]
pub struct PuzzleDevData {
    /// Orbits used to generated various elements of the puzzle.
    pub orbits: Vec<DevOrbit<PuzzleElement>>,
}
impl PuzzleDevData {
    /// Generates an empty `PuzzleDevData` struct which may optionally be filled
    /// later.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Element of the puzzle.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PuzzleElement {
    /// Twist axis
    Axis(Axis),
    /// Puzzle color
    Color(Color),
}
impl PuzzleElement {
    /// Returns the name specification of the element on the given puzzle.
    pub fn name(self, puzzle: &Puzzle) -> Option<&NameSpec> {
        match self {
            PuzzleElement::Axis(axis) => puzzle.axis_names.get(axis).ok(),
            PuzzleElement::Color(color) => puzzle.colors.names.get(color).ok(),
        }
    }
}

/// Information about an orbit of puzzle elements.
#[derive(Debug, Clone)]
pub struct DevOrbit<T> {
    /// Human-readable string saying what kind of thing was generated.
    pub kind: &'static str,
    /// Each puzzle element, in the order they were generated in the orbit. Some
    /// elements may be missing.
    pub elements: Vec<Option<T>>,
    /// Generator sequence for each element in the orbit, in the order they were
    /// generated, including missing ones.
    pub generator_sequences: Vec<GeneratorSequence>,
}
impl<T> Default for DevOrbit<T> {
    fn default() -> Self {
        Self {
            kind: "unknown",
            elements: vec![],
            generator_sequences: vec![],
        }
    }
}
impl<T: Copy + Eq + Hash> DevOrbit<T> {
    /// Applies a function to every element in the orbit.
    #[must_use]
    pub fn map<U>(&self, mut f: impl FnMut(T) -> Option<U>) -> DevOrbit<U> {
        DevOrbit {
            kind: self.kind,
            elements: self.elements.iter().map(|&t| f(t?)).collect(),
            generator_sequences: self.generator_sequences.clone(),
        }
    }

    /// Returns a human-readable description for the orbit.
    pub fn description(&self) -> String {
        let kind = self.kind;
        let len = self.elements.len();
        let count = self.elements.iter().filter(|e| e.is_some()).count();
        if count == len {
            format!("{count} {kind}")
        } else {
            format!("{count}/{len} {kind}")
        }
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
