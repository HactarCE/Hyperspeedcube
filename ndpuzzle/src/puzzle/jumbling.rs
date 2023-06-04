//! Jumbling puzzle engine.

use ahash::AHashMap;
use anyhow::{bail, ensure, Context, Result};
use itertools::Itertools;
use regex::Regex;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::sync::Arc;

use super::{spec::*, *};
use crate::math::*;
use crate::polytope::*;

const NO_INTERNAL: bool = true;

#[derive(Debug, Clone, PartialEq)]
struct PeriodicTwist {
    transforms: Vec<Rotoreflector>,
}
impl PeriodicTwist {
    fn new(r: Rotoreflector) -> Result<Self> {
        let transforms = std::iter::successors(Some(r.clone()), |a| {
            Some(&r * a).filter(|b| !approx_eq(b, &Rotoreflector::ident()))
        })
        .take(MAX_TWIST_PERIOD + 1)
        .collect_vec();
        if transforms.len() > MAX_TWIST_PERIOD {
            bail!("nonperiodic twist (or period is too big)");
        }

        Ok(Self { transforms })
    }

    fn transform_by(&self, r: &Rotoreflector) -> Self {
        Self {
            transforms: self
                .transforms
                .iter()
                .map(|t| r.transform_rotoreflector_uninverted(t))
                .collect(),
        }
    }

    #[must_use]
    fn reverse(mut self) -> Self {
        self.transforms.reverse();
        self
    }
}
impl AbsDiffEq for PeriodicTwist {
    type Epsilon = Float;

    fn default_epsilon() -> Self::Epsilon {
        crate::math::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.transforms[0].abs_diff_eq(other.transforms.first().unwrap(), epsilon)
            || self.transforms[0].abs_diff_eq(other.transforms.last().unwrap(), epsilon)
    }
}

#[derive(Debug, Clone)]
struct JumblingPuzzle {
    ty: Arc<PuzzleType>,
    piece_states: Vec<Rotoreflector>,
}
impl PuzzleState for JumblingPuzzle {
    fn ty(&self) -> &Arc<PuzzleType> {
        &self.ty
    }

    fn clone_boxed(&self) -> Box<dyn PuzzleState> {
        Box::new(self.clone())
    }

    fn twist(&mut self, twist: Twist) -> Result<(), &'static str> {
        let reference_frame = &self.ty.info(twist.axis).reference_frame;
        let transform = reference_frame
            .reverse()
            .transform_rotoreflector_uninverted(&self.ty.info(twist.direction).transform);
        for piece in (0..self.ty.pieces.len() as u16).map(Piece) {
            if twist.layers[self.layer_from_twist_axis(twist.axis, piece)] {
                self.piece_states[piece.0 as usize] =
                    &transform * &self.piece_states[piece.0 as usize];
            }
        }
        Ok(())
    }

    fn piece_transform(&self, p: Piece) -> Matrix {
        self.piece_states[p.0 as usize]
            .matrix()
            .at_ndim(self.ty.ndim())
    }

    fn is_solved(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
struct JumblingTwist {
    layer: u8,
    transform: Matrix,
}
impl approx::AbsDiffEq for JumblingTwist {
    type Epsilon = Float;

    fn default_epsilon() -> Self::Epsilon {
        crate::math::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.transform.abs_diff_eq(&other.transform, epsilon)
    }
}

fn parse_cut_depths(strings: &[impl AsRef<str>]) -> Result<Vec<Float>> {
    lazy_static! {
        // ^([^ ]+)\s+from\s+([^ ]+)\s+to\s+([^ ]+)$
        // ^                                       $    match whole string
        //  ([^ ]+)          ([^ ]+)        ([^ ]+)     match numbers
        //         \s+from\s+       \s+to\s+            match literal words
        static ref CUT_SEQ_REGEX: Regex =
            Regex::new(r#"^([^ ]+)\s+from\s+([^ ]+)\s+to\s+([^ ]+)$"#).unwrap();
    }

    let mut ret = vec![];
    for s in strings {
        let s = s.as_ref().trim();
        if let Some(captures) = CUT_SEQ_REGEX.captures(s) {
            let n = parse_u8_cut_count(&captures[1])?;
            let a = parse_Float(&captures[2])?;
            let b = parse_Float(&captures[3])?;
            ret.extend(
                (1..=n)
                    .map(|i| i as Float / (n + 1) as Float)
                    .map(|t| util::mix(a, b, t)),
            )
        } else if let Ok(n) = parse_Float(s) {
            ret.push(n)
        } else {
            bail!("expected floating-point number or range 'N from A to B'");
        }
    }
    Ok(ret)
}

fn parse_u8_cut_count(s: &str) -> Result<u8> {
    s.trim()
        .parse()
        .with_context(|| format!("expected integer number of cuts; got {s:?}"))
}
fn parse_Float(s: &str) -> Result<Float> {
    s.trim()
        .parse()
        .with_context(|| format!("expected floating-point number; got {s:?}"))
}

fn deserialize_cut_depths<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Float>, D::Error> {
    parse_cut_depths(&Vec::<String>::deserialize(deserializer)?).map_err(D::Error::custom)
}
