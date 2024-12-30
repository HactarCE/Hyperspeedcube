#![allow(missing_docs)] // TODO: remove this and rework the whole crate

use hyperpuzzle::chrono::Duration;
use hyperpuzzle::{LayeredTwist, Library, Timestamp, TwistMetric};
use hyperpuzzle_log::{LogEvent, Solve};
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::SmallVec;

#[macro_use]
extern crate lazy_static;

thread_local! {
    // TODO: try to make this private
    pub static LIBRARY: hyperpuzzle::Library = Library::new();
}

// TODO: make this private
pub static LUA_BUILTIN_DIR: include_dir::Dir<'_> = if hyperpaths::IS_OFFICIAL_BUILD {
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../lua")
} else {
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/resources/lua")
};

/// Loads or reloads all puzzles.
pub fn load_puzzles() {
    LIBRARY.with(load_puzzles_in_lib);
}

fn load_puzzles_in_lib(lib: &Library) {
    lib.reset();

    // Load built-in puzzles
    let mut stack = vec![LUA_BUILTIN_DIR.clone()];
    while let Some(dir) = stack.pop() {
        for entry in dir.entries() {
            match entry {
                include_dir::DirEntry::Dir(subdir) => {
                    stack.push(subdir.clone());
                }
                include_dir::DirEntry::File(file) => {
                    if file.path().extension().is_some_and(|ext| ext == "lua") {
                        let name = Library::relative_path_to_filename(file.path());
                        match file.contents_utf8() {
                            Some(contents) => lib.add_file(name, None, contents.to_string()),
                            None => log::error!("Error loading built-in file {name}"),
                        }
                    }
                }
            }
        }
    }

    // Load user puzzles
    let Ok(lua_dir) = hyperpaths::lua_dir() else {
        log::error!("Error locating Lua directory");
        return;
    };
    log::info!("Reading Lua files from path {}", lua_dir.to_string_lossy());
    lib.read_directory(lua_dir);
    log::info!("Loading Lua files");
    lib.load_files();
    lib.wait();
}

lazy_static! {
    // TODO: make private? maybe refactor for performance
    pub static ref LIBRARY_LOG_LINES: Mutex<Vec<hyperpuzzle::LuaLogLine>> = Mutex::new(vec![]);
}

pub fn verify(solve: &Solve) -> Option<SolveVerification> {
    verify_internal(solve, true)
}

pub fn verify_without_checking_solution(solve: &Solve) -> Option<SolveVerification> {
    verify_internal(solve, false)
}

fn verify_internal(solve: &Solve, check_solution: bool) -> Option<SolveVerification> {
    if !solve.solved {
        return None;
    }
    let scramble = solve.scramble.clone()?;
    let scramble_params = scramble.params()?;

    log::info!("building puzzle {} for verification", solve.puzzle.id);
    let puzzle = match LIBRARY.with(|lib| lib.build_puzzle_blocking(&solve.puzzle.id)) {
        Ok(p) => p,
        Err(()) => {
            log::error!("error building puzzle {}", solve.puzzle.id);
            return None;
        }
    };

    let scramble_twists: Vec<LayeredTwist> =
        hyperpuzzle_log::notation::parse_twists(&puzzle.twist_by_name, &scramble.twists)
            .try_collect()
            .ok()?;
    let expected_scrambled_puzzle = puzzle.new_scrambled(scramble_params); // TODO: this may be very slow
    let is_scramble_correct = expected_scrambled_puzzle.twists == scramble_twists;

    let mut log = solve.log.iter();

    let Some(LogEvent::Scramble) = log.next() else {
        return None; // didn't start by scrambling!
    };

    let mut twist_groups: Vec<SmallVec<[LayeredTwist; 1]>> = vec![];
    let mut time_completed = None;
    let mut speedsolve_start = None;
    let mut speedsolve_end = None;
    let mut single_session = true;
    for event in log {
        match event {
            LogEvent::Scramble => return None, // don't scramble again!
            LogEvent::Click { .. } => (),      // ignore interaction events
            LogEvent::Twists(twists_str) => {
                for twist_group in hyperpuzzle_log::notation::parse_grouped_twists(
                    &puzzle.twist_by_name,
                    twists_str,
                ) {
                    twist_groups.push(twist_group.into_iter().try_collect().ok()?);
                }
            }
            LogEvent::StartSolve { time: _, duration } => {
                speedsolve_start = *duration;
            }
            LogEvent::EndSolve { time, duration } => {
                time_completed = *time;
                speedsolve_end = *duration;
                break; // apparently we're done!
            }
            LogEvent::StartSession { .. } | LogEvent::EndSession { .. } => {
                single_session = false;
            }
        }
    }
    let time_completed = time_completed?; // must say when it was completed

    let mut twists_done;
    if check_solution {
        let mut puzzle_state = puzzle.new_solved_state();
        for twist in scramble_twists {
            if let Ok(new_state) = puzzle_state.do_twist(twist) {
                puzzle_state = new_state;
            }
        }
        twists_done = vec![];
        for twist in twist_groups.into_iter().flatten() {
            if let Ok(new_state) = puzzle_state.do_twist(twist) {
                puzzle_state = new_state;
                twists_done.push(twist);
            }
        }
        if !puzzle_state.is_solved() {
            return None;
        }
    } else {
        twists_done = twist_groups.into_iter().flatten().collect();
    }
    let solution_stm_count = TwistMetric::Stm.count_twists(&puzzle, twists_done);

    let speedsolve_duration = Option::zip(speedsolve_start, speedsolve_end)
        .and_then(|(start, end)| end.checked_sub(start))
        .map(Duration::milliseconds);

    Some(SolveVerification {
        puzzle: solve.puzzle.clone(),
        scramble: scramble_params,
        is_scramble_correct,
        solution_stm_count,
        single_session,
        used_macros: false, // not yet implemented
        speedsolve_duration,
        blindsolve_duration: None, // not yet implemented
        time_completed,
    })
}

pub enum Fact {
    Solve(SolveVerification),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolveVerification {
    pub puzzle: hyperpuzzle_log::Puzzle,
    pub scramble: hyperpuzzle::ScrambleParams,
    pub is_scramble_correct: bool,
    /// Number of twists in [Slice Turn Metric](https://hypercubing.xyz/notation/#turn-metrics).
    pub solution_stm_count: u64,
    pub single_session: bool,
    pub used_macros: bool,
    pub speedsolve_duration: Option<Duration>,
    pub blindsolve_duration: Option<Duration>,
    pub time_completed: Timestamp,
}

#[cfg(test)]
mod tests;
