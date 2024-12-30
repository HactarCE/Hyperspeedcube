use std::path::PathBuf;

use hyperpuzzle_library::SolveVerification;
use hyperpuzzle_log::{LogFile, Solve};
use hyperstats::NewPbs;

use crate::gui::{util::EguiTempValue, App};

#[derive(Debug, Clone)]
struct SolveCompletePopup {
    solve: Solve,
    puzzle_name: String,
    file_path: PathBuf,
    file_name: String,
    new_pbs: NewPbs,
    verification: SolveVerification,
    saved: bool,
}

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let solve_complete_popup = EguiTempValue::<Option<SolveCompletePopup>>::new(ui);

    if let Some(Some(mut popup)) = solve_complete_popup.get() {
        let r = egui::Modal::new(unique_id!()).show(ui.ctx(), |ui| {
            ui.heading(format!(
                "Yay! You solved the {} in {} twists",
                popup.puzzle_name, popup.verification.solution_stm_count,
            ));

            if let Some(dur) = popup
                .verification
                .blindsolve_duration
                .filter(|_| popup.new_pbs.blind)
            {
                // TODO: prettify this
                ui.label(format!("You set a new blindsolve PB of {dur}"));
            }

            if let Some(dur) = popup
                .verification
                .speedsolve_duration
                .filter(|_| popup.new_pbs.speed)
            {
                ui.label(format!("You set a new speedsolve PB of {dur}"));
            }

            if popup.new_pbs.fmc {
                let stm = popup.verification.solution_stm_count;
                ui.label(format!("You set a new move count PB of {stm} STM"));
            }

            if popup.saved {
                ui.label(format!("Saved to {}", popup.file_name));
            } else {
                if ui.button("Save this solve").clicked() {
                    if popup.new_pbs.any() {
                        app.stats
                            .record_new_pb(&popup.verification, &popup.file_name);
                        hyperstats::save(&app.stats);
                        if let Some(p) = popup.file_path.parent() {
                            std::fs::create_dir_all(p);
                        }
                        if let Ok(()) = std::fs::write(
                            &popup.file_path,
                            LogFile {
                                program: Some(crate::PROGRAM.clone()),
                                solves: vec![popup.solve.clone()],
                            }
                            .serialize(),
                        ) {
                            popup.saved = true;
                            solve_complete_popup.set(Some(Some(popup.clone())));
                        }
                    }
                }
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
                let solve = sim.serialize();
                let verification = hyperpuzzle_library::verify_without_checking_solution(&solve)?;
                let (file_path, file_name) = hyperpaths::solve_autosave_file(
                    &solve.puzzle.id,
                    &verification.time_completed.to_string(),
                    verification.solution_stm_count,
                )
                .ok()?;
                let new_pbs = app.stats.check_new_pb(&verification);

                solve_complete_popup.set(Some(Some(SolveCompletePopup {
                    solve,
                    puzzle_name: sim.puzzle_type().name.clone(),
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
