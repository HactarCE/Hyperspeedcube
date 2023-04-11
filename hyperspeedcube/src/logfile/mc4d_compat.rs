#![allow(missing_docs)]

use anyhow::Result;
use cgmath::{Matrix4, SquareMatrix};
use itertools::Itertools;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

use crate::puzzle::*;

const MAGIC_STRING: &str = "MagicCube4D";
const LOG_VERSION: &str = "3";
const RUBIKS_4D_SCHLAFLI_SYMBOL: &str = "{4,3,3}";

/// Returns whether the file starts with the MC4D header string.
pub fn is_mc4d_log_file(s: &str) -> bool {
    s.starts_with(MAGIC_STRING)
}

#[derive(Debug)]
pub struct Mc4dLogFile {
    edge_length: u8,
    scramble_state: ScrambleState,
    view_matrix: Matrix4<f32>,
    scramble_twists: Vec<Twist>,
    solve_twists: Vec<Twist>,
}
impl FromStr for Mc4dLogFile {
    type Err = LogFileError;

    fn from_str(s: &str) -> Result<Self, LogFileError> {
        let mut lines = s.lines();
        let header = lines.next().ok_or(LogFileError::MissingHeader)?;
        let segments = header.split_whitespace().collect_vec();

        if segments.len() != 6 || segments[0] != MAGIC_STRING {
            return Err(LogFileError::BadHeader);
        }

        if segments[1] != LOG_VERSION {
            return Err(LogFileError::UnsupportedLogVersion);
        }

        let scramble_state = match segments[2] {
            "0" => ScrambleState::None,
            "1" => ScrambleState::Partial,
            "2" => ScrambleState::Full,
            "3" => ScrambleState::Solved,
            _ => return Err(LogFileError::BadHeader),
        };

        // Ignore move count (`segments[3]`).

        let unsupported_puzzle_err =
            || LogFileError::UnsupportedPuzzle(format!("{} {}", segments[4], segments[5]));
        if segments[4] != RUBIKS_4D_SCHLAFLI_SYMBOL {
            return Err(unsupported_puzzle_err());
        }
        let edge_length = segments[5]
            .parse::<u8>()
            .map_err(|_| unsupported_puzzle_err())?;

        let mut view_matrix = [[0.0; 4]; 4];
        for row in &mut view_matrix {
            *row = lines
                .next()
                .ok_or(LogFileError::BadViewMatrix)?
                .split_whitespace()
                .map(|s| s.parse::<f32>().map_err(|_| LogFileError::BadViewMatrix))
                .collect::<Result<Vec<f32>, _>>()?
                .try_into()
                .map_err(|_| LogFileError::BadViewMatrix)?;
        }
        let view_matrix = cgmath::Matrix4::from(view_matrix);

        if lines.next() != Some("*") {
            return Err(LogFileError::MissingSep);
        }

        let mut scramble_twists = vec![];
        let mut solve_twists = vec![];
        for line in lines {
            for move_str in line
                .split_whitespace()
                .map(|s| s.trim_end_matches('.').trim())
                .filter(|s| !s.is_empty())
            {
                if move_str == "m|" {
                    scramble_twists = std::mem::take(&mut solve_twists);
                } else {
                    solve_twists.extend(Rubiks4D::from_mc4d_twist_string(move_str));
                }
            }
        }

        Ok(Self {
            edge_length,
            scramble_state,
            view_matrix,
            scramble_twists,
            solve_twists,
        })
    }
}
impl fmt::Display for Mc4dLogFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} {} {} {} {} {}",
            MAGIC_STRING,
            LOG_VERSION,
            self.scramble_state as u8,
            self.solve_twists.len(),
            RUBIKS_4D_SCHLAFLI_SYMBOL,
            self.edge_length,
        )?;
        let mat: [[f32; 4]; 4] = self.view_matrix.into();
        for col in mat {
            writeln!(f, "{} {} {} {}", col[0], col[1], col[2], col[3])?;
        }
        writeln!(f, "*")?;

        let mut twist_strs = vec![];
        if self.scramble_state != ScrambleState::None {
            twist_strs.extend(
                self.scramble_twists
                    .iter()
                    .copied()
                    .map(Rubiks4D::to_mc4d_twist_string),
            );
            twist_strs.push("m|".to_string());
        }
        twist_strs.extend(
            self.solve_twists
                .iter()
                .copied()
                .map(Rubiks4D::to_mc4d_twist_string),
        );

        if twist_strs.is_empty() {
            twist_strs.push(String::new());
        }
        *twist_strs.last_mut().unwrap() += ".";

        for line in twist_strs.chunks(10) {
            writeln!(f, "{}", line.iter().join(" "))?;
        }

        Ok(())
    }
}
impl Mc4dLogFile {
    pub fn from_puzzle(puzzle: &PuzzleController) -> Result<Self, LogFileError> {
        match puzzle.ty() {
            PuzzleTypeEnum::Rubiks4D { layer_count } => Ok(Self {
                edge_length: layer_count,
                scramble_state: puzzle.scramble_state(),
                view_matrix: Matrix4::identity(),
                scramble_twists: puzzle.scramble().to_vec(),
                solve_twists: puzzle
                    .undo_buffer()
                    .iter()
                    .filter_map(|entry| entry.twist())
                    .collect(),
            }),
            _ => Err(LogFileError::UnsupportedPuzzle(puzzle.name().to_string())),
        }
    }

    pub fn to_puzzle(&self) -> Result<PuzzleController, String> {
        let puzzle_type = PuzzleTypeEnum::Rubiks4D {
            layer_count: self.edge_length,
        };
        puzzle_type.validate()?;
        let mut ret = PuzzleController::new(puzzle_type);

        for &twist in &self.scramble_twists {
            if let Err(e) = ret.twist_no_collapse(twist) {
                log::warn!("Error executing twist {e:?} from MC4D log file")
            }
        }
        ret.add_scramble_marker(self.scramble_state);

        for &twist in &self.solve_twists {
            if let Err(e) = ret.twist_no_collapse(twist) {
                log::warn!("Error executing twist {e:?} from MC4D log file")
            }
        }
        ret.skip_twist_animations();
        ret.mark_saved();

        Ok(ret)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogFileError {
    MissingHeader,
    BadHeader,
    UnsupportedLogVersion,
    UnsupportedPuzzle(String),
    BadViewMatrix,
    MissingSep,
}
impl fmt::Display for LogFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => write!(f, "missing header"),
            Self::BadHeader => write!(f, "invalid header"),
            Self::UnsupportedLogVersion => write!(f, "unsupported log version"),
            Self::UnsupportedPuzzle(name) => write!(f, "unsupported puzzle: {name}"),
            Self::BadViewMatrix => write!(f, "invalid view matrix"),
            Self::MissingSep => write!(f, "missing sep"),
        }
    }
}
impl Error for LogFileError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mc4d_compat() {
        let ty = PuzzleTypeEnum::Rubiks4D { layer_count: 5 };

        for axis in (0..ty.twist_axes().len() as _).map(TwistAxis) {
            for direction in (0..ty.twist_directions().len() as _).map(TwistDirection) {
                let twist = Twist {
                    axis,
                    direction,
                    layers: LayerMask(5),
                };
                let s = Rubiks4D::to_mc4d_twist_string(twist);
                if let Some(t) = Rubiks4D::from_mc4d_twist_string(&s) {
                    assert_eq!(t, twist);
                }
            }
        }
    }
}
