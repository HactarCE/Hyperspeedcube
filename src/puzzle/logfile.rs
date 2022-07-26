use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::{fmt, io};
use strum::IntoEnumIterator;

use super::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct LogFile {
    pub version: usize,
    #[serde(default)]
    pub puzzle: Option<PuzzleTypeEnum>,
    #[serde(default)]
    pub state: u8,
    #[serde(default, skip_serializing_if = "cgmath::Zero::is_zero")]
    pub scramble_length: usize,
    #[serde(default)]
    pub twist_count: BTreeMap<TwistMetric, usize>,
    #[serde(default, skip_serializing)]
    pub scramble: String,
    #[serde(default, skip_serializing)]
    pub twists: String,
}
impl LogFile {
    const COMMENT_STRING: &'static str = "# Hyperspeedcube puzzle log";
    pub const VERSION: usize = 1;

    pub fn new(
        puzzle_type: &dyn PuzzleType,
        state: ScrambleState,
        scramble: &[Twist],
        twists: &[Twist],
    ) -> Self {
        let notation = puzzle_type.notation_scheme();

        Self {
            version: Self::VERSION,
            puzzle: Some(puzzle_type.ty()),
            state: state as u8,
            scramble_length: scramble.len(),
            twist_count: TwistMetric::iter()
                .map(|metric| (metric, puzzle_type.count_twists(twists, metric)))
                .collect(),
            scramble: crate::util::wrap_words(scramble.iter().map(|twist| twist.to_string())),
            twists: crate::util::wrap_words(
                twists.iter().map(|&twist| notation.twist_to_string(twist)),
            ),
        }
    }

    pub fn scramble<'a>(&'a self) -> (Vec<Twist>, Vec<TwistParseError<'a>>) {
        let mut ret_twists = vec![];
        let mut ret_errors = vec![];
        for twist_str in self.scramble.split_whitespace() {
            match twist_str.parse() {
                Ok(twist) => ret_twists.push(twist),
                Err(()) => ret_errors.push(TwistParseError {
                    twist_str,
                    error_msg: "invalid twist".to_string(),
                }),
            }
        }
        (ret_twists, ret_errors)
    }

    pub fn twists<'a>(
        &'a self,
        puzzle_type: &dyn PuzzleType,
    ) -> (Vec<Twist>, Vec<TwistParseError<'a>>) {
        let mut ret_twists = vec![];
        let mut ret_errors = vec![];
        for twist_str in self.twists.split_whitespace() {
            match puzzle_type.notation_scheme().parse_twist(twist_str) {
                Ok(twist) => ret_twists.push(twist),
                Err(error_msg) => ret_errors.push(TwistParseError {
                    twist_str,
                    error_msg,
                }),
            }
        }
        (ret_twists, ret_errors)
    }

    pub fn write_to_file(&self, mut f: impl io::Write) -> Result<()> {
        writeln!(&mut f, "{}", Self::COMMENT_STRING)?;
        serde_yaml::to_writer(&mut f, self)?;
        if !self.scramble.is_empty() {
            writeln!(&mut f, "scramble: >")?;
            for line in self.scramble.lines() {
                writeln!(&mut f, "  {line}")?;
            }
        }
        if !self.twists.is_empty() {
            writeln!(&mut f, "twists: >")?;
            for line in self.twists.lines() {
                writeln!(&mut f, "  {line}")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct TwistParseError<'a> {
    pub twist_str: &'a str,
    pub error_msg: String,
}
impl fmt::Display for TwistParseError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error parsing twist {:?}: {}",
            self.twist_str, self.error_msg,
        )
    }
}
impl Error for TwistParseError<'_> {}
