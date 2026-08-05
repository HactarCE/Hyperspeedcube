use eyre::{OptionExt, Result, bail};
use hypergroup::{AbbrGenSeq, CoxeterMatrix, PerGenerator};
use hypermath::pga::Motor;
use hypermath::prelude::*;
use hyperpuzzle_core::CatalogId;
use hypuz_notation::Str;
use hypuz_notation::charsets::CharSet;

/// Specification for a puzzle product, which is defined in terms of puzzle
/// factors.
#[derive(Debug)]
pub struct ProductPuzzleSpec {
    /// Puzzle factors, which will be combined using direct product.
    pub factors: Vec<FactorPuzzleSpec>,
}

/// Specification for a twist system factor.
pub struct FactorTwistSystemSpec {
    pub ndim: u8,
    pub coxeter_matrix: Option<CoxeterMatrix>,
    pub axis_orbits: Vec<AxisOrbitSpec>,
    pub named_point_orbits: Vec<SimpleOrbitSpec>,
    pub named_point_set_orbits: Vec<NamedPointSetOrbitSpec>,
    pub stabilizer_twist_orbits: Vec<StabilizerTwistOrbitSpec>,
}

/// Specification for an orbit of named point sets in a
/// [`FactorTwistSystemSpec`].
///
/// This struct only contains info about one named point set in the orbit; the
/// orbit is generated using the grip group.
pub struct NamedPointSetOrbitSpec {
    /// Names of the named points in the set.
    pub named_points: Vec<Str>,
    /// Gizmo pole distance for stabilizer twists using the named point set.
    pub gizmo_pole_distance: Float,
}

/// Specification for an orbit of twists in a [`FactorTwistSystemSpec`].
///
/// This struct only contains info about one twist in the orbit; the orbit is
/// generated using the grip group.
pub struct StabilizerTwistOrbitSpec {
    /// Name of the axis to twist.
    pub axis_name: Str,
    /// Names of the named points which remain stabilized by the axis.
    pub named_points: Vec<Str>,
    /// Gizmo pole distance for the twist.
    pub gizmo_pole_distance: Float,
}

/// Specification for a factor of a [`ProductPuzzleSpec`].
#[derive(Debug)]
#[deprecated]
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
    /// Orbits of facets, identified by their facet pole.
    ///
    /// Each facet is assigned a unique color.
    pub facet_orbits: Vec<SimpleOrbitSpec>,
    /// Orbits of twist axes.
    pub axis_orbits: Vec<AxisOrbitSpec>,
    /// Orbits of named points.
    pub named_point_orbits: Vec<SimpleOrbitSpec>,
    /// Orbits of named point sets, each with a gizmo pole distance.
    pub named_point_set_orbits: Vec<(Vec<Str>, f64)>,
}

/// Specification for an orbit of named points or facets in a
/// [`FactorPuzzleSpec`].
#[derive(Debug, Clone)]
pub struct SimpleOrbitSpec {
    /// Vector, name, and generator sequence for each member of the orbit.
    pub orbit_members: Vec<SimpleOrbitMemberSpec>,
}

impl SimpleOrbitSpec {
    /// Returns the lexicographically first name in the orbit, or an empty
    /// string if the orbit is empty.
    pub fn min_name(&self) -> &str {
        self.orbit_members
            .iter()
            .map(|point| point.name.as_str())
            .min()
            .unwrap_or("")
    }
}

impl SimpleOrbitSpec {
    /// Returns the number of named points in the orbit.
    #[allow(clippy::len_without_is_empty)] // should never be empty
    pub fn len(&self) -> usize {
        self.orbit_members.len()
    }
}

/// Specification for a member of a [`SimpleOrbitSpec`].
#[derive(Debug, Clone)]
pub struct SimpleOrbitMemberSpec {
    /// Vector corresponding to the member.
    pub vector: Vector,
    /// Name of the member.
    pub name: Str,
    /// Abbreviated generator sequence, which is used for the HPS code generator
    /// utility.
    pub abbr_gen_seq: AbbrGenSeq,
}

/// Specification for an orbit of axes in a [`FactorPuzzleSpec`].
///
/// This does not include the full axis name because those will be automatically
/// generated based on the nearby named points.
#[derive(Debug, Clone)]
pub struct AxisOrbitSpec {
    /// Prefix for the axis orbit.
    pub prefix: Str,
    /// Vector for one axis in the orbit.
    pub vector: Vector,
}

fn named_vectors<'a, T>(
    initial_vector: &'a Vector,
    generators: &'a PerGenerator<Motor>,
    names: Vec<(AbbrGenSeq, Str)>,
    warn_fn: impl FnOnce(String),
) -> SimpleOrbitSpec {
    let index_to_gen_seq = hyperpuzzle_core::util::lazy_resolve(
        names
            .iter()
            .map(|(abbr_gen_seq, _)| (abbr_gen_seq.generators.clone(), abbr_gen_seq.end))
            .enumerate(),
        |gens1, gens2| std::iter::chain(&gens1.0, &gens2.0).copied().collect(),
        warn_fn,
    );

    let orbit_members = names
        .into_iter()
        .enumerate()
        .map(move |(i, (abbr_gen_seq, name))| {
            let motor = index_to_gen_seq[&i]
                .0
                .iter()
                .map(|&g| &generators[g])
                .fold(Motor::ident(0), |a, b| a * b);
            SimpleOrbitMemberSpec {
                vector: motor.transform(initial_vector),
                name,
                abbr_gen_seq,
            }
        })
        .collect();

    SimpleOrbitSpec { orbit_members }
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
