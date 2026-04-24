use hypergroup::{AbbrGenSeq, CoxeterMatrix, PerGenerator};
use hypermath::pga::Motor;
use hypermath::prelude::*;
use hyperpuzzle_core::{CatalogId, TypedIndex};
use hypuz_notation::{Layer, LayerRange, Str};

/// Specification for a puzzle product, which is defined in terms of puzzle
/// factors.
#[derive(Debug)]
pub struct ProductPuzzleSpec {
    /// Puzzle factors, which will be combined using direct product.
    pub factors: Vec<FactorPuzzleSpec>,
}

/// Specification for a factor of a [`ProductPuzzleSpec`].
#[derive(Debug)]
pub struct FactorPuzzleSpec {
    /// ID for the puzzle.
    pub puzzle_id: CatalogId,
    /// ID for the shape / color system.
    pub shape_id: CatalogId,

    /// Symmetry for the puzzle factor.
    // TODO: split axes symmetry and facets symmetry (requires expanding shape
    // symmetry before slicing)
    pub coxeter_matrix: CoxeterMatrix,
    /// Orbits of facets.
    ///
    /// Each facet is assigned a unique color.
    pub facet_orbits: Vec<FacetOrbitSpec>,
    /// Orbits of twist axes.
    pub axis_orbits: Vec<AxisOrbitSpec>,
    /// Orbits of named points.
    pub named_point_orbits: Vec<NamedPointOrbitSpec>,
    /// Orbits of named point sets, each with a gizmo pole distance.
    pub named_point_set_orbits: Vec<(Vec<Str>, f64)>,
}

impl FactorPuzzleSpec {
    /// Constructs the spec for a facet-turning puzzle.
    pub fn new_ft(
        puzzle_id: CatalogId,
        shape_id: CatalogId,
        coxeter_matrix: CoxeterMatrix,
        axis_orbits: Vec<AxisOrbitSpec>,
        named_point_orbits: Vec<NamedPointOrbitSpec>,
        named_point_set_orbits: Vec<(Vec<Str>, f64)>,
    ) -> Self {
        let facet_orbits = axis_orbits
            .iter()
            .map(|axis_orbit| axis_orbit.facets())
            .collect();

        Self {
            puzzle_id,
            shape_id,

            coxeter_matrix,
            facet_orbits,
            axis_orbits,
            named_point_orbits,
            named_point_set_orbits,
        }
    }
}

#[derive(Debug)]
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
    pub fn ft_axes(
        &self,
        cut_distances: Vec<Float>,
        adjacent_sets: Vec<(Vec<Str>, Float)>,
    ) -> AxisOrbitSpec {
        AxisOrbitSpec {
            initial_vector: self.initial_facet_pole.clone(),
            cut_distances,
            names: self.names.clone(),
            stabilizer_sets: adjacent_sets,
        }
    }
}

/// Specification for an orbit of named points in a [`FactorPuzzleSpec`].
#[derive(Debug, Clone)]
pub struct NamedPointOrbitSpec {
    /// Vector for the first named point in the orbit.
    pub initial_vector: Vector,
    /// Names for the named points, with associated generator sequences.
    pub names: Vec<(AbbrGenSeq, String)>,
}

impl NamedPointOrbitSpec {
    /// Constructs an orbit of named points.
    pub fn new(initial_vector: Vector, names: Vec<(AbbrGenSeq, String)>) -> Self {
        Self {
            initial_vector,
            names,
        }
    }

    /// Converts an orbit of named points into an orbit of axes.
    pub fn to_axes(
        &self,
        cut_distances: Vec<Float>,
        adjacent_sets: Vec<(Vec<Str>, Float)>,
    ) -> AxisOrbitSpec {
        let Self {
            initial_vector,
            names,
        } = self.clone();

        AxisOrbitSpec {
            initial_vector,
            cut_distances,
            names,
            stabilizer_sets: adjacent_sets,
        }
    }

    /// Returns the number of named points in the orbit.
    #[allow(clippy::len_without_is_empty)] // should never be empty
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Returns the generator sequence, vector, and name for each named point.
    pub fn named_point_vectors<'a>(
        &'a self,
        generators: &'a PerGenerator<Motor>,
        warn_fn: impl FnOnce(String),
    ) -> Vec<(Vector, &'a String)> {
        named_vectors(&self.initial_vector, generators, &self.names, warn_fn)
    }
}

/// Specification for an orbit of axes in a [`FactorPuzzleSpec`].
#[derive(Debug, Clone)]
pub struct AxisOrbitSpec {
    /// Vector for the first axis in the orbit.
    pub initial_vector: Vector,
    /// Cut distances from the origin, which must be sorted from outermost
    /// (greatest) to innermost (least).
    pub cut_distances: Vec<Float>,
    /// Names for the axes, with associated generator sequences.
    pub names: Vec<(AbbrGenSeq, String)>,
    /// Named points that can be stabilized to produce twists on the first axis.
    pub stabilizer_sets: Vec<(Vec<Str>, Float)>,
}

impl AxisOrbitSpec {
    /// Returns the number of axes in the orbit.
    #[allow(clippy::len_without_is_empty)] // should never be empty
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Returns the number of layers on each axis in the orbit.
    pub fn layer_count(&self) -> usize {
        self.cut_distances.len().saturating_sub(1)
    }

    /// Returns the cut distance bounding the outside of each layer, from
    /// outermost to innermost, with an extra `None` at the end.
    fn layer_outside_distances(&self) -> impl Iterator<Item = (Option<Layer>, Float)> {
        Layer::iter(self.layer_count())
            .map(Some)
            .chain([None])
            .zip(self.cut_distances.iter().copied())
    }

    /// Returns the layer range for a piece that spans from `min_distance` to
    /// `max_distance` along the axis vector.
    pub fn layer_range_for_distance_range(
        &self,
        max_distance: Float,
        min_distance: Float,
    ) -> Option<LayerRange> {
        // TODO: `None` should represent "not in any layer". blocking the axis completely is currently unrepresentable
        let (max_layer, _) = self
            .layer_outside_distances()
            .take_while(|(_, d)| APPROX.gt_eq(d, &max_distance))
            .last()?;
        let (min_layer, _) = self
            .layer_outside_distances()
            .take_while(|(_, d)| APPROX.gt(d, &min_distance))
            .last()?;
        Some(LayerRange::new(min_layer?, max_layer?))
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

/// Data for a named rotation of the entire polytope.
///
/// One of these automatically created for each axis orbit.
struct NamedRotationSpec {
    /// Set of axes that the rotation is named for.
    pub axis_names: Vec<Str>,
    /// Distance from the axis for the 4D twist gizmo.
    pub gizmo_pole_distance: f64,
}

impl NamedRotationSpec {
    pub fn new(axis_names: Vec<Str>, gizmo_pole_distance: f64) -> Self {
        Self {
            axis_names,
            gizmo_pole_distance,
        }
    }
}
