#![allow(unused, unused_imports)]

use anyhow::{anyhow, Context, Result};
use bitvec::vec::BitVec;
use num_enum::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::io::{self, Read};
use std::path::Path;
use std::str::FromStr;
use strum::IntoEnumIterator;

mod mc4d_compat;

use crate::puzzle::*;
use crate::util::wrap_words;

/// Loads a log file and returns the puzzle state, along with any warnings.
pub fn load_file(path: &Path) -> anyhow::Result<(PuzzleController, Vec<String>)> {
    let mut file = std::fs::File::open(path)?;

    if mc4d_compat::is_mc4d_log_file(&file) {
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        let puzzle = mc4d_compat::Mc4dLogFile::from_str(&string)?
            .to_puzzle()
            .map_err(|e| anyhow!(e))?;
        let warnings = vec![];
        Ok((puzzle, warnings))
    } else {
        serde_yaml::from_reader::<_, LogFile>(file)?.to_puzzle()
    }
}

/// Saves the puzzle state to a log file. Marks the puzzle as saved.
pub fn save_file(path: &Path, puzzle: &mut PuzzleController) -> anyhow::Result<()> {
    let mc4d_log = path
        .extension()
        .filter(|ext| ext.eq_ignore_ascii_case("log"))
        .and_then(|_| mc4d_compat::Mc4dLogFile::from_puzzle(puzzle).ok());
    if let Some(log) = mc4d_log {
        std::fs::write(path, log.to_string())?;
    } else {
        LogFile::new(puzzle).write_to_file(std::fs::File::create(path)?)?;
    }
    puzzle.mark_saved();
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct LogFile {
    version: usize,
    #[serde(default)]
    puzzle: Option<String>, // TODO: accept old puzzle enum
    #[serde(default)]
    state: u8,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_impl::hex_bitvec::opt"
    )]
    visible_pieces: Option<BitVec>,
    #[serde(
        default,
        skip_serializing_if = "cgmath::Zero::is_zero",
        skip_deserializing
    )]
    scramble_length: usize,
    #[serde(default, skip_deserializing)]
    twist_count: BTreeMap<TwistMetric, usize>,
    #[serde(default, skip_serializing)] // manually serialized
    scramble: String,
    #[serde(default, skip_serializing)] // manually serialized
    twists: String,
}
impl LogFile {
    const COMMENT_STRING: &'static str = "# Hyperspeedcube puzzle log";
    const VERSION: usize = 1;

    fn new(puzzle: &PuzzleController) -> Self {
        wrap_words(vec!["word"].into_iter()); // stop complaining, I'm gonna use this
        todo!("log file from puzzle controller {:?}", puzzle.ty())

        // let notation = puzzle.notation_scheme();

        // Self {
        //     version: Self::VERSION,
        //     puzzle: Some(puzzle.ty()),
        //     state: puzzle.scramble_state() as u8,
        //     visible_pieces: puzzle
        //         .is_any_piece_hidden()
        //         .then(|| puzzle.visible_pieces().to_bitvec()),
        //     scramble_length: puzzle.scramble().len(),
        //     twist_count: TwistMetric::iter()
        //         .map(|metric| (metric, puzzle.twist_count(metric)))
        //         .collect(),
        //     scramble: crate::util::wrap_words(
        //         puzzle.scramble().iter().map(|twist| twist.to_string()),
        //     ),
        //     twists: crate::util::wrap_words(
        //         puzzle
        //             .undo_buffer()
        //             .iter()
        //             .map(|&entry| entry.to_string(notation)),
        //     ),
        // }
    }

    fn validate(&self) -> Result<()> {
        todo!("validate")
        // if let Some(puzzle_ty) = self.puzzle {
        //     puzzle_ty.validate().map_err(|e| anyhow!(e))?;
        // }
        // Ok(())
    }

    fn scramble(&self) -> (Vec<Twist>, Vec<TwistParseError<'_>>) {
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

    fn twists(&self, puzzle_type: &PuzzleType) -> (Vec<Twist>, Vec<TwistParseError<'_>>) {
        let mut ret_twists = vec![];
        let mut ret_errors = vec![];
        for twist_str in self.twists.split_whitespace() {
            match puzzle_type.notation.parse_twist(twist_str) {
                Ok(twist) => ret_twists.push(twist),
                Err(error_msg) => ret_errors.push(TwistParseError {
                    twist_str,
                    error_msg,
                }),
            }
        }
        (ret_twists, ret_errors)
    }

    fn write_to_file(&self, mut f: impl io::Write) -> Result<()> {
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

    fn to_puzzle(&self) -> Result<(PuzzleController, Vec<String>)> {
        todo!("logfile to puzzle")

        // self.validate()?;

        // let mut warnings = vec![];

        // if self.version != LogFile::VERSION {
        //     warnings.push(format!(
        //         "This log file was saved using a \
        //          different version of Hyperspeedcube \
        //          (log file format v{:?}; expected v{:?})",
        //         self.version,
        //         LogFile::VERSION,
        //     ));
        // }

        // let puzzle_type = self.puzzle.context("unable to find puzzle type")?;
        // let mut ret = PuzzleController::new(&puzzle_type);

        // let scramble_state = ScrambleState::from_primitive(self.state);

        // if let Some(visible_pieces) = &self.visible_pieces {
        //     ret.set_visible_pieces(visible_pieces);
        // }

        // let (twists, parse_errors) = self.scramble();
        // warnings.extend(parse_errors.iter().map(|e| e.to_string()));
        // for twist in twists {
        //     if let Err(e) = ret.twist_no_collapse(twist) {
        //         warnings.push(e.to_string());
        //     }
        // }
        // ret.add_scramble_marker(scramble_state);

        // let (twists, parse_errors) = self.twists(todo!("puzzle type"));
        // warnings.extend(parse_errors.iter().map(|e| e.to_string()));
        // for twist in twists {
        //     if let Err(e) = ret.twist_no_collapse(twist) {
        //         warnings.push(e.to_string());
        //     }
        // }
        // ret.skip_twist_animations();
        // ret.mark_saved();

        // Ok((ret, warnings))
    }
}

#[derive(Debug)]
struct TwistParseError<'a> {
    twist_str: &'a str,
    error_msg: String,
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
