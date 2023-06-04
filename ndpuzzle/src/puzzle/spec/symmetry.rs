use anyhow::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::math::*;
use crate::schlafli::SchlafliSymbol;

use super::MathExpr;

/// Specification for a set of symmeties.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum SymmetrySpec {
    /// Shorthand for a single Schlafli symbol.
    Schlafli(String),
    /// List of symmetries.
    List(SymmetrySpecList),
}
impl Default for SymmetrySpec {
    fn default() -> Self {
        Self::List(SymmetrySpecList::default())
    }
}
impl SymmetrySpec {
    /// Returns a list of generators for the symmetry. This set may not be
    /// minimal and may contain duplicates.
    pub fn generators(&self) -> Result<Vec<Rotoreflector>> {
        match self {
            SymmetrySpec::Schlafli(s) => SymmetrySpecListEntry::Schlafli(s.clone()).generators(),
            SymmetrySpec::List(list) => list
                .0
                .iter()
                .map(|sym| sym.generators())
                .flatten_ok()
                .collect(),
        }
    }

    /// Multiplies each element of the symmetry group by each seed and returns
    /// the list of unique results.
    pub fn generate<T>(
        &self,
        seeds: impl IntoIterator<Item = T>,
        transform: impl Fn(&Rotoreflector, &T) -> T,
    ) -> Result<Vec<(Rotoreflector, T)>>
    where
        T: approx::AbsDiffEq<Epsilon = Float>,
    {
        let generators = self.generators()?;
        let mut ret = seeds
            .into_iter()
            .map(|seed| (Rotoreflector::ident(), seed))
            .collect_vec();
        let mut unprocessed_idx = 0;
        while unprocessed_idx < ret.len() {
            for gen in &generators {
                let old = &ret[unprocessed_idx];
                let new = transform(gen, &old.1);
                if !ret.iter().any(|old| approx_eq(&old.1, &new)) {
                    ret.push((gen * &old.0, new));
                }
            }
            unprocessed_idx += 1;
        }
        Ok(ret)
    }
}

/// List of symmetry group specifications.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(transparent)]
pub struct SymmetrySpecList(pub Vec<SymmetrySpecListEntry>);
impl From<SymmetrySpec> for SymmetrySpecList {
    fn from(value: SymmetrySpec) -> Self {
        match value {
            SymmetrySpec::Schlafli(s) => SymmetrySpecList(vec![SymmetrySpecListEntry::Schlafli(s)]),
            SymmetrySpec::List(list) => list,
        }
    }
}

/// Specification for a single symmetry group.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SymmetrySpecListEntry {
    /// Schlafli symbol describing a symmetry group.
    Schlafli(String),
    /// Transformation describing a single generator for a symmetry group.
    Transform(MathExpr),
}
impl SymmetrySpecListEntry {
    /// Returns a list of generators for the symmetry group, which may include
    /// duplicates.
    fn generators(&self) -> Result<Vec<Rotoreflector>> {
        match self {
            Self::Schlafli(string) => Ok(SchlafliSymbol::from_string(string).generators()),
            Self::Transform(expr) => todo!(),
        }
    }
}
