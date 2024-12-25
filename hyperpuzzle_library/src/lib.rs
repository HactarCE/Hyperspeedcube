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

pub fn load_built_in_puzzles() {
    // TODO: load puzzle library async
    let mut stack = vec![LUA_BUILTIN_DIR.clone()];
    LIBRARY.with(|lib| {
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
                                None => {
                                    log::error!("Error loading built-in file {name}");
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

pub fn load_user_puzzles() {
    let Ok(lua_dir) = hyperpaths::lua_dir() else {
        log::error!("Error locating Lua directory");
        return;
    };
    log::info!("Loading Lua files from path {}", lua_dir.to_string_lossy());
    // TODO: load puzzle library async
    LIBRARY.with(|lib| lib.load_directory(lua_dir).take_result_blocking());
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

    log::info!("building puzzle {}", solve.puzzle.id);
    let puzzle = match LIBRARY
        .with(|lib| lib.build_puzzle(&solve.puzzle.id))
        .take_result_blocking()
    {
        Ok(p) => p,
        Err(e) => {
            log::error!("error building puzzle {}: {e}", solve.puzzle.id);
            return None;
        }
    };

    let scramble_twists: Vec<LayeredTwist> =
        hyperpuzzle_log::notation::parse_twists(&puzzle.twist_by_name, &scramble.twists)
            .try_collect()
            .ok()?;
    let (expected_scramble_twists, _puzzle_state) = puzzle.new_scrambled(scramble_params);
    let is_scramble_correct = expected_scramble_twists == scramble_twists;

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
mod tests {
    use hyperpuzzle::chrono::TimeDelta;
    use hyperpuzzle::{ScrambleParams, ScrambleType};
    use hyperpuzzle_log::{Program, Puzzle};

    use super::*;

    #[test]
    fn test_solve_verification() {
        const EXAMPLE_SOLVE: &str = r#"
// Hyperspeedcube puzzle log
version 2
program name="Hyperspeedcube" version="2.0.0-pre.17"
solve {
    puzzle id="ft_cube:2" version="0.1.0"
    solved true
    duration 132564
    scramble "full" time="2024-12-24T14:26:51.676Z" seed=1381999110 {
        twists "{1-2}R {1-2}L2' {1-2}R' {1-2}D' 2U2 {1-2}D2' U U 2R L2' 2B' 2B2 B2' R' {1-2}D2 2U2 2B' 2L L {1-2}D2 R2' B' F2 {1-2}R2' 2F {1-2}B2 {1-2}D {1-2}F 2F F' 2U {1-2}B2 2U' 2B2' {1-2}L' 2B' {1-2}R' U {1-2}F2' 2L2' F {1-2}R 2F' {1-2}D' L2' {1-2}F' 2U2' F2' U' F R' B D2 2D {1-2}D {1-2}L' {1-2}L2 2U2 {1-2}L2 U' {1-2}R 2R' {1-2}D {1-2}R2' D {1-2}L 2D2' 2L2' 2R2 U2' 2F' 2R2' {1-2}F2 {1-2}L' B' 2D U L2 {1-2}D 2D' D' F {1-2}L {1-2}U2 {1-2}U' D {1-2}F2 {1-2}L2 2B R 2D2' 2L2' 2F {1-2}D2' {1-2}F 2L' R2' F' {1-2}L2' {1-2}D2' 2U' {1-2}F2' L 2R2' 2D {1-2}B' 2B 2R2 {1-2}U R2 L2' 2F' 2F' {1-2}D2' 2B {1-2}L {1-2}R2 {1-2}B' U' {1-2}B' {1-2}U2 {1-2}D2' {1-2}U2 {1-2}L F {1-2}U' {1-2}F' {1-2}F2' 2D 2B' D2 U2 U2' {1-2}R' {1-2}L 2B2' L' {1-2}U2 2R' {1-2}R2 U B {1-2}D' 2F2' L U2' 2F2' R' 2R2 F 2R2 2U' D' D' F2' {1-2}U U2 {1-2}B2 R2 2D 2U2 {1-2}B' 2R {1-2}U U2' {1-2}U2 2R2' B2 F2' D2' {1-2}B F2 2R' 2D' 2R2' F2 {1-2}R2' L 2D2 {1-2}F2' 2F2 D2' 2R' 2R' 2D {1-2}U' {1-2}L2' {1-2}F' 2B2' 2R {1-2}D2 {1-2}F2' B2' 2F 2F2 {1-2}R L' 2D2 2B {1-2}L' {1-2}B2 {1-2}F2 {1-2}L2' F' {1-2}U2 {1-2}B' R U2 2U {1-2}R' 2L2' {1-2}U2 {1-2}B 2R' 2B' 2F 2L2' 2D' {1-2}F2 L B {1-2}D' {1-2}B2 U' 2B' {1-2}R2 2R' {1-2}D 2D2 2F B' B2 L2' 2R {1-2}D R' R U2' {1-2}D2 {1-2}L2' L' {1-2}R {1-2}R' {1-2}D2' L2' L' R2' F 2F {1-2}B' {1-2}R2' 2R' 2B' F' L' 2F' 2U2' B2 R2' 2B2 B2' L2 U2' {1-2}F2' 2L2' {1-2}D2' {1-2}F2' L 2U {1-2}R {1-2}D2' F2' D2' D F2' U2 R' 2R 2B' L 2B 2L2' L2' D' {1-2}R' 2F' 2D2' {1-2}D {1-2}D2 2B' L' 2R' {1-2}U2 2L' {1-2}B2' R2' F {1-2}D R2' 2D2' 2F {1-2}F' D' {1-2}L2' 2R' L {1-2}D2' 2B2' R2' {1-2}B' U2 B2 L2' B2 L2 L U2 {1-2}B' B2 R' 2F' 2F {1-2}B' 2B2 {1-2}R2 R {1-2}D' {1-2}L' 2F2' B2' D2' 2U2 B2' 2F {1-2}F' {1-2}L 2L2 F 2B2' 2U L2 U U2' 2F2 2L 2R' 2B2 D2 {1-2}F' 2R2' {1-2}L2 F2 D2' {1-2}U' {1-2}F2 {1-2}L' {1-2}L' 2U F2 F' {1-2}B2 {1-2}L2' {1-2}B {1-2}B2 R2' {1-2}D2' {1-2}U2 2R' {1-2}L {1-2}R F' {1-2}F 2D' F2' {1-2}B' {1-2}D' {1-2}F' 2D' 2F2' {1-2}F2 2D2 2D' 2U2 R2' D {1-2}D' {1-2}F2' {1-2}L2 {1-2}L' 2U D' 2L' {1-2}F' R2' 2F' D' F2 R 2D2' 2L 2F2' {1-2}L2 2R2 {1-2}F2 2R' 2L2' B B2 {1-2}U D2 L2' 2R2' 2D2 D' 2U2 2U R' 2U2' 2B2' 2D 2D' 2U 2L {1-2}D' D2 2L D2' 2L2' {1-2}D2' 2R2 D2' D2 2B F' 2D2 L L L' L {1-2}F2 {1-2}U 2U2 2B' R2 2F' 2R2' 2F F2' D 2B2' 2F D' {1-2}B {1-2}U2' F 2L {1-2}U2 {1-2}L2 D2' {1-2}B' 2R2' B2' F2 {1-2}R2' 2R2' {1-2}U {1-2}U' F' {1-2}B' 2D 2F 2F' R 2L2' 2B2 2F2' D2 2L2 B {1-2}U2 D B2 {1-2}R2' 2F' 2F2 2U {1-2}L2 2U2' 2D2' {1-2}B {1-2}F' U 2D2' U2 2U2' {1-2}D 2B' B 2F2' 2R2' {1-2}F {1-2}D2' {1-2}L2' U2 {1-2}U' {1-2}R2 {1-2}U' 2F2' R' 2R' F2' B2' {1-2}L2 {1-2}D2' 2R2 {1-2}B 2B' B2 {1-2}U2' {1-2}R2' {1-2}L2 2B 2B2' 2F' {1-2}F 2L2' L' 2F R' B' 2L2' {1-2}R 2L2 {1-2}D 2B 2D2' 2U' 2R2' 2B' U' {1-2}L2 2U2 B 2F2' B' {1-2}F2' 2R' {1-2}B2 {1-2}L2' {1-2}B' {1-2}D L' F 2L D2 {1-2}R2' {1-2}U' 2R 2F2 2L2 2D' {1-2}U 2U' {1-2}U2' 2U2' F2 R2' {1-2}D2 2D2 2R {1-2}F {1-2}R2' {1-2}U2 {1-2}U2 {1-2}L2' F2 B' 2U2' L2 D2' 2D2 R2' {1-2}D2 2F2 {1-2}L' 2L 2R2 2L' D2' 2B R {1-2}L 2R 2R2' {1-2}F F' 2D2' {1-2}L' {1-2}D' 2F' 2R' {1-2}L2 F' D2' 2F' D2 2F2 2D2' 2L2 2U' D2' {1-2}U2 2R2' {1-2}B2' 2U2' 2U 2L' 2F' D' L2' {1-2}D R2' D2 2R2' {1-2}U2' 2D {1-2}F2 B2 D2 {1-2}B {1-2}U 2U' {1-2}L 2R 2L' {1-2}U2' 2F' 2B R2 2F' D' U {1-2}F' 2R2' U' {1-2}R2' 2D2' {1-2}B2 D2 {1-2}D2' D' D2 {1-2}U2' 2F' {1-2}B' F2' 2D2' 2B2' R2 R2 {1-2}L2' B2' D' {1-2}U' 2L2' {1-2}U2 2B2' B' F D' 2L2 F2 2B2' {1-2}D' {1-2}U2' {1-2}B2' {1-2}R' U2' {1-2}F2' 2F' 2U' {1-2}L2' {1-2}R2 {1-2}L2' L' U2 {1-2}R' {1-2}U2' F' F' U' 2F2' {1-2}R2' 2R2 {1-2}B {1-2}R2 2R2 U2' 2U2 {1-2}B L2' R {1-2}R2' {1-2}R' 2F2' 2D2 2U2 {1-2}U' 2U2' 2B2' 2L2 D' {1-2}R2' {1-2}L2 D 2R 2D2' {1-2}L2' {1-2}B {1-2}R' 2F2' {1-2}F' 2F2' {1-2}R2 2B 2R' F' {1-2}D2' 2L2 F' {1-2}F L {1-2}L {1-2}D' {1-2}U2' {1-2}B' 2B 2B L 2F 2B L' {1-2}B2' {1-2}U2' B2' R2' L 2L 2B2' F' {1-2}U2' U2 {1-2}F 2R2' 2L' {1-2}L' {1-2}B 2R F {1-2}B2 {1-2}B' B B2' 2D' {1-2}F2 {1-2}R2 2R2 {1-2}D2 B2' {1-2}U2' 2L2' B' {1-2}D' 2D F2' {1-2}R2' D' 2L {1-2}U2' {1-2}L2 {1-2}U2 L2 2B2' 2D2 R' 2U2 {1-2}D2' 2L2 2F2' 2B R2 {1-2}B' D' {1-2}L {1-2}D2 2F' 2R2 {1-2}B' 2B 2U2 {1-2}B2 D {1-2}B2' {1-2}L2 D {1-2}U 2D2 D2' {1-2}U 2D2 U' 2F' L 2F2 2B' D' {1-2}F 2U2' 2R2' 2L 2R R' L' {1-2}B' B 2D 2R2 {1-2}F2 U' 2R' U2 {1-2}B2 {1-2}D' F2' L' {1-2}R' {1-2}D2' F' 2B2 {1-2}B' {1-2}F2' F2' 2L 2F R 2F' R2' {1-2}F2 L' D2 {1-2}D F2 L {1-2}F2 2U2 {1-2}U U' {1-2}B 2U2' U2 {1-2}D2 D2' B D' B2 L 2U' {1-2}D' {1-2}D2' {1-2}F2' D2' 2R' {1-2}B {1-2}L' L2 2D 2U2' 2U {1-2}B2' B2 {1-2}D2 {1-2}L2 {1-2}B2 B D2 F' 2D2 L2 2R2 {1-2}F2' 2U2' {1-2}D2' U2' {1-2}B2 2F2 2R {1-2}D2' 2B' F2 U2' {1-2}D2' U F F2' D' F {1-2}U {1-2}D2' 2B' {1-2}U2' D' F R2 {1-2}F2 2B2' L2 {1-2}R2 {1-2}B' 2L R2' U L2 L2' U2 L 2B 2U {1-2}R {1-2}U' 2U 2D2 2B2 {1-2}F2 2F2 {1-2}B2' 2F2 2R2 U' 2D' 2L2 F' U {1-2}F2' {1-2}R2' 2D' 2R2 U 2D' 2B2 {1-2}F2' 2U' {1-2}R B2' 2R2 2R' L' D2' F2 R2' 2L' {1-2}F' U' F D 2D' 2B2 {1-2}B' U2 {1-2}U' 2F R R' {1-2}F U' {1-2}R U 2U' {1-2}U2 L' {1-2}B2 2U' 2U' 2F2' {1-2}L2 2B2' {1-2}D2' {1-2}F2' {1-2}D' {1-2}D2 D {1-2}F2 2F2' D2' {1-2}L2 2U2' 2U2' D2 {1-2}F 2D2 {1-2}U2"
    }
    log {
        scramble
        start-solve time="2024-12-24T14:27:03.699Z" duration=12023
        twists "F' R D R D' D' U F R F' R' F R F' R' U' R' R' U U R U' U' R R B B R' R R B' B' R'"
        end-solve time="2024-12-24T14:29:01.231Z" duration=129554
    }
}"#;

        crate::load_built_in_puzzles();
        crate::load_user_puzzles();
        crate::LIBRARY.with(|lib| {
            lib.puzzles()
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<Option<String>>>()
        });

        let (log_file, warnings) = LogFile::deserialize(EXAMPLE_SOLVE).unwrap();
        assert!(warnings.is_empty());
        dbg!(verify(&log_file));

        let expected_verification = SolveVerification {
            solve_index: 0,
            program: Some(Program {
                name: Some("Hyperspeedcube".to_string()),
                version: Some("2.0.0-pre.17".to_string()),
            }),
            puzzle: Puzzle {
                id: "ft_cube:2".to_string(),
                version: "0.1.0".to_string(),
            },
            scramble: ScrambleParams {
                ty: ScrambleType::Full,
                time: "2024-12-24T14:26:51.676Z".parse().unwrap(),
                seed: 1381999110,
            },
            is_scramble_correct: true,
            solution_stm_count: 24,
            single_session: true,
            used_macros: false,
            speedsolve_duration: Some(TimeDelta::new(117, 531000000).unwrap()),
            blindsolve_duration: None,
            time_completed: "2024-12-24T14:29:01.231Z".parse().unwrap(),
        };
        assert_eq!(verify(&log_file), vec![expected_verification]);
    }
}
