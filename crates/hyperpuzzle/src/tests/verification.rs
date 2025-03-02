use hyperpuzzle_core::{ScrambleParams, ScrambleType, chrono};
use hyperpuzzle_log::verify::{SolveVerification, verify};
use hyperpuzzle_log::{LogEvent, LogFile, Puzzle};

use super::{load_new_catalog, time_it};

#[test]
fn test_solve_verification() {
    const EXAMPLE_SOLVE: &str = r#"
// Hyperspeedcube puzzle log
version 2
program name="Hyperspeedcube" version="2.0.0-pre.18"
solve {
    puzzle id="ft_cube:2" version="1.0.0"
    solved #true
    duration 24255
    scramble "full" time="2025-01-25T01:11:26.574Z" seed="2025-01-25T01:11:26.574Z_8378422379567660491" {
        twists "2U2' U2 F' {1-2}L2' 2R' 2L' B' {1-2}U' U' B' {1-2}L' {1-2}L2' 2B2' D2' 2B2' 2L' 2D2 R' U {1-2}U {1-2}F2' F' 2L2 F2' R' {1-2}L {1-2}F 2B2 {1-2}R' {1-2}F2 {1-2}L2' U2 B2' 2B U2 2U 2D L' {1-2}L' {1-2}L {1-2}R' R' L 2R2 {1-2}R2' B2 {1-2}L' {1-2}D U2' 2L' {1-2}R2' F2' 2R U 2F' {1-2}F {1-2}B' {1-2}R 2R2 {1-2}U2' 2L2 2U' 2F2' {1-2}D2 U2 2D F2 {1-2}B {1-2}R2 B2' F2 L' {1-2}D2' {1-2}L2' 2U2 2F2 B U' {1-2}B D' 2U2' 2F2 {1-2}R2 R2' U2 2R2' 2U2' {1-2}D' {1-2}U' 2R2 2B2 2B2' {1-2}R2' F' U2 B' D2 2L2' L {1-2}D' 2D 2U U2' D2' D' D {1-2}U2 2U' U2 {1-2}R2 {1-2}F2 F F F2' B2 {1-2}L 2L {1-2}B2 2L' 2F2 2L2' 2L2' {1-2}L' D2 {1-2}R R2 2D2 D' {1-2}U D' 2B2 F2' 2L2 L R2 L L2' {1-2}U' {1-2}U' L2' B B2' {1-2}U2' {1-2}U2 F2 2F2' {1-2}F2' 2U2 {1-2}F' U' 2B {1-2}U2' {1-2}F 2B2' B' B' {1-2}F' D L2' R' R' 2U2' 2D2 L2 2L2' U L' {1-2}F' 2F B {1-2}L 2B2' L2' {1-2}U' {1-2}R {1-2}B {1-2}B F' D' {1-2}F R2' D' {1-2}U2 {1-2}D {1-2}R {1-2}F' L' 2D' 2D 2B2' {1-2}B' {1-2}B' D2 {1-2}L2' 2F2 R' {1-2}B2' {1-2}B2' 2F R2' {1-2}F2 2D' L {1-2}L2 L' U 2L2' 2D2 {1-2}L2' F2' 2L {1-2}R' R2' 2D 2L2 {1-2}D' 2D 2U L' 2D2 2L' {1-2}R2 {1-2}F 2U2 R' 2B U2 U {1-2}F' B2 {1-2}L' 2B2 B2' R' {1-2}U {1-2}D2 {1-2}D2 {1-2}L2 {1-2}F2 {1-2}R' U 2B {1-2}L2 2F2 D' L {1-2}L2 {1-2}B2 2F F2' {1-2}L' B2 {1-2}F 2B2 2L' D2 L2 D' {1-2}B2' 2L2 R' 2R2' L {1-2}L2 {1-2}U' F' 2D' {1-2}D2 {1-2}L 2B R' {1-2}D 2F2 {1-2}F 2L2 2F' B 2U {1-2}U {1-2}U' {1-2}L2 2R2' 2D L2 2U2' 2B 2L2 2F {1-2}R {1-2}F' U2' R2 2F D2' {1-2}U2' {1-2}D2' D' 2F' {1-2}F2' F2' {1-2}U2 D2 {1-2}F2' L 2F2' {1-2}U2 F2 F2' 2D 2L' D 2L' {1-2}U2' {1-2}L2 R2 {1-2}F2' {1-2}F {1-2}D' {1-2}U {1-2}B {1-2}R L2 R 2D {1-2}F2' D' 2R2' 2D2 B2' D2' B 2R 2L' D' 2D' {1-2}L2' B2 L2' B 2B F' 2F2 {1-2}B {1-2}L2' D 2F 2L L' D2' R' {1-2}B2 2B' 2D2 2R' {1-2}R2' {1-2}B2' R' {1-2}F' {1-2}D D2' {1-2}U' 2L2 {1-2}U' {1-2}B L2 L2 2B2' {1-2}D2' {1-2}F2 2D' {1-2}U {1-2}F' 2F2 {1-2}D' 2B 2B2' {1-2}B2' {1-2}U2 {1-2}B2 U 2L2 R2' 2B2' U2 2F2' 2L2 {1-2}F2 {1-2}B B2 {1-2}R' 2F2' {1-2}B2 {1-2}U2' B {1-2}R2' D 2F2' {1-2}R U' R2' F2' 2U2 {1-2}U2' U 2R' B2' U2' 2U2' {1-2}F' 2L {1-2}F' B F2 2F2 B {1-2}R2 {1-2}R B 2L2' R' {1-2}R' D2 {1-2}B 2B' {1-2}F' R2 2L2 {1-2}F2 {1-2}U {1-2}B2 2B' {1-2}U 2L2' L2 {1-2}F' 2U' 2R2 2U' {1-2}U L 2U2 2F' L2' D2 B' {1-2}B2' F {1-2}B 2B' 2R {1-2}U2 U 2D {1-2}B2 {1-2}U 2F' 2D' {1-2}R2' 2F2' {1-2}D2' {1-2}F2 {1-2}B {1-2}D2 B2' D' 2U F' B2' L 2U2' 2D 2L 2B' D2' 2U' {1-2}B2 {1-2}D' 2D2 F2 U' 2D2' 2F2 2F' {1-2}U2 {1-2}F2' 2R2 2U' {1-2}D2' {1-2}D B2 {1-2}B2' 2L 2R2' {1-2}B' {1-2}R' 2B2' U' R 2R D2' U2 R2 D2' 2D2' {1-2}U2' {1-2}L2' 2F' L2 {1-2}B2 2F' 2U2 2F2 2F2 {1-2}B2' R2' {1-2}U' L2' 2U2' {1-2}D2' L' D' R' 2L2 R' 2R2 {1-2}F {1-2}D2' 2F2' {1-2}F B2' 2B U2' 2D 2U 2B F2 {1-2}F' 2L2' L2' D2 {1-2}B F2' 2D' 2R2 {1-2}R' {1-2}U' L2' {1-2}U' {1-2}L2 2D' U' R {1-2}B 2L 2F2' R2 F2' {1-2}U2 {1-2}F2' 2L2 L' F2 {1-2}R {1-2}B F2 {1-2}U2 2B' {1-2}U L2 R' 2L2 R2' B' 2F B2' 2R 2B2' B {1-2}B' R {1-2}L' 2D' {1-2}B' L L2' {1-2}D2' 2R2' F U2' B 2R' R U B2 2D2' L2 {1-2}B2' 2R2 {1-2}L2 2L 2R2' F 2D' F2 U 2B F2' 2B 2U D {1-2}U R2' 2B' D' {1-2}R2' 2D2 {1-2}L' 2U 2B' B' 2U' 2B' {1-2}F2 {1-2}L2 2R' L2 2F' {1-2}U2 {1-2}B' {1-2}R2 {1-2}L' 2U' 2L' 2U' {1-2}U {1-2}F' {1-2}F' B' 2U' {1-2}L2 U' {1-2}B L2' 2U2 2D2' B2 2R B' {1-2}D {1-2}L2' U' U2' {1-2}F' B2 D' F' 2L2 R' 2U2 {1-2}B2 F {1-2}D' R' {1-2}L2 {1-2}L2 {1-2}L' {1-2}B' 2F2' 2D2 2R F' U' {1-2}R2' 2R {1-2}F' B2 2D2 L2' 2L2 F' {1-2}D2 {1-2}B2 B2' {1-2}U2' R' 2B2 D2 B' U' D' {1-2}B' L 2R2' {1-2}F' 2U' 2B' U' R2' 2U' 2B B L2 2U' {1-2}D2 F' {1-2}B2 2L2' D2 2R2' 2F R 2F2' R2 B' B B' 2L D2' U2 R2' {1-2}D2' L2' {1-2}F2 2U2' {1-2}B2 2D' F 2L 2R L R' 2U 2L2 {1-2}R2' 2U2' {1-2}L {1-2}R' 2U2 {1-2}U2 B' 2R2' {1-2}F L2' 2U2' L2' L2 {1-2}D2' {1-2}D2' {1-2}L2 {1-2}B' 2R2' {1-2}D2' 2D 2B U2' 2B' {1-2}D2 2R 2F B {1-2}D {1-2}D2' 2B 2B2' L2 F L' D 2U2' {1-2}R2' L' {1-2}U2 2R2' {1-2}D' U B U U' {1-2}U2' {1-2}B' 2U2 {1-2}L R2 B2' 2L2' {1-2}R L' F2' {1-2}F2 {1-2}D' L2' {1-2}F B' {1-2}U 2F' 2U2 {1-2}U' F2 U2' 2D U R2' {1-2}B' 2F2' {1-2}U2 U2' D 2B2' L2' 2B2' {1-2}U2 {1-2}L' {1-2}F {1-2}F2 {1-2}L' L 2R' D2 F {1-2}B2 {1-2}D' D' {1-2}U' R2 2U {1-2}B2 2B' U 2R2 F2' {1-2}U2' 2D' {1-2}R' L2' {1-2}R 2L2 {1-2}B2' 2U2' {1-2}R' L2 2F2 2R F2 2F {1-2}B2' R D2' F2 D {1-2}F2 R2 2U2' {1-2}D2' 2B' {1-2}B 2F2 {1-2}U2' 2R2' 2D' 2R' B' {1-2}F 2B2 2U' 2B2' {1-2}L {1-2}U2' 2U2 D2 F2 2B F' {1-2}F2 2B2 2U' {1-2}B2' B' D 2U' 2D 2R' D' D2' 2F2' {1-2}B' 2U2 B2 {1-2}R2 {1-2}D R2 2D2 L' 2R2' 2U R2' B2' 2F2 2L' F2 {1-2}R' 2B2' 2B2 D' F {1-2}D' B B F2 B 2B {1-2}R2' 2L' U2 {1-2}B {1-2}L2' 2F' 2U2 2B' 2D2' 2F {1-2}U2 {1-2}B2' B' 2B 2F2 {1-2}F' L' F 2F2 {1-2}B2 {1-2}F {1-2}L' 2L U2 B' {1-2}L F 2U2' L2 {1-2}F 2B2 {1-2}U' R2 {1-2}U' 2B2 2D F 2D B' {1-2}U' 2U F' 2U {1-2}F2 {1-2}R' 2L2' {1-2}L {1-2}U' {1-2}R {1-2}U2' {1-2}B2' U2 {1-2}U2 2B2 U' 2U2' 2L' 2L2 2F2' {1-2}R2' R 2L2' {1-2}U2' F' 2R' 2L {1-2}B' {1-2}B {1-2}B2' F2 F 2L' 2R2' {1-2}B' {1-2}D' 2D2' {1-2}D' D U' B {1-2}B2 2D D2 {1-2}R2 U2 2B2 2B2 U2 {1-2}D2 R2"
    }
    log {
        scramble
        start-solve time="2025-01-25T01:11:33.200Z" duration=6600
        twists "L B L' L' B L B' 2B L' 2B' L 2B' 2D 2B U 2L' 2B2' 2L' 2B2' 2L2' 2D2' 2L' 2D2' 2L2'"
        end-solve time="2025-01-25T01:11:47.028Z" duration=20428
    }
}"#;

    let catalog = load_new_catalog();

    let (mut log_file, warnings) = LogFile::deserialize(EXAMPLE_SOLVE).unwrap();
    assert!(warnings.is_empty());

    let expected_verification = SolveVerification {
        puzzle: Puzzle {
            id: "ft_cube:2".to_string(),
            version: "1.0.0".to_string(),
        },
        scramble: ScrambleParams {
            ty: ScrambleType::Full,
            time: "2025-01-25T01:11:26.574Z".parse().unwrap(),
            seed: "2025-01-25T01:11:26.574Z_8378422379567660491".to_string(),
        },
        is_scramble_correct: true,
        solution_stm_count: 23, // `L' L'` is 1 STM, but `B' 2B` is 2 STM
        single_session: true,
        used_macros: false,
        inspection_duration: Some(chrono::TimeDelta::new(6, 600000000).unwrap()),
        speedsolve_duration: Some(chrono::TimeDelta::new(13, 828000000).unwrap()),
        blindsolve_duration: None,
        time_completed: "2025-01-25T01:11:47.028Z".parse().unwrap(),
    };
    let (actual_verification, _) =
        time_it("verifying solve", || verify(&catalog, &log_file.solves[0]));
    assert_eq!(actual_verification, Some(expected_verification));

    if let LogEvent::Twists(twists_str) = &mut log_file.solves[0].log[2] {
        *twists_str = twists_str.strip_suffix(" 2L2'").unwrap().to_string();
    }
    assert_eq!(verify(&catalog, &log_file.solves[0]), None);
}
