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
    /// ID for the twist system.
    pub twists_id: CatalogId,

    /// Name for the puzzle.
    pub puzzle_name: String,
    /// Name for the twist system.
    pub twists_name: String,

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
        puzzle_name: &str,
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
            puzzle_id: puzzle_id.clone(),
            shape_id,
            twists_id: puzzle_id, // TODO
            puzzle_name: puzzle_name.to_string(),
            twists_name: puzzle_name.to_string(), // TODO

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
    /// Pole vector, name, and generator sequence for each facet in the orbit.
    pub named_facet_poles: Vec<(Vector, String, AbbrGenSeq)>,
}

impl FacetOrbitSpec {
    pub fn new(
        generators: &PerGenerator<Motor>,
        initial_facet_pole: Vector,
        names: Vec<(AbbrGenSeq, String)>,
        warn_fn: impl FnOnce(String),
    ) -> Self {
        Self {
            named_facet_poles: named_vectors(&initial_facet_pole, generators, names, warn_fn),
        }
    }

    /// Returns the axis orbit for a facet-turning puzzle.
    pub fn ft_axes(&self, adjacent_sets: Vec<(Vec<Str>, Float)>) -> AxisOrbitSpec {
        AxisOrbitSpec {
            named_axis_vectors: self.named_facet_poles.clone(),
            stabilizer_sets: adjacent_sets,
        }
    }
}

/// Specification for an orbit of named points in a [`FactorPuzzleSpec`].
#[derive(Debug, Clone)]
pub struct NamedPointOrbitSpec {
    /// Vector, name, and generator sequence for each named point in the orbit.
    pub named_point_vectors: Vec<(Vector, String, AbbrGenSeq)>,
}

impl NamedPointOrbitSpec {
    /// Constructs an orbit of named points.
    pub fn new(
        generators: &PerGenerator<Motor>,
        initial_vector: Vector,
        names: Vec<(AbbrGenSeq, String)>,
        warn_fn: impl FnOnce(String),
    ) -> Self {
        Self {
            named_point_vectors: named_vectors(&initial_vector, generators, names, warn_fn),
        }
    }

    /// Converts an orbit of named points into an orbit of axes.
    pub fn to_axes(&self, adjacent_sets: Vec<(Vec<Str>, Float)>) -> AxisOrbitSpec {
        let Self {
            named_point_vectors,
        } = self.clone();

        AxisOrbitSpec {
            named_axis_vectors: named_point_vectors.clone(),
            stabilizer_sets: adjacent_sets,
        }
    }

    /// Returns the number of named points in the orbit.
    #[allow(clippy::len_without_is_empty)] // should never be empty
    pub fn len(&self) -> usize {
        self.named_point_vectors.len()
    }
}

/// Specification for an orbit of axes in a [`FactorPuzzleSpec`].
#[derive(Debug, Clone)]
pub struct AxisOrbitSpec {
    /// Vector, name, and generator sequence for each axis in the orbit.
    pub named_axis_vectors: Vec<(Vector, String, AbbrGenSeq)>,
    /// Named points that can be stabilized to produce twists on the first axis.
    pub stabilizer_sets: Vec<(Vec<Str>, Float)>,
}

impl AxisOrbitSpec {
    /// Returns the number of axes in the orbit.
    #[allow(clippy::len_without_is_empty)] // should never be empty
    pub fn len(&self) -> usize {
        self.named_axis_vectors.len()
    }

    /// Returns the corresponding facet orbit for a facet-turning puzzle.
    pub fn facets(&self) -> FacetOrbitSpec {
        FacetOrbitSpec {
            named_facet_poles: self.named_axis_vectors.clone(),
        }
    }

    pub fn contains_name(&self, name: &str) -> bool {
        self.named_axis_vectors
            .iter()
            .any(|(_, axis_name, _)| axis_name == name)
    }
}

fn named_vectors<'a>(
    initial_vector: &'a Vector,
    generators: &'a PerGenerator<Motor>,
    names: Vec<(AbbrGenSeq, String)>,
    warn_fn: impl FnOnce(String),
) -> Vec<(Vector, String, AbbrGenSeq)> {
    let index_to_gen_seq = hyperpuzzle_core::util::lazy_resolve(
        names
            .iter()
            .map(|(abbr_gen_seq, _)| (abbr_gen_seq.generators.clone(), abbr_gen_seq.end))
            .enumerate(),
        |gens1, gens2| std::iter::chain(&gens1.0, &gens2.0).copied().collect(),
        warn_fn,
    );

    names
        .into_iter()
        .enumerate()
        .map(move |(i, (abbr_gen_seq, name))| {
            let motor = index_to_gen_seq[&i]
                .0
                .iter()
                .map(|&g| &generators[g])
                .fold(Motor::ident(0), |a, b| a * b);
            let transformed_vector = motor.transform(initial_vector);
            (transformed_vector, name, abbr_gen_seq)
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
