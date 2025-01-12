use hyperpuzzle::chrono::TimeDelta;
use hyperpuzzle::{ScrambleParams, ScrambleType};
use hyperpuzzle_log::{LogEvent, LogFile, Puzzle};

use super::time_it;
use crate::SolveVerification;

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

    // TODO: don't use global puzzle library
    crate::load_puzzles();

    let (mut log_file, warnings) = LogFile::deserialize(EXAMPLE_SOLVE).unwrap();
    assert!(warnings.is_empty());

    let expected_verification = SolveVerification {
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
    let (actual_verification, _) =
        time_it("verifying solve", || crate::verify(&log_file.solves[0]));
    assert_eq!(actual_verification, Some(expected_verification));

    if let LogEvent::Twists(twists_str) = &mut log_file.solves[0].log[2] {
        *twists_str = twists_str.strip_suffix(" R'").unwrap().to_string();
    }
    assert_eq!(crate::verify(&log_file.solves[0]), None);
}
