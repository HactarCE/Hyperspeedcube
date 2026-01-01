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
    replay: Solve,
    digest: Arc<[u8]>,
    signature: Arc<Mutex<SignedState>>,
    puzzle_name: String,
    file_path: PathBuf,
    file_name: String,
    new_pbs: NewPbs,
    verification: SolveVerification,
    saved: bool,
}

impl SolveCompletePopup {
    fn try_sign(&self) {
        let digest = Arc::clone(&self.digest);
        let signature = Arc::clone(&self.signature);
        std::thread::spawn(move || match hyperpuzzle_log::verify::timestamp(&digest) {
            Ok(s) => *signature.lock() = SignedState::Ok(s),
            Err(e) => *signature.lock() = SignedState::Err(e.to_string()),
        });
    }
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

            let mut signature_state = popup.signature.lock();

            let try_sign = match &*signature_state {
                SignedState::Init => ui.small_button("Timestamp").clicked(),
                SignedState::Waiting => {
                    ui.label("Timestamping ...");
                    if ui.small_button("Cancel").clicked() {
                        *signature_state = SignedState::Init;
                    }
                    false
                }
                SignedState::Err(e) => {
                    ui.horizontal(|ui| {
                        ui.label(format!("error timestamping solve: {e}"));
                        ui.small_button("Retry").clicked()
                    })
                    .inner
                }
                SignedState::Ok(s) => {
                    popup.replay.tsa_signature_v1 = Some(s.clone());
                    *signature_state = SignedState::OkDone;
                    false
                }
                SignedState::OkDone => {
                    ui.label("This solve is timestamped");
                    false
                }
            };

            let timestamp_in_progress = matches!(*signature_state, SignedState::Waiting);

            drop(signature_state); // unlock mutex
            if try_sign {
                popup.try_sign();
            }

            ui.add_enabled_ui(!timestamp_in_progress, |ui| {
                if popup.saved {
                    ui.label(format!("Saved to {}", popup.file_name));
                    if let Some(dir_path) = popup.file_path.parent() {
                        if ui.button("Show folder").clicked() {
                            if let Err(e) = opener::open(dir_path) {
                                log::error!("{e}");
                            }
                        }
                    }
                } else {
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
                                solves: vec![popup.replay.clone()],
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
                }
            });

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

                let digest = Arc::from(replay.digest_v1());

                let mut popup = SolveCompletePopup {
                    replay,
                    digest,
                    signature: Arc::new(Mutex::new(SignedState::Init)),
                    puzzle_name: sim.puzzle_type().meta.name.clone(),
                    file_path,
                    file_name,
                    new_pbs,
                    verification,
                    saved: false,
                };

                if app.prefs.online_mode {
                    popup.try_sign();
                }

                solve_complete_popup.set(Some(Some(popup)));
            }
            None::<std::convert::Infallible>
        });
    }
}

#[derive(Debug, Default, Clone)]
enum SignedState {
    #[default]
    Init,
    Waiting,
    Err(String),
    /// Signed successfully (signature must be added to the replay next time the
    /// UI is shown).
    Ok(String),
    /// Signed successfully and the signature has already been added to the
    /// replay.
    OkDone,
}
