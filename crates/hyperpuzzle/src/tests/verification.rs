use hyperpuzzle_core::chrono::TimeDelta;
use hyperpuzzle_core::verification::*;
use hyperpuzzle_log::LogFile;
use hyperpuzzle_log::verify::{SolveVerificationError, VerificationOptions, verify};

use super::{load_new_catalog, time_it};

#[test]
fn test_online_solve_verification() {
    let catalog = load_new_catalog();

    let (log_file, warnings) =
        LogFile::deserialize(include_str!("2025-12-30T05_57_13.266Z_stm26.hsc")).unwrap();
    assert!(warnings.is_empty());

    let expected_verification = SolveVerification {
        puzzle_canonical_id: "ft_cube:2".to_string(),
        puzzle_version: "1.0.0".to_string(),
        solution_stm: 26,
        used_filters: true,
        used_macros: false,
        timestamps: Timestamps {
            scramble_generation: Some("2025-12-30T05:56:57.735Z".parse().unwrap()),
            inspection_start: Some("2025-12-30T05:56:57.752Z".parse().unwrap()),
            blindfold_don: None,
            solve_start: Some("2025-12-30T05:56:59.696Z".parse().unwrap()),
            solve_completion: Some("2025-12-30T05:57:13.266Z".parse().unwrap()),
        },
        verified_timestamps: VerifiedTimestamps {
            scramble_range_start: Some("2025-12-30T05:56:57Z".parse().unwrap()),
            scramble_range_end: Some("2025-12-30T05:57:00Z".parse().unwrap()),
            completion: Some("2025-12-30T05:57:13Z".parse().unwrap()),
        },
        durations: Durations {
            scramble_network_latency: Some(TimeDelta::new(-3, 735000000).unwrap()),
            scramble_application: Some(TimeDelta::new(0, 17000000).unwrap()),
            inspection: Some(TimeDelta::new(1, 944000000).unwrap()),
            speedsolve: Some(TimeDelta::new(13, 570000000).unwrap()),
            memo: None,
            blindsolve: None,
            timestamp_network_latency: Some(TimeDelta::new(-1, 734000000).unwrap()),
        },
        errors: vec![],
    };

    let (actual_verification, _) = time_it("verifying solve", || {
        verify(&catalog, &log_file.solves[0], VerificationOptions::FULL)
    });
    assert_eq!(actual_verification, Ok(expected_verification));
}

#[test]
fn test_offline_solve_verification() {
    let catalog = load_new_catalog();

    let (mut log_file, warnings) =
        LogFile::deserialize(include_str!("2025-12-30T06_00_51.251Z_stm27.hsc")).unwrap();
    assert!(warnings.is_empty());

    let expected_verification = SolveVerification {
        puzzle_canonical_id: "ft_cube:2".to_string(),
        puzzle_version: "1.0.0".to_string(),
        solution_stm: 27,
        used_filters: true,
        used_macros: false,
        timestamps: Timestamps {
            scramble_generation: Some("2025-12-30T06:00:32.581Z".parse().unwrap()),
            inspection_start: Some("2025-12-30T06:00:32.620Z".parse().unwrap()),
            blindfold_don: None,
            solve_start: Some("2025-12-30T06:00:35.841Z".parse().unwrap()),
            solve_completion: Some("2025-12-30T06:00:51.251Z".parse().unwrap()),
        },
        verified_timestamps: VerifiedTimestamps {
            scramble_range_start: None,
            scramble_range_end: None,
            completion: None,
        },
        durations: Durations {
            scramble_network_latency: None,
            scramble_application: Some(TimeDelta::new(0, 39000000).unwrap()),
            inspection: Some(TimeDelta::new(3, 221000000).unwrap()),
            speedsolve: Some(TimeDelta::new(15, 410000000).unwrap()),
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
    assert_eq!(actual_verification, Ok(expected_verification));

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
        actual_verification,
        Err(SolveVerificationError::ScrambleSeedMismatch),
    );
}
