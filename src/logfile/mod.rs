use anyhow::{anyhow, Context, Result};
use num_enum::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;
use std::{fmt, io};
use strum::IntoEnumIterator;

use crate::puzzle::*;

/// Loads a log file and returns the puzzle state, along with any warnings.
pub fn load_file(path: &Path) -> anyhow::Result<(PuzzleController, Vec<String>)> {
    let log_file: LogFile = serde_yaml::from_reader(std::fs::File::open(path)?)?;
    log_file.validate()?;

    let mut warnings = vec![];

    if log_file.version != LogFile::VERSION {
        warnings.push(format!(
            "This log file was saved using a \
                 different version of Hyperspeedcube \
                 (log file format v{:?}; expected v{:?})",
            log_file.version,
            LogFile::VERSION,
        ));
    }

    let puzzle_type = log_file.puzzle.context("unable to find puzzle type")?;
    let mut ret = PuzzleController::new(puzzle_type);

    let scramble_state = ScrambleState::from_primitive(log_file.state);

    match scramble_state {
        ScrambleState::None => (),
        ScrambleState::Partial | ScrambleState::Full | ScrambleState::Solved => {
            let (twists, parse_errors) = log_file.scramble();
            warnings.extend(parse_errors.iter().map(|e| e.to_string()));
            for twist in twists {
                if let Err(e) = ret.twist(twist) {
                    warnings.push(e.to_string());
                }
            }
            ret.add_scramble_marker(scramble_state);
        }
    }

    let (twists, parse_errors) = log_file.twists(&puzzle_type);
    warnings.extend(parse_errors.iter().map(|e| e.to_string()));
    for twist in twists {
        if let Err(e) = ret.twist(twist) {
            warnings.push(e.to_string());
        }
    }
    ret.catch_up();

    Ok((ret, warnings))
}

/// Saves the puzzle state to a log file. Marks the puzzle as saved.
pub fn save_file(path: &Path, puzzle: &mut PuzzleController) -> anyhow::Result<()> {
    LogFile::new(puzzle).write_to_file(std::fs::File::create(path)?)?;
    puzzle.mark_saved();
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LogFile {
    pub version: usize,
    #[serde(default)]
    pub puzzle: Option<PuzzleTypeEnum>,
    #[serde(default)]
    pub state: u8,
    #[serde(
        default,
        skip_serializing_if = "cgmath::Zero::is_zero",
        skip_deserializing
    )]
    pub scramble_length: usize,
    #[serde(default, skip_deserializing)]
    pub twist_count: BTreeMap<TwistMetric, usize>,
    #[serde(default, skip_serializing)] // manually serialized
    pub scramble: String,
    #[serde(default, skip_serializing)] // manually serialized
    pub twists: String,
}
impl LogFile {
    const COMMENT_STRING: &'static str = "# Hyperspeedcube puzzle log";
    pub const VERSION: usize = 1;

    pub fn new(puzzle: &PuzzleController) -> Self {
        let notation = puzzle.notation_scheme();

        Self {
            version: Self::VERSION,
            puzzle: Some(puzzle.ty()),
            state: puzzle.scramble_state() as u8,
            scramble_length: puzzle.scramble().len(),
            twist_count: TwistMetric::iter()
                .map(|metric| (metric, puzzle.twist_count(metric)))
                .collect(),
            scramble: crate::util::wrap_words(
                puzzle.scramble().iter().map(|twist| twist.to_string()),
            ),
            twists: crate::util::wrap_words(
                puzzle
                    .undo_buffer()
                    .iter()
                    .map(|&entry| entry.to_string(notation)),
            ),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(puzzle_ty) = self.puzzle {
            puzzle_ty.validate().map_err(|e| anyhow!(e))?;
        }
        Ok(())
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
