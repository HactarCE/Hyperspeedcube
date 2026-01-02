use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use hypercubing_leaderboards_client::{AutoVerifySubmission, Leaderboards};
use hyperpuzzle::chrono::Utc;
use hyperpuzzle::verification::SolveVerification;
use hyperpuzzle::{Timestamp, chrono};
use hyperpuzzle_log::{LogFile, Solve};
use hyperpuzzle_view::PuzzleSimulation;
use hyperstats::NewPbs;
use parking_lot::Mutex;

use crate::gui::App;
use crate::gui::util::EguiTempValue;
use crate::leaderboards::LeaderboardsClientState;

#[derive(Debug, Clone)]
struct SolveCompletePopup {
    replay: Solve,
    puzzle_name: String,
    file_path: PathBuf,
    file_name: String,
    new_pbs: NewPbs,
    verification: SolveVerification,
    saved: bool,

    signature: Arc<Mutex<SignedState>>,
    digest: Arc<[u8]>,

    solver_notes: String,
    computer_assisted: bool,
    will_upload_video: bool,
    submission: Arc<Mutex<SubmissionState>>,
}

impl SolveCompletePopup {
    fn try_sign(&self) {
        *self.signature.lock() = SignedState::Waiting;
        let signature = Arc::clone(&self.signature);
        let digest = Arc::clone(&self.digest);
        std::thread::spawn(move || match hyperpuzzle_log::verify::timestamp(&digest) {
            Ok(s) => *signature.lock() = SignedState::Ok(s),
            Err(e) => *signature.lock() = SignedState::Err(e.to_string()),
        });
    }

    fn try_submit(&self, leaderboards: &Arc<Leaderboards>) {
        *self.submission.lock() = SubmissionState::Waiting;
        let data_to_submit = AutoVerifySubmission {
            program_abbr: "HSC2".to_string(),
            solver_notes: self.solver_notes.to_string(),
            computer_assisted: self.computer_assisted,
            will_upload_video: self.will_upload_video,
            log_file_name: self
                .file_name
                .rsplit_once('/')
                .unwrap_or(("", &self.file_name))
                .1
                .to_string(),
            log_file_contents: LogFile {
                program: Some(crate::PROGRAM.clone()),
                solves: vec![self.replay.clone()],
            }
            .serialize(),
        };
        let submission = Arc::clone(&self.submission);
        let leaderboards = Arc::clone(&leaderboards);
        std::thread::spawn(move || {
            match leaderboards.submit_solve_to_auto_verify(data_to_submit) {
                Ok(url) => *submission.lock() = SubmissionState::Ok(url.to_string()),
                Err(e) => *submission.lock() = SubmissionState::Err(e.to_string()),
            }
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

            ui.separator();

            let mut signature_state = popup.signature.lock();
            let mut new_signature = None;
            let try_sign = ui
                .horizontal(|ui| match &*signature_state {
                    SignedState::Init => ui.small_button("Timestamp").clicked(),
                    SignedState::Waiting => {
                        ui.label("Timestamping ...");
                        if ui.small_button("Cancel").clicked() {
                            *signature_state = SignedState::Init;
                        }
                        false
                    }
                    SignedState::Err(e) => {
                        ui.label(format!("error timestamping solve: {e}"));
                        ui.small_button("Retry").clicked()
                    }
                    SignedState::Ok(s) => {
                        new_signature = Some(s.clone());
                        *signature_state = SignedState::OkDone;
                        false
                    }
                    SignedState::OkDone => {
                        ui.label("This solve is timestamped");
                        false
                    }
                })
                .inner;
            if let Some(s) = new_signature {
                popup.replay.tsa_signature_v1 = Some(s);
                solve_complete_popup.set(Some(Some(popup.clone())));
            }

            let timestamp_in_progress = matches!(*signature_state, SignedState::Waiting);
            let timestamp_ok = matches!(*signature_state, SignedState::OkDone);

            drop(signature_state); // unlock mutex
            if try_sign {
                popup.try_sign();
            }

            ui.separator();

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

            ui.separator();

            ui.label(
                "Please do not submit a solve to the leaderboard unless it is a personal best.",
            );

            ui.add_enabled_ui(timestamp_ok, |ui| {
                crate::gui::components::show_leaderboards_ui(ui, &app.leaderboards);
                if let LeaderboardsClientState::SignedIn(lb) = &*app.leaderboards.lock() {
                    let mut changed = false;
                    let try_submit = match &*popup.submission.lock() {
                        SubmissionState::Init => {
                            changed |= ui
                                .checkbox(&mut popup.computer_assisted, "Computer-assisted FMC")
                                .changed();
                            changed |= ui
                                .checkbox(
                                    &mut popup.will_upload_video,
                                    "I want to add a video by editing the submission later",
                                )
                                .changed();
                            ui.label("Notes (optional)");
                            changed |= ui.text_edit_multiline(&mut popup.solver_notes).changed();
                            ui.button("Submit to leaderboards").clicked()
                        }
                        SubmissionState::Waiting => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Submitting ...");
                                false
                            })
                            .inner
                        }
                        SubmissionState::Err(e) => {
                            ui.label(format!("error submitting solve: {e}"));
                            ui.button("Retry").clicked()
                        }
                        SubmissionState::Ok(s) => {
                            ui.horizontal(|ui| {
                                ui.label("Submitted!");
                                ui.hyperlink_to("View your submission", s);
                                false
                            })
                            .inner
                        }
                    };
                    if try_submit {
                        popup.try_submit(&lb);
                        changed = true;
                    }
                    if changed {
                        solve_complete_popup.set(Some(Some(popup.clone())));
                    }
                }
            })
            .response
            .on_disabled_hover_text(
                "You must timestamp this solve before submitting to the leaderboards",
            );

            ui.separator();

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
                    puzzle_name: sim.puzzle_type().meta.name.clone(),
                    file_path,
                    file_name,
                    new_pbs,
                    verification,
                    saved: false,

                    signature: Arc::new(Mutex::new(SignedState::Init)),
                    digest,

                    solver_notes: String::new(),
                    computer_assisted: false,
                    will_upload_video: false,
                    submission: Arc::new(Mutex::new(SubmissionState::Init)),
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

#[derive(Debug, Default, Clone)]
enum SubmissionState {
    #[default]
    Init,
    Waiting,
    Err(String),
    Ok(String),
}
