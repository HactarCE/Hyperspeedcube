use hypergroup::{AbbrGenSeq, IsometryGroup, PerGenerator};
use hypermath::{pga::Motor, prelude::*};
use hyperpuzzle_core::TypedIndex;
use hypuz_notation::Layer;

pub struct ProductPuzzleSpec {
    pub factors: Vec<FactorPuzzleSpec>,
}

pub struct FactorPuzzleSpec {
    pub symmetry: IsometryGroup,
    pub facet_orbits: Vec<FacetOrbitSpec>,
    pub axis_orbits: Vec<AxisOrbitSpec>,
}

impl FactorPuzzleSpec {
    /// Constructs the spec for a facet-turning puzzle.
    pub fn new_ft(symmetry: IsometryGroup, axis_orbits: Vec<AxisOrbitSpec>) -> Self {
        let facet_orbits = axis_orbits
            .iter()
            .map(|axis_orbit| axis_orbit.facets())
            .collect();

        Self {
            symmetry,
            facet_orbits,
            axis_orbits,
        }
    }

    pub fn ndim(&self) -> u8 {
        self.symmetry.ndim()
    }

    pub fn axis_count(&self) -> usize {
        self.axis_orbits.iter().map(|orbit| orbit.len()).sum()
    }
}

pub struct FacetOrbitSpec {
    pub initial_facet_pole: Vector,
    pub names: Vec<(AbbrGenSeq, String)>,
}

impl FacetOrbitSpec {
    /// Returns the generator sequence, pole vector, and name for each facet.
    pub fn named_facet_poles<'a>(
        &'a self,
        generators: &'a PerGenerator<Motor>,
        warn_fn: impl FnOnce(String),
    ) -> Vec<(Vector, &'a String)> {
        named_vectors(&self.initial_facet_pole, generators, &self.names, warn_fn)
    }

    /// Returns the axis orbit for a facet-turning puzzle.
    pub fn ft_axes(&self, cut_distances: Vec<Float>) -> AxisOrbitSpec {
        AxisOrbitSpec {
            initial_vector: self.initial_facet_pole.clone(),
            cut_distances,
            names: self.names.clone(),
        }
    }
}

pub struct AxisOrbitSpec {
    pub initial_vector: Vector,
    /// Cut distances from the origin, which must be sorted from outermost
    /// (greatest) to innermost (least).
    pub cut_distances: Vec<Float>,
    pub names: Vec<(AbbrGenSeq, String)>,
}

impl AxisOrbitSpec {
    /// Returns the number of axes in the orbit.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn layer_count(&self) -> usize {
        self.cut_distances.len().saturating_sub(1)
    }

    /// Returns the cut distance bounding the outside of layer, with an extra
    /// `None` at the end.
    pub fn layer_cut_distances(&self) -> impl Iterator<Item = (Option<Layer>, Float)> {
        Layer::iter(self.layer_count())
            .map(Some)
            .chain([None])
            .zip(self.cut_distances.iter().copied())
    }

    /// Returns the generator sequence, vector, and name for each axis.
    pub fn named_axis_vectors<'a>(
        &'a self,
        generators: &'a PerGenerator<Motor>,
        warn_fn: impl FnOnce(String),
    ) -> Vec<(Vector, &'a String)> {
        named_vectors(&self.initial_vector, generators, &self.names, warn_fn)
    }

    /// Returns the corresponding facet orbit for a facet-turning puzzle.
    pub fn facets(&self) -> FacetOrbitSpec {
        FacetOrbitSpec {
            initial_facet_pole: self.initial_vector.clone(),
            names: self.names.clone(),
        }
    }
}

fn named_vectors<'a>(
    initial_vector: &'a Vector,
    generators: &'a PerGenerator<Motor>,
    names: &'a [(AbbrGenSeq, String)],
    warn_fn: impl FnOnce(String),
) -> Vec<(Vector, &'a String)> {
    let index_to_gen_seq = hyperpuzzle_core::util::lazy_resolve(
        names
            .iter()
            .map(|(abbr_gen_seq, _)| (abbr_gen_seq.generators.clone(), abbr_gen_seq.end))
            .enumerate(),
        |gens1, gens2| std::iter::chain(&gens1.0, &gens2.0).copied().collect(),
        warn_fn,
    );

    names
        .iter()
        .enumerate()
        .map(move |(i, (_abbr_gen_seq, name))| {
            let motor = index_to_gen_seq[&i]
                .0
                .iter()
                .map(|&g| &generators[g])
                .fold(Motor::ident(0), |a, b| a * b);
            let transformed_vector = motor.transform(initial_vector);
            (transformed_vector, name)
        })
        .collect()
}
