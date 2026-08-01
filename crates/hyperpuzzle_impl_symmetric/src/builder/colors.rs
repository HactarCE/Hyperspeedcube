use std::{borrow::Cow, sync::Arc};

use eyre::{OptionExt, Result, eyre};
use hyperpuzzle_core::{
    CatalogId, CatalogObject, Color, ColorSystem, Component, ComponentList, IndexOverflow,
    NameSpecBiMap, NameSpecBiMapBuilder, PaletteColor, PerColor, TypedIndex, catalog::BuildCtx,
    util::MaybeAdHoc,
};
use hypuz_notation::family::SequentialLowercaseName;
use indexmap::IndexMap;

/// Disjoint union of color systems.
#[derive(Debug, Clone)]
pub struct ColorSystemDisjointUnion {
    /// ID computed from `summand_ids`.
    pub id: CatalogId,
    pub summand_ids: Vec<CatalogId>,
    pub summand_names: Vec<String>,

    /// List of names for each orbit of colors.
    ///
    /// Each orbit is assigned a [`SequentialLowercaseName`] prefix.
    pub orbits: Vec<Vec<Option<String>>>,

    pub existing: Option<Arc<ColorSystem>>,
}

impl ColorSystemDisjointUnion {
    /// Constructs the empty color system, which is the identity of the disjoint
    /// union.
    pub fn disjoint_union_identity() -> Self {
        Self {
            id: crate::sum_id(&[]),
            summand_ids: vec![],
            summand_names: vec![],
            orbits: vec![],
            existing: None,
        }
    }

    /// Returns the disjoint union of two color systems.
    ///
    /// The result has a distinct color for each color in `self` and for each
    /// color in `rhs`.
    pub fn disjoint_union(&self, rhs: &Self) -> Result<Self, IndexOverflow> {
        if self.len() + rhs.len() >= Color::MAX_INDEX {
            return Err(IndexOverflow::new::<Color>());
        }
        let summand_ids: Vec<CatalogId> = crate::chain_cloned(&self.summand_ids, &rhs.summand_ids);
        Ok(Self {
            id: crate::sum_id(&summand_ids),
            summand_ids,
            summand_names: crate::chain_cloned(&self.summand_names, &rhs.summand_names),
            orbits: crate::chain_cloned(&self.orbits, &rhs.orbits),
            existing: match (self.len(), rhs.len()) {
                (0, _) => rhs.existing.clone(),
                (_, 0) => self.existing.clone(),
                _ => None,
            },
        })
    }

    /// Returns the number of colors.
    pub fn len(&self) -> usize {
        self.orbits.iter().map(|orbit| orbit.len()).sum()
    }

    /// Constructs a disjoint union color system with exactly one summand.
    pub fn from_color_system(color_system: Arc<ColorSystem>) -> Self {
        Self {
            id: color_system.id.clone(),
            summand_ids: vec![color_system.id.clone()],
            summand_names: vec![color_system.name.clone()],
            orbits: vec![
                color_system
                    .names
                    .iter_values()
                    .map(|name_spec| Some(name_spec.spec.clone()))
                    .collect(),
            ],
            existing: Some(color_system),
        }
    }

    pub fn build(
        &self,
        build_ctx: &BuildCtx,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Arc<ColorSystem>> {
        if let Some(existing) = &self.existing {
            return Ok(Arc::clone(existing));
        }

        let mut autonames = crate::autonames();

        let mut names = NameSpecBiMapBuilder::new();
        let mut display_names = PerColor::new();
        let mut color_id_counter = PerColor::new();
        for (i, orbit_names) in self.orbits.iter().enumerate() {
            let prefix = SequentialLowercaseName(i as u32);
            for name in orbit_names {
                let id = color_id_counter.push(())?;
                let prefixed_name = name
                    .as_ref()
                    .map(|s| {
                        if self.orbits.len() == 1 {
                            s.clone()
                        } else {
                            format!("{prefix}{s}")
                        }
                    })
                    .or_else(|| autonames.next());
                names.set(id, prefixed_name.clone())?;
                display_names.push(prefixed_name.unwrap_or_else(String::new))?;
            }
        }
        let names = names
            .build(color_id_counter.len())
            .ok_or_eyre("missing name for color")?; // error shouldn't be possible

        if autonames.next() != crate::autonames().next() {
            warn_fn(eyre!("color system is missing at least one name"));
        }

        let default_scheme = "Automatic".to_string();
        let mut schemes = IndexMap::new();
        schemes.insert(
            default_scheme.clone(),
            std::iter::repeat_n(
                PaletteColor::Gradient {
                    gradient_name: "Rainbow".to_string(),
                    index: 0,
                    total: 0,
                },
                names.len(),
            )
            .collect(),
        );

        Ok(Arc::new(ColorSystem {
            id: self.id.clone(),
            name: crate::product_name(&self.summand_names),
            components: ComponentList::new(),
            names,
            display_names,
            schemes,
            default_scheme,
            orbits: vec![], // TODO
        }))
    }
}
