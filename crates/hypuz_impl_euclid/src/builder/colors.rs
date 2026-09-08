use std::sync::Arc;

use eyre::Result;
use hyperpuzzle_core::{
    CatalogId, Color, ColorSystem, ComponentList, IndexOverflow, Names, Orbit, PaletteColor,
    TypedIndex,
};
use hypuz_notation::Str;
use hypuz_notation::family::SequentialLowercaseName;
use indexmap::IndexMap;
use itertools::Itertools;

/// Disjoint union of color systems.
#[derive(Debug, Clone)]
pub struct ColorSystemDisjointUnion {
    /// ID computed from `summand_ids`.
    pub id: CatalogId,
    pub terms: Vec<ColorSystemTerm>,
}

impl ColorSystemDisjointUnion {
    pub fn name(&self) -> String {
        crate::sum_name(self.terms.iter().map(|t| &t.name))
    }

    /// Constructs the empty color system, which is the identity of the disjoint
    /// union.
    pub fn disjoint_union_identity() -> Self {
        Self {
            id: crate::disjoint_union_id([].iter()),
            terms: vec![],
        }
    }

    /// Returns the disjoint union of two color systems.
    ///
    /// The result has a distinct color for each color in `self` and for each
    /// color in `rhs`.
    pub fn disjoint_union(&self, rhs: &Self) -> Result<Self> {
        if self.len() + rhs.len() >= Color::MAX_INDEX {
            return Err(IndexOverflow::new::<Color>().into());
        }
        Ok(Self {
            id: crate::disjoint_union_id(
                std::iter::chain(&self.terms, &rhs.terms)
                    .map(|t| &t.id)
                    .collect_vec()
                    .into_iter(),
            ),
            terms: crate::chain_cloned(&self.terms, &rhs.terms),
        })
    }

    /// Returns the number of colors.
    pub fn len(&self) -> usize {
        self.terms.iter().map(|t| t.len()).sum()
    }

    /// Constructs a disjoint union color system with exactly one summand.
    pub fn from_factor_color_system(color_system: Arc<ColorSystem>) -> Self {
        Self {
            id: color_system.id.clone(),
            terms: vec![ColorSystemTerm::from(&*color_system)],
        }
    }

    pub fn terms_with_prefixes(&self) -> Vec<(Str, &ColorSystemTerm)> {
        if let [t] = self.terms.as_slice() {
            vec![(Str::new(), t)]
        } else {
            self.terms
                .iter()
                .enumerate()
                .map(|(i, t)| (SequentialLowercaseName(i as u32).to_string().into(), t))
                .collect()
        }
    }

    pub fn build(&self) -> Result<Arc<ColorSystem>> {
        let terms = self.terms_with_prefixes();

        let names = Names::new_simple(
            terms
                .iter()
                .flat_map(|(prefix, t)| {
                    t.color_names
                        .iter()
                        .map(move |s| format!("{prefix}{s}").into())
                })
                .collect(),
        )?;

        // TODO: implement some fancy thing that distributes colors cleverly?
        //       e.g., 2x blue -> dark blue + light blue
        let scheme_name = hyperpuzzle_core::DEFAULT_COLOR_SCHEME_NAME;
        let scheme = if let [t] = self.terms.as_slice() {
            t.default_scheme.clone().into()
        } else {
            ColorSystem::new_rainbow_scheme(self.len())
        };

        let orbits = terms
            .iter()
            .flat_map(|(prefix, t)| t.orbits.iter().map(move |orbit| (prefix, orbit)))
            .map(|(prefix, orbit)| orbit.map(|s| names.lookup(&format!("{prefix}{s}"))))
            .collect();

        Ok(Arc::new(ColorSystem {
            id: self.id.clone(),
            name: self.name(),
            components: ComponentList::new(),
            names,
            schemes: IndexMap::from_iter([(scheme_name.to_string(), scheme)]),
            default_scheme: scheme_name.to_ascii_lowercase(),
            orbits,
        }))
    }
}

#[derive(Debug, Clone)]
struct ColorSystemTerm {
    id: CatalogId,
    name: String,
    color_names: Vec<Str>,
    default_scheme: Vec<PaletteColor>,
    orbits: Vec<Orbit<Str>>,
}

impl From<&ColorSystem> for ColorSystemTerm {
    fn from(color_system: &ColorSystem) -> Self {
        Self {
            id: color_system.id.clone(),
            name: color_system.name.clone(),
            color_names: color_system.names.list().to_vec(),
            // TODO: try to keep all color schemes
            default_scheme: color_system.default_scheme().to_vec(),
            orbits: color_system
                .orbits
                .iter()
                .map(|orbit| orbit.map(|&color| Some(color_system.names[color].clone())))
                .collect(),
        }
    }
}

impl ColorSystemTerm {
    pub fn len(&self) -> usize {
        self.color_names.len()
    }
}
