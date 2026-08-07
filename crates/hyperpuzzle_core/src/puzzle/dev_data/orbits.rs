use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

use hypergroup::AbbrGenSeq;
use hypuz_notation::Str;
use itertools::Itertools;

use super::*;

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
    /// Returns the name of the first non-null element in the orbit. Returns
    /// `"<unnamed>"` if no element exists and has a name.
    pub fn first_name(&self, puz: &Puzzle) -> Str {
        match self {
            AnyOrbit::Axes(orbit) => orbit.first_name(puz),
            AnyOrbit::Colors(orbit) => orbit.first_name(puz),
        }
        .unwrap_or_else(|| "<unnamed>".into())
    }
}

/// Element of a puzzle that can appear in an [`Orbit`].
pub trait PuzzleElement: fmt::Debug + Copy + Eq + Hash + Ord {
    /// Human-friendly plural noun for the element.
    const ELEMENT_STRING_PLURAL: &'static str = "axes";

    /// Returns the canonical name of the element on the given puzzle.
    fn name(self, puzzle: &Puzzle) -> Option<&Str>;
}
impl PuzzleElement for Axis {
    const ELEMENT_STRING_PLURAL: &'static str = "axes";

    fn name(self, puzzle: &Puzzle) -> Option<&Str> {
        puzzle.axes().names.get(self).ok()
    }
}
impl PuzzleElement for Color {
    const ELEMENT_STRING_PLURAL: &'static str = "colors";

    fn name(self, puzzle: &Puzzle) -> Option<&Str> {
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
    pub generator_sequences: Arc<Vec<AbbrGenSeq>>,
}
impl<T> Default for Orbit<T> {
    fn default() -> Self {
        Self {
            elements: Arc::new(vec![]),
            generator_sequences: Arc::new(vec![]),
        }
    }
}
impl<T> Orbit<T> {
    /// Returns the number of elements in the orbit.
    pub fn len(&self) -> usize {
        self.elements.len()
    }
    /// Returns whether the orbit is completely empty.
    ///
    /// An empty orbit should generally only occur when using
    /// `DevOrbit::default()` to stand in for an empty value.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Applies a function to every element in the orbit.
    #[must_use]
    pub fn map<U>(&self, mut f: impl FnMut(&T) -> Option<U>) -> Orbit<U> {
        Orbit {
            elements: Arc::new(self.elements.iter().map(|t| f(t.as_ref()?)).collect()),
            generator_sequences: self.generator_sequences.clone(),
        }
    }
}
impl<T: PuzzleElement> Orbit<T> {
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
            .filter_map(|(i, elem)| Some((i, elem.as_ref()?.name(puz)?.to_string())))
            .collect()
    }
    /// Returns the name of the first non-null element in the orbit.
    pub fn first_name(&self, puz: &Puzzle) -> Option<Str> {
        self.elements
            .iter()
            .find_map(|elem| Some(elem.as_ref()?.name(puz)?.clone()))
    }
}
