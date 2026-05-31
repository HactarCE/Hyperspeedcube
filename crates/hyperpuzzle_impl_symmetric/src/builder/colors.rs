use std::{borrow::Cow, sync::Arc};

use eyre::{OptionExt, Result, eyre};
use hyperpuzzle_core::{
    CatalogId, CatalogMetadata, Color, ColorSystem, Component, ComponentList, IndexOverflow,
    NameSpecBiMap, NameSpecBiMapBuilder, PaletteColor, PerColor, TypedIndex, catalog::BuildCtx,
    util::MaybeAdHoc,
};
use hypuz_notation::family::SequentialLowercaseName;
use indexmap::IndexMap;

/// Disjoint union of color systems.
#[derive(Debug, Clone)]
pub struct ColorSystemDisjointUnion {
    pub summand_ids: Vec<CatalogId>,
    pub summand_names: Vec<String>,

    /// List of names for each orbit of colors.
    ///
    /// Each orbit is assigned a [`SequentialLowercaseName`] prefix.
    pub orbits: Vec<Vec<Option<String>>>,
}

impl Component<ColorSystem> for ColorSystemDisjointUnion {}

impl ColorSystemDisjointUnion {
    /// Constructs the empty color system, which is the identity of the disjoint
    /// union.
    pub fn disjoint_union_identity() -> Self {
        Self {
            summand_ids: vec![],
            summand_names: vec![],
            orbits: vec![],
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
        Ok(Self {
            summand_ids: crate::chain_cloned(&self.summand_ids, &rhs.summand_ids),
            summand_names: crate::chain_cloned(&self.summand_names, &rhs.summand_names),
            orbits: crate::chain_cloned(&self.orbits, &rhs.orbits),
        })
    }

    /// Gets the [`ColorSystemDisjointUnion`] from a color system, or constructs
    /// a new one consisting of a single factor.
    pub fn from_color_system(c: &MaybeAdHoc<ColorSystem, Self>) -> Cow<'_, Self> {
        match c {
            MaybeAdHoc::Fixed(f) => match f.components.get_ref() {
                Ok(component) => Cow::Borrowed(component),
                Err(_) => Cow::Owned(Self {
                    summand_ids: vec![f.meta.id.clone()],
                    summand_names: vec![f.meta.name.clone()],
                    orbits: vec![
                        f.names
                            .iter_values()
                            .map(|s| Some(s.spec.clone()))
                            .collect(),
                    ],
                }),
            },
            MaybeAdHoc::AdHoc(a) => Cow::Borrowed(a),
        }
    }

    /// Returns the number of colors.
    pub fn len(&self) -> usize {
        self.orbits.iter().map(|orbit| orbit.len()).sum()
    }

    pub fn build(
        &self,
        build_ctx: &BuildCtx,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Arc<ColorSystem>> {
        build_ctx.set_building::<ColorSystem>();

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
                display_names.push(prefixed_name.unwrap_or_else(String::new));
            }
        }
        let names = names
            .build(color_id_counter.len())
            .ok_or_eyre("missing name for color")?; // error shouldn't be possible

        if autonames.next() != crate::autonames().next() {
            warn_fn(eyre!("color system is missing at least one name"));
        }

        let mut components = ComponentList::new();
        components.insert(Arc::new(self.clone()));

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
            meta: Arc::new(CatalogMetadata::simple(
                crate::sum_id(&self.summand_ids),
                crate::product_name(&self.summand_names),
            )),
            components,
            names,
            display_names,
            schemes,
            default_scheme,
            orbits: vec![], // TODO
        }))
    }
}
