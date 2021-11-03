#![allow(missing_docs)]

use cgmath::Matrix4;
use itertools::Itertools;
use std::fmt;
use std::str::FromStr;

use crate::puzzle::controller::ScrambleState;
use crate::puzzle::rubiks4d::*;

const MAGIC_STRING: &str = "MagicCube4D";
const LOG_VERSION: &str = "3";
const SCHLAFLI_SYMBOL: &str = "{4,3,3}";
const EDGE_LENGTH: &str = "3";

pub struct LogFile {
    pub scramble_state: ScrambleState,
    pub view_matrix: Matrix4<f32>,
    pub scramble_twists: Vec<Twist>,
    pub solve_twists: Vec<Twist>,
}
impl FromStr for LogFile {
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

        // ignore move count (`segments[3]`)

        if segments[4] != SCHLAFLI_SYMBOL || segments[5] != EDGE_LENGTH {
            return Err(LogFileError::UnsupportedPuzzle);
        }

        let mut view_matrix = [[0.0; 4]; 4];
        for j in 0..4 {
            view_matrix[j] = lines
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
            for mut move_str in line.split_whitespace() {
                if move_str.ends_with('.') {
                    move_str = &move_str[..move_str.len() - 1];
                }

                if move_str == "m|" {
                    scramble_twists = solve_twists;
                    solve_twists = vec![];
                } else {
                    solve_twists.push(
                        move_str
                            .parse::<Twist>()
                            .map_err(|_| LogFileError::BadTwists)?,
                    );
                }
            }
        }

        Ok(Self {
            scramble_state,
            view_matrix,
            scramble_twists,
            solve_twists,
        })
    }
}
impl fmt::Display for LogFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} {} {} {} {} {}",
            MAGIC_STRING,
            LOG_VERSION,
            self.scramble_state as u8,
            self.solve_twists.len(),
            SCHLAFLI_SYMBOL,
            EDGE_LENGTH,
        )?;
        let mat: [[f32; 4]; 4] = self.view_matrix.into();
        for col in mat {
            writeln!(f, "{} {} {} {}", col[0], col[1], col[2], col[3])?;
        }
        writeln!(f, "*")?;

        let mut twist_strs = vec![];
        if self.scramble_state != ScrambleState::None {
            twist_strs.extend(self.scramble_twists.iter().map(|t| t.to_string()));
            twist_strs.push("m|".to_string());
        }
        twist_strs.extend(self.solve_twists.iter().map(|t| t.to_string()));

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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LogFileError {
    MissingHeader,
    BadHeader,
    UnsupportedLogVersion,
    UnsupportedPuzzle,
    BadViewMatrix,
    MissingSep,
    BadTwists,
}
impl fmt::Display for LogFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => write!(f, "missing header"),
            Self::BadHeader => write!(f, "bad header"),
            Self::UnsupportedLogVersion => write!(f, "unsupported log version"),
            Self::UnsupportedPuzzle => write!(f, "unsupported puzzle"),
            Self::BadViewMatrix => write!(f, "bad view matrix"),
            Self::MissingSep => write!(f, "missing sep"),
            Self::BadTwists => write!(f, "bad twists"),
        }
    }
}
