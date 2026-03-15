use hyperpuzzle_core::chrono::TimeDelta;
use hyperpuzzle_core::verification::*;
use hyperpuzzle_log::verify::{SolveVerificationError, VerificationOptions, verify};
use pretty_assertions::assert_eq;

use super::{load_new_catalog, time_it};

// These tests also catch scramble compatibility regressions.

#[test]
fn test_online_solve_verification() {
    let catalog = load_new_catalog();

    let (log_file, warnings) =
        hyperpuzzle_log::deserialize(include_str!("2026-03-11T04_05_29.603Z_stm25.hsc")).unwrap();
    assert!(warnings.is_empty());

    let expected_verification = SolveVerification {
        puzzle_canonical_id: "ft_cube:2".to_string(),
        puzzle_version: "1.0.1".to_string(),
        solution_stm: 25,
        used_filters: true,
        used_macros: false,
        timestamps: Timestamps {
            scramble_generation: Some("2026-03-11T04:04:51.250Z".parse().unwrap()),
            inspection_start: Some("2026-03-11T04:04:51.263Z".parse().unwrap()),
            blindfold_don: None,
            solve_start: Some("2026-03-11T04:04:54.040Z".parse().unwrap()),
            solve_completion: Some("2026-03-11T04:05:29.603Z".parse().unwrap()),
        },
        verified_timestamps: VerifiedTimestamps {
            scramble_range_start: Some("2026-03-11T04:04:51Z".parse().unwrap()),
            scramble_range_end: Some("2026-03-11T04:04:54Z".parse().unwrap()),
            completion: Some("2026-03-11T04:05:29Z".parse().unwrap()),
        },
        durations: Durations {
            scramble_network_latency: Some(TimeDelta::new(-3, 250000000).unwrap()),
            scramble_application: Some(TimeDelta::new(0, 13000000).unwrap()),
            inspection: Some(TimeDelta::new(2, 777000000).unwrap()),
            speedsolve: Some(TimeDelta::new(35, 563000000).unwrap()),
            memo: None,
            blindsolve: None,
            timestamp_network_latency: Some(TimeDelta::new(-1, 397000000).unwrap()),
        },
        errors: vec![],
    };

    let (actual_verification, _) = time_it("verifying solve", || {
        verify(&catalog, &log_file.solves[0], VerificationOptions::FULL)
    });
    assert_eq!(Ok(expected_verification), actual_verification);
}

#[test]
fn test_offline_solve_verification() {
    let catalog = load_new_catalog();

    let (mut log_file, warnings) =
        hyperpuzzle_log::deserialize(include_str!("2026-03-11T04_06_10.518Z_stm19.hsc")).unwrap();
    assert!(warnings.is_empty());

    let expected_verification = SolveVerification {
        puzzle_canonical_id: "ft_cube:2".to_string(),
        puzzle_version: "1.0.1".to_string(),
        solution_stm: 19,
        used_filters: false,
        used_macros: false,
        timestamps: Timestamps {
            scramble_generation: Some("2026-03-11T04:06:00.546Z".parse().unwrap()),
            inspection_start: Some("2026-03-11T04:06:00.560Z".parse().unwrap()),
            blindfold_don: None,
            solve_start: Some("2026-03-11T04:06:03.103Z".parse().unwrap()),
            solve_completion: Some("2026-03-11T04:06:10.518Z".parse().unwrap()),
        },
        verified_timestamps: VerifiedTimestamps {
            scramble_range_start: None,
            scramble_range_end: None,
            completion: None,
        },
        durations: Durations {
            scramble_network_latency: None,
            scramble_application: Some(TimeDelta::new(0, 14000000).unwrap()),
            inspection: Some(TimeDelta::new(2, 543000000).unwrap()),
            speedsolve: Some(TimeDelta::new(7, 415000000).unwrap()),
            memo: None,
            blindsolve: None,
            timestamp_network_latency: None,
        },
        errors: vec![
            "cannot validate scramble timestamp: offline scramble".to_string(),
            "cannot validate completion timestamp: no signature".to_string(),
        ],
    };

    let (actual_verification, _) = time_it("verifying solve", || {
        verify(&catalog, &log_file.solves[0], VerificationOptions::FULL)
    });
    assert_eq!(Ok(expected_verification), actual_verification);

    // Mess up the scramble seed
    log_file.solves[0]
        .scramble
        .as_mut()
        .unwrap()
        .seed
        .as_mut()
        .unwrap()
        .truncate(10);
    let (actual_verification, _) = time_it("verifying solve", || {
        verify(&catalog, &log_file.solves[0], VerificationOptions::FULL)
    });
    assert_eq!(
        Err(SolveVerificationError::ScrambleSeedMismatch),
        actual_verification,
    );
}
