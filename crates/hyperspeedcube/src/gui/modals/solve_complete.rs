use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use hyperpuzzle::chrono::Utc;
use hyperpuzzle::verification::SolveVerification;
use hyperpuzzle::{Timestamp, chrono};
use hyperpuzzle_log::{LogFile, Solve};
use hyperpuzzle_view::PuzzleSimulation;
use hyperstats::NewPbs;
use parking_lot::Mutex;

use crate::gui::App;
use crate::gui::util::EguiTempValue;

#[derive(Debug, Clone)]
struct SolveCompletePopup {
    replay: Arc<Mutex<Option<Solve>>>,
    puzzle_name: String,
    file_path: PathBuf,
    file_name: String,
    new_pbs: NewPbs,
    verification: SolveVerification,
    saved: bool,
}

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let solve_complete_popup = EguiTempValue::<Option<SolveCompletePopup>>::new(ui);

    if let Some(Some(mut popup)) = solve_complete_popup.get().filter(|_| {
        app.active_puzzle
            .with_sim(|sim| sim.special_anim().get().is_none())
            .unwrap_or(true)
    }) {
        let r = egui::Modal::new(unique_id!()).show(ui.ctx(), |ui| {
            ui.heading(format!(
                "Yay! You solved the {} in {} twists",
                popup.puzzle_name, popup.verification.solution_stm,
            ));

            if let Some(dur) = popup
                .verification
                .durations
                .blindsolve
                .filter(|_| popup.new_pbs.blind)
            {
                // TODO: prettify this
                ui.label(format!("You set a new blindsolve PB of {dur}"));
            }

            if let Some(dur) = popup
                .verification
                .durations
                .speedsolve
                .filter(|_| popup.new_pbs.speed)
            {
                ui.label(format!("You set a new speedsolve PB of {dur}"));
            }

            if popup.new_pbs.fmc {
                let stm = popup.verification.solution_stm;
                ui.label(format!("You set a new move count PB of {stm} STM"));
            }

            if popup.saved {
                ui.label(format!("Saved to {}", popup.file_name));
                if let Some(dir_path) = popup.file_path.parent() {
                    if ui.button("Show folder").clicked() {
                        if let Err(e) = opener::open(dir_path) {
                            log::error!("{e}");
                        }
                    }
                }
            } else if let Some(replay) = &*popup.replay.lock() {
                if ui.button("Save this solve").clicked() {
                    // Save log file
                    if let Some(p) = popup.file_path.parent() {
                        std::fs::create_dir_all(p);
                    }
                    // TODO: handle error
                    if let Ok(()) = std::fs::write(
                        &popup.file_path,
                        LogFile {
                            program: Some(crate::PROGRAM.clone()),
                            solves: vec![replay.clone()],
                        }
                        .serialize(),
                    ) {
                        popup.saved = true;
                        solve_complete_popup.set(Some(Some(popup.clone())));

                        // Save PBs
                        if popup.new_pbs.any() {
                            app.stats
                                .record_new_pb(&popup.verification, &popup.file_name);
                            hyperstats::save(&app.stats);
                        }
                    }
                }
            } else {
                // TODO: detect when time-stamping fails
                ui.label("Time-stamping ...");
            }

            if ui.button("Close").clicked() {
                solve_complete_popup.set(None);
            }
        });
        if r.should_close() {
            solve_complete_popup.set(None);
        }
    } else {
        app.active_puzzle.with_sim(|sim| {
            if sim.has_been_fully_scrambled() && sim.handle_newly_solved_state() {
                let replay = sim.serialize(true);
                let verification = hyperpuzzle_log::verify::verify(
                    &hyperpuzzle::catalog(),
                    &replay,
                    hyperpuzzle_log::verify::VerificationOptions::QUICK,
                )
                .map_err(|e| log::error!("solve verification error: {e}"))
                .ok()?;
                let (file_path, file_name) = hyperpaths::solve_autosave_file(
                    &replay.puzzle.id,
                    &verification
                        .timestamps
                        .solve_completion
                        .unwrap_or_else(Utc::now)
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    verification.solution_stm,
                )
                .ok()?;
                let new_pbs = app.stats.check_new_pb(&verification);

                if new_pbs.first {
                    sim.start_special_anim();
                }

                let mut solve_to_timestamp = replay;
                let replay = Arc::new(Mutex::new(None));
                let replay_ref = Arc::clone(&replay);

                if app.prefs.online_mode {
                    std::thread::spawn(move || {
                        if let Err(e) = hyperpuzzle_log::verify::timestamp(&mut solve_to_timestamp)
                        {
                            log::error!("error timestamping solve: {e}");
                        }
                        *replay_ref.lock() = Some(solve_to_timestamp);
                    });
                } else {
                    *replay_ref.lock() = Some(solve_to_timestamp);
                }

                // TODO: button to retry timestamping if fails

                solve_complete_popup.set(Some(Some(SolveCompletePopup {
                    replay,
                    puzzle_name: sim.puzzle_type().meta.name.clone(),
                    file_path,
                    file_name,
                    new_pbs,
                    verification,
                    saved: false,
                })));
            }
            None::<std::convert::Infallible>
        });
    }
}
