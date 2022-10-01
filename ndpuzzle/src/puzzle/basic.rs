use anyhow::Result;
use itertools::Itertools;

use crate::math::*;
use crate::polytope::*;
use crate::schlafli::SchlafliSymbol;
use crate::spec::*;

const EPSILON: f32 = 0.001;

pub fn build_puzzle(spec: BasicPuzzleSpec) -> Result<Puzzle> {
    let shape_spec = &spec.shape[0];
    let shape_generators = shape_spec
        .symmetries
        .iter()
        .flat_map(|sym| match sym {
            SymmetriesSpec::Schlafli(string) => SchlafliSymbol::from_string(&string).generators(),
        })
        .collect_vec();
    // let m1 = Matrix::from_cols(shape_schlafli.mirrors().iter().rev().map(|v| &v.0))
    //     .inverse()
    //     .unwrap_or(Matrix::EMPTY_IDENT) // TODO: isn't really right
    //     .transpose();
    let poles = shape_spec
        .face_generators
        .iter()
        .map(|v| v.clone().resize(spec.ndim))
        .collect::<Vec<_>>();
    let mut arena = generate_polytope(spec.ndim, &shape_generators, &poles)?;

    let mut axes = vec![];
    for twist_spec in spec.twists {
        let axis_generators = twist_spec
            .symmetries
            .iter()
            .flat_map(|sym| match sym {
                SymmetriesSpec::Schlafli(string) => {
                    SchlafliSymbol::from_string(&string).generators()
                }
            })
            .collect_vec();
        // let m2 = Matrix::from_cols(twist_schlafli.mirrors().iter().rev().map(|v| &v.0))
        //     .inverse()
        //     .unwrap_or(Matrix::EMPTY_IDENT) // TODO: isn't really right
        //     .transpose();
        let base_axes = twist_spec
            .axes
            .iter()
            .map(
                |AxisSpec {
                     normal,
                     cuts,
                     twist_generators,
                 }| {
                    // let normal = m2.transform(normal.clone().resize(spec.ndim));
                    let normal = normal.clone().resize(spec.ndim).normalise().expect("msg");
                    let mut distances = cuts.clone();
                    distances.sort_by(f32::total_cmp);
                    distances.reverse();
                    Axis {
                        normal,
                        distances,
                        transforms: twist_generators
                            .iter()
                            .map(|gen| parse_transform(gen).expect("oops"))
                            .collect(),
                    }
                },
            )
            .collect::<Vec<_>>();
        axes.extend(build_axes(&&axis_generators, &base_axes)?);
    }
    for axis in &axes {
        for i in 0..axis.distances.len() {
            arena.slice_by_plane(&axis.plane(i), false)?;
        }
    }

    Ok(Puzzle { arena, axes })
}

pub fn build_axes(twist_generators: &[Matrix<f32>], base_axes: &[Axis]) -> Result<Vec<Axis>> {
    let mut axes: Vec<Axis> = base_axes.to_vec();
    let mut transforms = axes
        .iter()
        .enumerate()
        .flat_map(|(i, axis)| {
            axis.transforms
                .iter()
                .map(move |transform| (i, transform.clone()))
        })
        .collect_vec();
    let mut next_unprocessed = 0;
    while next_unprocessed < transforms.len() {
        for gen in twist_generators {
            let curr_twist = &transforms[next_unprocessed];
            let curr_axis = &axes[curr_twist.0];
            let new_normal = gen.transform(curr_axis.normal.clone());
            let new_transform =
                &(gen * &curr_twist.1) * &gen.inverse().ok_or(PolytopeError::BadMatrix)?;
            let new_axis = Axis {
                normal: new_normal,
                distances: curr_axis.distances.clone(),
                transforms: vec![],
            };
            let new_i = (0..axes.len())
                .find(|&index| axes[index].normal.approx_eq(&new_axis.normal, EPSILON))
                .unwrap_or_else(|| {
                    axes.push(new_axis);
                    axes.len() - 1
                });
            if transforms
                .iter()
                .all(|(i, t)| !(t.approx_eq(&new_transform, EPSILON) && *i == new_i))
            {
                transforms.push((new_i, new_transform.clone()));
                axes[new_i].add_transform(new_transform);
            }
        }
        next_unprocessed += 1;
    }

    Ok(axes)
}

#[derive(Debug)]
pub struct Puzzle {
    arena: PolytopeArena,
    axes: Vec<Axis>,
}
impl Puzzle {
    pub fn apply_twist(
        &mut self,
        axis_id: AxisId,
        twist_id: TwistId,
    ) -> Result<Result<(), Vec<PolytopeId>>> {
        let axis = &self.axes[axis_id.0];
        let twist = axis.twist(twist_id);
        let spans = self.arena.axis_spans(&axis.normal)?;
        let layer_spans = spans.into_iter().map(|(p, s)| {
            (
                p,
                (
                    axis.layer_from_depth(s.above),
                    axis.layer_from_depth(s.below),
                ),
            )
        });

        let mut pieces = vec![];
        let mut blocking = vec![];
        for (p, (la, lb)) in layer_spans {
            if la == lb && lb == twist.layer {
                pieces.push(p);
            } else if (la..=lb).contains(&twist.layer) {
                blocking.push(p);
            }
        }
        if !blocking.is_empty() {
            return Ok(Err(blocking));
        }

        for p in pieces {
            self.arena.transform_polytope(p, &twist.transform)?;
        }
        Ok(Ok(()))
    }

    pub fn remove_internal(&mut self) -> Result<()> {
        self.arena.remove_internal()
    }

    pub fn polygons(&self) -> Result<Vec<(PolytopeId, Vec<Polygon>)>> {
        self.arena.polygons(true)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Twist {
    pub layer: u8,
    pub transform: Matrix<f32>,
}
impl Twist {
    pub fn approx_eq(&self, other: Twist, epsilon: f32) -> bool {
        self.layer == other.layer && self.transform.approx_eq(&other.transform, epsilon)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    pub normal: Vector<f32>,
    pub distances: Vec<f32>,
    pub transforms: Vec<Matrix<f32>>,
}
impl Axis {
    fn add_transform(&mut self, transform: Matrix<f32>) {
        self.transforms.push(transform);
    }

    pub fn plane(&self, layer: usize) -> Hyperplane {
        Hyperplane {
            normal: self.normal.clone(),
            distance: self.distances[layer],
        }
    }

    pub fn twist(&self, id: TwistId) -> Twist {
        Twist {
            layer: id.layer as u8,
            transform: self.transforms[id.transform].clone(),
        }
    }

    pub fn layer_from_depth(&self, depth: f32) -> u8 {
        // distances is sorted in descending order
        self.distances
            .binary_search_by(|probe| depth.total_cmp(probe))
            .unwrap_or_else(|i| i) as u8
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistId {
    pub layer: usize,
    pub transform: usize,
}

pub struct AxisId(pub usize);
