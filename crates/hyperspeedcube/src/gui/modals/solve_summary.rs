use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use egui::{NumExt, include_image};
use hypercubing_leaderboards_client::{
    AutoVerifySubmission, BestSolves, BestSolvesRequest, Leaderboards,
};
use hyperprefs::Preferences;
use hyperpuzzle::chrono::{TimeDelta, Utc};
use hyperpuzzle::verification::SolveVerification;
use hyperpuzzle::{FloatMinMaxIteratorExt, Puzzle, chrono};
use hyperpuzzle_log::verify::SolveVerificationError;
use hyperpuzzle_log::{LogFile, Solve};
use hyperpuzzle_view::PuzzleSimulation;
use hyperstats::{NewPbs, PuzzlePBs, StatsDb};
use parking_lot::Mutex;

use crate::L;
use crate::gui::App;
use crate::gui::ext::ResponseExt;
use crate::gui::markdown::md;
use crate::gui::util::{
    GuiRoundingExtRect, MDI_BIG_SIZE, MDI_STYLE_ICON_ROTATE_SQUARE, text_width,
};
use crate::leaderboards::LeaderboardsClientState;
use crate::util::centiseconds_to_string;

#[derive(Debug, Default, Clone)]
struct LeaderboardSubmitForm {
    solver_notes: String,
    computer_assisted: bool,
    will_upload_video: bool,
}

impl egui::Widget for &mut LeaderboardSubmitForm {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let l = L.solve_summary.leaderboards;
        ui.vertical(|ui| {
            ui.checkbox(&mut self.computer_assisted, l.computer_assisted_fmc);
            ui.checkbox(&mut self.will_upload_video, l.will_upload_video);
            ui.label(l.notes);
            ui.text_edit_multiline(&mut self.solver_notes);
        })
        .response
    }
}

pub struct SolveSummaryModal {
    sim: Arc<Mutex<PuzzleSimulation>>,
    puzzle: Arc<Puzzle>,

    replay: Solve,
    digest: Arc<[u8]>,
    verification: SolveVerification,
    file_path: Result<PathBuf, String>,
    file_name: String,
    /// This contains `Ok(Some(..))` on the first frame after receiving a
    /// response from the Time Stamp Authority. Then the timestamp is stored in
    /// the replay file and this value is replaced with `Ok(None)`.
    timestamp_signature: RequestState<Option<String>>,
    new_pbs: NewPbs,
    /// Saved personal bests.
    old_pbs: PuzzlePBs,
    /// Leaderboards personal bests.
    leaderboard_pbs: RequestState<BestSolves>,
    /// Leaderboards world records.
    leaderboard_wrs: RequestState<BestSolves>,

    best_solves_request: BestSolvesRequest,

    save_result: Option<Result<(), String>>,

    is_leaderboard_eligible: bool,
    leaderboard_submit_form: LeaderboardSubmitForm,
    leaderboard_submission: RequestState<String>,
}

impl SolveSummaryModal {
    /// Constructs a new solve summary modal from a puzzle simulation that was
    /// just solved.
    pub fn new(
        sim: &Arc<Mutex<PuzzleSimulation>>,
        stats: &mut StatsDb,
        prefs: &Preferences,
    ) -> Result<Self, SolveVerificationError> {
        let mut sim_guard = sim.lock();

        let puzzle = Arc::clone(sim_guard.puzzle_type());

        let replay = sim_guard.serialize(true);

        let verification = hyperpuzzle_log::verify::verify(
            &hyperpuzzle::catalog(),
            &replay,
            hyperpuzzle_log::verify::VerificationOptions::QUICK,
        )?;

        // IIFE to mimic try_block
        let completion_timestamp_str = verification
            .timestamps
            .solve_completion
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let file_name = hyperpaths::solve_autosave_filename(
            &completion_timestamp_str,
            verification.solution_stm,
        );
        let file_path = hyperpaths::solve_autosave_path(
            &replay.puzzle.id,
            &completion_timestamp_str,
            verification.solution_stm,
        )
        .map_err(|e| e.to_string());

        let new_pbs = stats.check_new_pb(&verification);
        let old_pbs = stats.pbs(&verification.puzzle_canonical_id).clone();

        if new_pbs.first {
            stats.record_first_solve(&verification);
            hyperstats::save(stats);
            sim_guard.start_special_anim();
        }

        let digest = Arc::from(replay.digest_v3());

        let is_leaderboard_eligible = puzzle.meta.tags.has_present("external/leaderboard");

        // Immediately timestamp if not yet timestamped
        let mut timestamp_signature = RequestState::Ok(None);
        if replay.tsa_signature_v3.is_none() {
            if prefs.online_mode {
                try_timestamp(&mut timestamp_signature, Arc::clone(&digest));
            } else {
                timestamp_signature = RequestState::Init;
            }
        }

        let leaderboard_pbs = RequestState::Init;
        let leaderboard_wrs = RequestState::Init;

        let best_solves_request = BestSolvesRequest {
            hsc_puzzle_id: Some(verification.puzzle_canonical_id.clone()),
            blind: verification.durations.blindsolve.is_some(),
            filters: Some(verification.used_filters),
            macros: Some(verification.used_macros),
            ..Default::default()
        };

        Ok(Self {
            sim: Arc::clone(sim),
            puzzle,

            replay,
            digest,
            verification,
            file_name,
            file_path,
            timestamp_signature,
            new_pbs,
            old_pbs,
            leaderboard_pbs,
            leaderboard_wrs,

            best_solves_request,

            save_result: sim_guard.saved_to_autonamed_file.then_some(Ok(())),

            is_leaderboard_eligible,
            leaderboard_submit_form: LeaderboardSubmitForm::default(),
            leaderboard_submission: if let Some(url) = &sim_guard.leaderboard_url {
                RequestState::Ok(url.clone())
            } else {
                RequestState::Init
            },
        })
    }

    /// Shows the modal and returns whether to close it.
    pub fn show(&mut self, ui: &mut egui::Ui, app: &mut App) {
        ui.vertical_centered(|ui| {
            ui.heading(L.solve_summary.title.with(&self.puzzle.meta.name));

            let title_rect = ui.min_rect();

            let button_center =
                title_rect.right_center() - egui::vec2(title_rect.height() / 2.0, 0.0);
            let button_size = egui::Vec2::splat(ui.spacing().icon_width);
            let button_rect = egui::Rect::from_center_size(button_center, button_size);
            let button_rect = button_rect.round_to_pixels_ui_inward(ui.ctx());

            if crate::gui::util::close_button(ui, button_rect).clicked() {
                ui.close();
            }
        });

        ui.separator();

        egui::ScrollArea::both()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 1.0); // TODO: workaround for https://github.com/emilk/egui/issues/7910
                egui::Frame::new()
                    .inner_margin(egui::vec2(24.0, 12.0))
                    .show(ui, |ui| {
                        ui.group(|ui| {
                            egui::ScrollArea::horizontal()
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    self.show_comparison_table(ui, &app.leaderboards);
                                });
                        });

                        ui.group(|ui| {
                            ui.set_width(ui.available_width());
                            self.show_save_button(ui, &mut app.stats);
                        });
                        ui.group(|ui| {
                            ui.set_width(ui.available_width());
                            if self.is_leaderboard_eligible {
                                ui.add(crate::gui::components::LeaderboardsUi(&app.leaderboards));
                                ui.separator();
                                self.show_timestamp_button(ui);
                                self.show_leaderboard_submit_ui(ui, &app.leaderboards.lock());
                            } else {
                                self.show_timestamp_button(ui);
                                ui.label(L.solve_summary.leaderboards.ineligible);
                            }
                        });

                        ui.add_space(20.0);

                        md(ui, L.solve_summary.reopen_hint);

                        self.show_special_puzzle_message(ui, app);
                    });
            });
    }

    fn show_save_button(&mut self, ui: &mut egui::Ui, stats: &mut StatsDb) {
        let l = L.solve_summary.save;
        ui.horizontal(|ui| {
            match &mut self.save_result {
                Some(Ok(())) => {
                    let mut sim = self.sim.lock();
                    sim.saved_to_autonamed_file = true;
                    sim.clear_unsaved_changes();
                    ui.add(mdi!(ui, CONTENT_SAVE_CHECK, 18));
                    ui.label(l.success);
                }
                Some(Err(e)) => {
                    let error_fg_color = ui.visuals().error_fg_color;
                    ui.add(mdi!(error_fg_color, CONTENT_SAVE_ALERT_OUTLINE, 18));
                    ui.colored_label(error_fg_color, l.error).on_hover_text(&*e);
                    if ui.button(L.try_again).clicked() {
                        self.save_result = None;
                    }
                }
                None => {
                    if ui.button((mdi!(ui, CONTENT_SAVE), l.button)).clicked() {
                        self.save_result = Some(self.try_save(stats));
                    }
                }
            }
            if let Ok(dir_path) = hyperpaths::solves_dir_for_puzzle(&self.puzzle.meta.id) {
                if self.save_result == Some(Ok(())) {
                    if let Ok(file_path) = &self.file_path
                        && ui
                            .add(egui::Hyperlink::new(l.show_this_saved_solve))
                            .clicked()
                    {
                        if let Err(e) = open_with::show_in_folder(file_path.clone()) {
                            crate::error_dialog(L.error_dialog.opening_folder, e);
                        }
                    }
                } else if ui.add(egui::Hyperlink::new(l.show_saved_solves)).clicked() {
                    crate::open_dir(&dir_path);
                }
            }
        });
    }

    fn show_leaderboard_submit_ui(
        &mut self,
        ui: &mut egui::Ui,
        leaderboards: &LeaderboardsClientState,
    ) {
        let is_signed_in = matches!(leaderboards, LeaderboardsClientState::SignedIn(_));

        ui.add_enabled(
            matches!(self.leaderboard_submission, RequestState::Init) && is_signed_in,
            &mut self.leaderboard_submit_form,
        )
        .on_disabled_hover_text(L.solve_summary.leaderboards.needs_sign_in);

        let l = L.solve_summary.leaderboards;
        let icon_size = 18.0;
        ui.horizontal(|ui| {
            if !is_signed_in {
                ui.disable();
            }

            ui.set_min_height(f32::max(ui.spacing().interact_size.y, icon_size));
            self.leaderboard_submission.try_recv();

            match &mut self.leaderboard_submission {
                RequestState::Init => {
                    if ui.button((mdi!(ui, UPLOAD), l.submit)).clicked()
                        && let LeaderboardsClientState::SignedIn(lb) = leaderboards
                    {
                        let data_to_submit = AutoVerifySubmission {
                            program_abbr: "HSC2".to_string(),
                            solver_notes: self.leaderboard_submit_form.solver_notes.to_string(),
                            computer_assisted: self.leaderboard_submit_form.computer_assisted,
                            will_upload_video: self.leaderboard_submit_form.will_upload_video,
                            log_file_name: self.file_name.clone(),
                            log_file_contents: LogFile {
                                program: Some(crate::PROGRAM.clone()),
                                solves: vec![self.replay.clone()],
                            }
                            .serialize(),
                        };
                        let lb = Arc::clone(lb);
                        let sim = Arc::clone(&self.sim);
                        self.leaderboard_submission.request_async(move || {
                            let result = lb
                                .submit_solve_to_auto_verify(data_to_submit)
                                .map_err(|e| e.to_string());
                            if let Ok(url) = &result {
                                sim.lock().leaderboard_url = Some(url.clone());
                            }
                            result
                        });
                    }
                }
                RequestState::Waiting(_) => {
                    ui.spinner();
                    ui.label(l.waiting);
                }
                RequestState::Ok(url) => {
                    ui.add(mdi!(ui, CHECK, icon_size));
                    ui.hyperlink_to(l.success, url);
                }
                RequestState::Err(e) => {
                    let error_fg_color = ui.visuals().error_fg_color;
                    ui.add(mdi!(error_fg_color, ALERT_OUTLINE, icon_size));
                    ui.colored_label(error_fg_color, l.error).on_hover_text(&*e);
                    if ui.button(L.try_again).clicked() {
                        self.leaderboard_submission = RequestState::Init;
                    }
                }
            };
        });
    }

    fn show_timestamp_button(&mut self, ui: &mut egui::Ui) {
        self.try_recv_timestamp();

        let l = &L.solve_summary.timestamp;
        let icon_size = 18.0;
        ui.horizontal(|ui| {
            ui.set_min_height(f32::max(ui.spacing().interact_size.y, icon_size));

            match &mut self.timestamp_signature {
                RequestState::Init => {
                    if ui
                        .button((mdi!(ui, CLOCK_CHECK), l.button))
                        .on_i18n_hover_explanation(&l.hover)
                        .clicked()
                    {
                        try_timestamp(&mut self.timestamp_signature, Arc::clone(&self.digest));
                    }
                }
                RequestState::Waiting(_) => {
                    ui.spinner();
                    ui.label(l.waiting);
                }
                RequestState::Ok(_) => {
                    ui.add(mdi!(ui, CLOCK_CHECK, icon_size));
                    ui.label(l.success);
                }
                RequestState::Err(e) => {
                    let error_fg_color = ui.visuals().error_fg_color;
                    ui.add(mdi!(ui, CLOCK_ALERT_OUTLINE, icon_size).tint(error_fg_color));
                    ui.colored_label(error_fg_color, l.error).on_hover_text(&*e);
                    if ui.button(L.try_again).clicked() {
                        self.timestamp_signature = RequestState::Init;
                    }
                }
            }
        });
    }

    fn try_recv_timestamp(&mut self) {
        self.timestamp_signature.try_recv();
        if let RequestState::Ok(signature @ Some(_)) = &mut self.timestamp_signature {
            let mut sim = self.sim.lock();
            sim.tsa_signature_v3 = signature.take();
            self.replay.tsa_signature_v3 = sim.tsa_signature_v3.clone();

            // Mark as not yet saved
            self.save_result = None;
            sim.saved_to_autonamed_file = false;
            sim.mark_unsaved();
        }
    }

    fn show_comparison_table(
        &mut self,
        ui: &mut egui::Ui,
        leaderboards: &Arc<Mutex<LeaderboardsClientState>>,
    ) {
        let lb = match &*leaderboards.lock() {
            LeaderboardsClientState::SignedIn(lb) => Some(Arc::clone(lb)),
            _ => None,
        };

        let durations = self.verification.durations;
        let is_blind = self.verification.durations.blindsolve.is_some();

        let this_speed =
            Option::or(durations.speedsolve, durations.blindsolve).map(timedelta_to_centiseconds);
        let this_move_count = Some(self.verification.solution_stm as i64);

        let speed_pb = if is_blind {
            self.old_pbs.blind.as_ref()
        } else {
            self.old_pbs.speed.as_ref()
        };
        let fmc_pb = self.old_pbs.fmc.as_ref();

        // Fetch leaderboard personal bests.
        if matches!(self.leaderboard_pbs, RequestState::Init)
            && let Some(lb) = lb.clone()
        {
            let request = self.best_solves_request.clone();
            self.leaderboard_pbs
                .request_async(move || lb.get_best_solves(&request).map_err(|e| e.to_string()));
        }
        self.leaderboard_pbs.try_recv();

        // Fetch leaderboard world records.
        if matches!(self.leaderboard_wrs, RequestState::Init) {
            let request = self.best_solves_request.clone();
            self.leaderboard_wrs.request_async(move || {
                Leaderboards::get_world_records(crate::leaderboards::LEADERBOARDS_DOMAIN, &request)
                    .map_err(|e| e.to_string())
            });
        }
        self.leaderboard_wrs.try_recv();

        if lb.is_none() {
            self.leaderboard_pbs = RequestState::Init;
        }

        let left_column_width = [
            L.solve_summary.table.this_solve,
            L.solve_summary.table.saved_pb,
            L.solve_summary.table.leaderboard_pb,
            L.solve_summary.table.world_record,
        ]
        .into_iter()
        .map(|text| text_width(ui, text) + ui.spacing().item_spacing.x + MDI_BIG_SIZE.x)
        .max_float()
        .unwrap_or(0.0);

        let header_width = |text: &egui::RichText| -> f32 {
            MDI_BIG_SIZE.x * 0.75 + ui.spacing().item_spacing.x + text_width(ui, text.clone())
        };

        let mut this_solve_speed = SolveMetric::new_speed(this_speed);
        let mut saved_pb_solve_speed = SolveMetric::new_speed(speed_pb.map(|pb| pb.duration / 10))
            .with_file_path(speed_pb.and_then(|pb| pb.abs_path().ok()))
            .compare_to_new_solve(SolveMetricCategory::Saved, this_speed);
        // IIFE to mimic try_block
        let mut leaderboard_pb_solve_speed =
            SolveMetric::from_speed_lb_solve((|| self.leaderboard_pbs.as_ok()?.speed.as_ref())())
                .compare_to_new_solve(SolveMetricCategory::LeaderboardPb, this_speed);
        let mut wr_solve_speed =
            SolveMetric::from_speed_lb_solve((|| self.leaderboard_wrs.as_ok()?.speed.as_ref())())
                .compare_to_new_solve(SolveMetricCategory::WorldRecord, this_speed);

        let speed_header_text = egui::RichText::new(L.solve_summary.table.time).size(16.0);
        let speed_width = f32::max(
            header_width(&speed_header_text),
            SolveMetric::max_width(
                ui,
                [
                    &this_solve_speed,
                    &saved_pb_solve_speed,
                    &leaderboard_pb_solve_speed,
                    &wr_solve_speed,
                ],
            ),
        );

        this_solve_speed.align_to_max_width(speed_width);
        saved_pb_solve_speed.align_to_max_width(speed_width);
        leaderboard_pb_solve_speed.align_to_max_width(speed_width);
        wr_solve_speed.align_to_max_width(speed_width);

        let mut this_solve_move_count = SolveMetric::new_move_count(this_move_count);
        let mut saved_pb_solve_move_count = SolveMetric::new_move_count(fmc_pb.map(|pb| pb.stm))
            .with_file_path(fmc_pb.and_then(|pb| pb.abs_path().ok()))
            .compare_to_new_solve(SolveMetricCategory::Saved, this_move_count);
        // IIFE to mimic try_block
        let mut leaderboard_pb_solve_move_count = if self.leaderboard_submit_form.computer_assisted
        {
            SolveMetric::from_fmc_lb_solve((|| self.leaderboard_pbs.as_ok()?.fmcca.as_ref())())
        } else {
            SolveMetric::from_fmc_lb_solve((|| self.leaderboard_pbs.as_ok()?.fmc.as_ref())())
        }
        .compare_to_new_solve(SolveMetricCategory::LeaderboardPb, this_move_count);
        let mut wr_solve_move_count = if self.leaderboard_submit_form.computer_assisted {
            SolveMetric::from_fmc_lb_solve((|| self.leaderboard_wrs.as_ok()?.fmcca.as_ref())())
        } else {
            SolveMetric::from_fmc_lb_solve((|| self.leaderboard_wrs.as_ok()?.fmc.as_ref())())
        }
        .compare_to_new_solve(SolveMetricCategory::WorldRecord, this_move_count);

        let move_count_header_text =
            egui::RichText::new(L.solve_summary.table.move_count).size(16.0);
        let move_count_width = f32::max(
            header_width(&move_count_header_text),
            SolveMetric::max_width(
                ui,
                [
                    &this_solve_move_count,
                    &saved_pb_solve_move_count,
                    &leaderboard_pb_solve_move_count,
                    &wr_solve_move_count,
                ],
            ),
        );

        this_solve_move_count.align_to_max_width(move_count_width);
        saved_pb_solve_move_count.align_to_max_width(move_count_width);
        leaderboard_pb_solve_move_count.align_to_max_width(move_count_width);
        wr_solve_move_count.align_to_max_width(move_count_width);

        // I tried using `egui_extras::TableBuilder` and it wasn't worth it. I
        // ended up needing to compute row widths myself anyway, and it's tricky
        // to get rectangles out of it, and we don't care about the fancy
        // rendering options. It's easier to just do everything manually.

        // Compute X coordinate range for each column
        let col = {
            let combined_columns_width = left_column_width + speed_width + move_count_width;
            let min_excess = 40.0 * 2.0;
            ui.set_min_width(combined_columns_width + min_excess);
            let excess = ui.available_width() - combined_columns_width;
            let x0 = ui.max_rect().min.x;
            let x1 = x0 + left_column_width + excess / 2.0;
            let x2 = x1 + move_count_width + excess / 2.0;
            let column_ranges = [
                egui::Rangef::new(x0, x0 + left_column_width),
                egui::Rangef::new(x1, x1 + move_count_width),
                egui::Rangef::new(x2, x2 + speed_width),
            ];
            move |i| column_ranges[i]
        };

        // Compute Y coordinate range for each row
        let row = {
            let row_height = SolveMetric::HEIGHT;
            let row_offset = row_height + ui.spacing().item_spacing.y;
            let y0 = ui.cursor().min.y;
            move |i| {
                let y = y0 + row_offset * i as f32;
                egui::Rangef::new(y, y + row_height)
            }
        };

        let cell = |x, y| egui::Rect::from_x_y_ranges(col(x), row(y));

        // Draw header
        let icon = icon!(ui, MDI_STYLE_ICON_ROTATE_SQUARE, 18);
        ui.put(
            cell(1, 0),
            egui::AtomLayout::new((icon, move_count_header_text))
                .align2(egui::Align2::RIGHT_CENTER),
        )
        .on_i18n_hover_explanation(&L.status_bar.move_count);
        let icon = mdi!(ui, TIMER, 18);
        ui.put(
            cell(2, 0),
            egui::AtomLayout::new((icon, speed_header_text)).align2(egui::Align2::RIGHT_CENTER),
        )
        .on_i18n_hover_explanation(if is_blind {
            &L.status_bar.blindsolve_time
        } else {
            &L.status_bar.time
        });

        fn put_left(
            ui: &mut egui::Ui,
            max_rect: egui::Rect,
            widget: impl egui::Widget,
        ) -> egui::Response {
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(max_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| ui.add(widget),
            )
            .inner
        }

        // New solve
        let row_label = (mdi_big!(ui, NEW_BOX), L.solve_summary.table.this_solve);
        put_left(ui, cell(0, 1), egui::AtomLayout::new(row_label));
        ui.put(cell(1, 1), this_solve_move_count);
        ui.put(cell(2, 1), this_solve_speed);

        // Saved personal best
        let row_label = (mdi_big!(ui, HARDDISK), L.solve_summary.table.saved_pb);
        put_left(ui, cell(0, 2), egui::AtomLayout::new(row_label));
        ui.put(cell(1, 2), saved_pb_solve_move_count);
        ui.put(cell(2, 2), saved_pb_solve_speed);

        if self.is_leaderboard_eligible {
            // Leaderboard personal best
            let row_label = (mdi_big!(ui, MEDAL), L.solve_summary.table.leaderboard_pb);
            put_left(ui, cell(0, 3), egui::AtomLayout::new(row_label));
            let merged_cell = || cell(1, 3).union(cell(2, 3));
            match &self.leaderboard_pbs {
                RequestState::Init => (), // should have been handled already
                RequestState::Waiting(_) => {
                    ui.put(merged_cell(), egui::Spinner::new());
                }
                RequestState::Ok(_) => {
                    ui.put(cell(1, 3), leaderboard_pb_solve_move_count);
                    ui.put(cell(2, 3), leaderboard_pb_solve_speed);
                }
                RequestState::Err(e) => {
                    let e = e.clone();
                    put_left(ui, merged_cell(), |ui: &mut egui::Ui| {
                        ui.scope(|ui| {
                            ui.colored_label(ui.visuals().error_fg_color, e);
                            if ui.button(L.try_again).clicked() {
                                self.leaderboard_pbs = RequestState::Init;
                            }
                        })
                        .response
                    });
                }
            }
            if lb.is_none() {
                ui.put(
                    merged_cell(),
                    crate::gui::components::LeaderboardsUi(leaderboards),
                );
            }

            // Leaderboard world record
            let row_label = (mdi_big!(ui, TROPHY), L.solve_summary.table.world_record);
            put_left(ui, cell(0, 4), egui::AtomLayout::new(row_label));
            let merged_cell = || cell(1, 4).union(cell(2, 4));
            match &self.leaderboard_wrs {
                RequestState::Init => (), // should have been handled already
                RequestState::Waiting(_) => {
                    ui.put(merged_cell(), egui::Spinner::new());
                }
                RequestState::Ok(_) => {
                    ui.put(cell(1, 4), wr_solve_move_count);
                    ui.put(cell(2, 4), wr_solve_speed);
                }
                RequestState::Err(e) => {
                    let e = e.clone();
                    put_left(ui, merged_cell(), |ui: &mut egui::Ui| {
                        ui.scope(|ui| {
                            ui.colored_label(ui.visuals().error_fg_color, e);
                            if ui.button(L.try_again).clicked() {
                                self.leaderboard_wrs = RequestState::Init;
                            }
                        })
                        .response
                    });
                }
            }
        }
    }

    fn show_special_puzzle_message(&self, ui: &mut egui::Ui, app: &mut App) {
        let mut message = None;

        if !self.new_pbs.first {
            return;
        }

        if self.puzzle.meta.tags.has_present("algebraic/trivial") {
            return;
        }

        // TODO: make sure that these IDs generalize across dimensions
        if self.puzzle.meta.id.starts_with("ft_hypercube:")
            && self.puzzle.meta.id != "ft_hypercube:2"
        {
            ui.add_space(20.0);

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                md(ui, L.solve_summary.solved_4d_1);
                if ui.button(L.menu.file.save_log_as).clicked() {
                    app.save_file_as(false);
                }
                md(ui, L.solve_summary.solved_4d_2);
            });

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                egui::CollapsingHeader::new(L.solve_summary.solved_4d_history_title)
                    .show_unindented(ui, |ui| md(ui, L.solve_summary.solved_4d_history));
            });

            self.show_support_request(ui);

            return;
        } else if self.puzzle.meta.id.starts_with("ft_5_cube:") {
            message = Some(L.solve_summary.solved_5d);
        } else if self.puzzle.meta.id.starts_with("ft_6_cube:") {
            message = Some(L.solve_summary.solved_6d);
        } else if self.puzzle.meta.id.starts_with("ft_7_cube:") {
            message = Some(L.solve_summary.solved_7d);
        } else if self.puzzle.meta.id.starts_with("ft_8_cube:") {
            message = Some(L.solve_summary.solved_8d);
        }

        if let Some(m) = message {
            ui.add_space(20.0);

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                md(ui, m);
            });

            self.show_support_request(ui);
        }
    }

    fn show_support_request(&self, ui: &mut egui::Ui) {
        ui.add_space(20.0);

        ui.group(|ui| {
            ui.set_width(ui.available_width());
            md(ui, L.solve_summary.support_request);
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.add(
                    egui::Image::new(include_image!("../../../resources/img/signature.svg"))
                        .tint(ui.visuals().text_color())
                        .fit_to_exact_size(egui::Vec2::INFINITY)
                        .max_height(80.0),
                );
            });

            ui.add_space(20.0);
        });
    }

    fn try_save(&self, stats: &mut StatsDb) -> Result<(), String> {
        let file_path = self.file_path.clone()?;

        if let Some(dir) = file_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }

        std::fs::write(
            file_path,
            LogFile {
                program: Some(crate::PROGRAM.clone()),
                solves: vec![self.replay.clone()],
            }
            .serialize(),
        )
        .map_err(|e| e.to_string())?;

        stats.record_new_pb(&self.verification, &self.file_name);
        if let Err(e) = hyperstats::save(stats) {
            crate::error_dialog(L.error_dialog.saving_file, e);
        }

        Ok(())
    }
}

struct SolveMetric {
    /// Time vs. move count.
    kind: SolveMetricKind,
    /// Time (centiseconds) or move count.
    value: Option<u64>,

    file_path: Option<PathBuf>,
    url: Option<String>,

    alignment_width: Option<f32>,

    category: Option<SolveMetricCategory>,
    new_solve_is_better: bool,
}

impl SolveMetric {
    /// Font size, which is assumed to be smaller than [`MDI_BIG_SIZE`].
    const FONT_SIZE: f32 = 16.0;

    const HEIGHT: f32 = MDI_BIG_SIZE.y;

    fn new_speed(speed_cs: Option<impl Into<i64>>) -> Self {
        Self {
            value: speed_cs.map(|n| n.into().at_least(0) as u64),
            ..Self::empty(SolveMetricKind::Time)
        }
    }
    fn new_move_count(move_count: Option<impl Into<i64>>) -> Self {
        Self {
            value: move_count.map(|n| n.into().at_least(0) as u64),
            ..Self::empty(SolveMetricKind::MoveCount)
        }
    }

    fn from_speed_lb_solve(solve: Option<&hypercubing_leaderboards_client::Solve>) -> Self {
        solve
            .map(|s| Self::new_speed(s.speed_cs).with_url(&s.url))
            .unwrap_or(Self::empty(SolveMetricKind::Time))
    }
    fn from_fmc_lb_solve(solve: Option<&hypercubing_leaderboards_client::Solve>) -> Self {
        solve
            .map(|s| Self::new_move_count(s.move_count).with_url(&s.url))
            .unwrap_or(Self::empty(SolveMetricKind::MoveCount))
    }

    fn empty(kind: SolveMetricKind) -> Self {
        Self {
            kind,
            value: None,
            file_path: None,
            url: None,
            alignment_width: None,
            category: None,
            new_solve_is_better: false,
        }
    }

    fn compare_to_new_solve(
        mut self,
        category: SolveMetricCategory,
        new_solve_value: Option<i64>,
    ) -> Self {
        self.category = Some(category);
        self.new_solve_is_better = match (self.value, new_solve_value) {
            (None, None) => false,
            (None, Some(_)) => true,
            (Some(_), None) => false,
            (Some(a), Some(b)) => a > b.at_least(0) as u64,
        };
        self
    }

    fn with_file_path(mut self, file_path: Option<impl Into<PathBuf>>) -> Self {
        self.file_path = file_path.map(|p| p.into());
        self
    }

    fn with_url(mut self, url: impl ToString) -> Self {
        self.url = Some(url.to_string());
        self
    }

    fn rich_text(&self) -> Option<egui::RichText> {
        self.value.map(|n| {
            let s = match self.kind {
                SolveMetricKind::Time => centiseconds_to_string(n, true),
                SolveMetricKind::MoveCount => n.to_string(),
            };
            let rich_text = egui::RichText::new(s).monospace().size(Self::FONT_SIZE);
            if self.new_solve_is_better {
                rich_text.strikethrough()
            } else {
                rich_text
            }
        })
    }

    /// Returns the total width consumed by the widget.
    ///
    /// Space is allocated for the icon even if it is not present.
    fn total_width(&self, ui: &egui::Ui) -> f32 {
        let text_width = match self.rich_text() {
            Some(s) => text_width(ui, s),
            None => 0.0,
        };
        MDI_BIG_SIZE.x + ui.spacing().item_spacing.x + text_width
    }

    /// Returns the maximum width among all widgets, or `0` if none are visible.
    fn max_width<'a>(ui: &egui::Ui, solves: impl IntoIterator<Item = &'a SolveMetric>) -> f32 {
        solves
            .into_iter()
            .map(|s| s.total_width(ui))
            .max_float()
            .unwrap_or(0.0)
    }

    /// Aligns the text to match other text with the given maximum width.
    fn align_to_max_width(&mut self, max_width: f32) {
        self.alignment_width = Some(max_width);
    }
}

impl egui::Widget for SolveMetric {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let l = L.solve_summary.table;
        ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
            let width_to_allocate = self.alignment_width.unwrap_or_else(|| self.total_width(ui));

            ui.allocate_ui_with_layout(
                egui::vec2(width_to_allocate, Self::HEIGHT),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    // Draw text
                    let text_response = self.rich_text().map(|rich_text| {
                        let rich_text = rich_text.color(ui.visuals().text_color());
                        if let Some(url) = self.url {
                            ui.add(
                                egui::Hyperlink::from_label_and_url(rich_text, url)
                                    .open_in_new_tab(true),
                            )
                            .on_hover_text(l.click_to_open_in_browser)
                        } else if let Some(file_path) = self.file_path {
                            let r = ui
                                .add(egui::Link::new(rich_text))
                                .on_hover_text(l.click_to_show_in_file_manager);
                            if r.clicked() {
                                if file_path.is_file() {
                                    if let Err(e) = open_with::show_in_folder(file_path) {
                                        crate::error_dialog(L.error_dialog.opening_folder, e);
                                    }
                                } else {
                                    rfd::MessageDialog::new()
                                        .set_level(rfd::MessageLevel::Error)
                                        .set_title("File not found")
                                        .show();
                                }
                            }
                            r
                        } else {
                            ui.add(egui::Label::new(rich_text))
                        }
                    });

                    // Draw icon
                    if self.new_solve_is_better {
                        let icon =
                            mdi_big!(ui, ALERT_DECAGRAM).tint(egui::Color32::from_rgb(255, 255, 0));
                        match text_response {
                            Some(r) => {
                                let icon_center = r.rect.left_center()
                                    - egui::vec2(
                                        ui.spacing().item_spacing.x + MDI_BIG_SIZE.x / 2.0,
                                        0.0,
                                    );
                                let icon_rect =
                                    egui::Rect::from_center_size(icon_center, MDI_BIG_SIZE);
                                let icon_response = ui.put(icon_rect, icon);
                                if icon_response.hovered() || icon_response.has_focus() {
                                    icon_response.show_tooltip_text(match self.kind {
                                        SolveMetricKind::Time => l.new_solve_is_faster,
                                        SolveMetricKind::MoveCount => l.new_solve_is_shorter,
                                    });
                                }
                            }
                            _ => {
                                let icon_response = ui.add(icon);
                                if let Some(category) = self.category {
                                    icon_response.on_hover_text(match category {
                                        SolveMetricCategory::Saved => l.first_saved_solve,
                                        SolveMetricCategory::LeaderboardPb => {
                                            l.first_leaderboard_pb
                                        }
                                        SolveMetricCategory::WorldRecord => l.first_wr,
                                    });
                                }
                            }
                        };
                    }
                },
            );
        })
        .response
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum SolveMetricKind {
    Time,
    MoveCount,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum SolveMetricCategory {
    Saved,
    LeaderboardPb,
    WorldRecord,
}

/// Value computed on another thread.
#[derive(Debug, Default)]
enum RequestState<T> {
    #[default]
    Init,
    Waiting(mpsc::Receiver<Result<T, String>>),
    Ok(T),
    Err(String),
}

impl<T> RequestState<T> {
    fn try_recv(&mut self) {
        if let RequestState::Waiting(recv) = self {
            match recv.try_recv() {
                Ok(Ok(ok)) => *self = Self::Ok(ok),
                Ok(Err(e)) => *self = Self::Err(e),
                Err(mpsc::TryRecvError::Empty) => (),
                Err(e @ mpsc::TryRecvError::Disconnected) => *self = Self::Err(e.to_string()),
            }
        }
    }

    /// Runs `f` asynchronously and sends the result back.
    ///
    /// This function does not block.
    fn request_async(&mut self, f: impl 'static + Send + FnOnce() -> Result<T, String>)
    where
        T: 'static + Send,
    {
        let (tx, rx) = mpsc::channel();
        *self = RequestState::Waiting(rx);
        std::thread::spawn(move || tx.send(f()));
    }

    /// Returns a reference to the contained `Ok` value, if it exists.
    fn as_ok(&self) -> Option<&T> {
        match self {
            RequestState::Ok(ok) => Some(ok),
            _ => None,
        }
    }
}

fn try_timestamp(output: &mut RequestState<Option<String>>, digest: Arc<[u8]>) {
    output.request_async(move || match hyperpuzzle_log::verify::timestamp(&digest) {
        Ok(s) => Ok(Some(s)),
        Err(e) => Err(e.to_string()),
    });
}

fn timedelta_to_centiseconds(delta: TimeDelta) -> i64 {
    if delta < TimeDelta::zero() {
        0
    } else {
        delta.num_milliseconds() / 10
    }
}

#[cfg(test)]
mod tests {
    use hyperpuzzle::notation::Invert;
    use itertools::Itertools;
    use smallvec::smallvec;

    use super::*;

    #[test]
    fn test_timestamp_and_reload_solve() {
        hyperpuzzle::load_global_catalog();

        let mut prefs = Preferences::default();
        prefs.online_mode = true;

        let puzzle: Arc<Puzzle> = hyperpuzzle::catalog().build_blocking("ft_cube:2").unwrap();
        let mut sim = PuzzleSimulation::new(&puzzle);

        // Scramble the puzzle
        sim.scramble(hyperpuzzle::ScrambleType::Full, prefs.online_mode);
        while sim.scramble_progress().is_some() {
            sleep_ms(10);
        }

        sleep_ms(10);

        // Solve the puzzle
        let twists = hyperpuzzle::notation::parse_notation(
            &sim.get_scramble().as_ref().unwrap().twists,
            hyperpuzzle::notation::Features::MAXIMAL,
        )
        .unwrap()
        .inv_deep()
        .unwrap();
        for twist in twists.0 {
            sim.do_event(hyperpuzzle_view::ReplayEvent::Twists(smallvec![
                twist.into_move().unwrap()
            ]));
        }

        std::thread::sleep(std::time::Duration::from_millis(10));

        // Open solve summary modal
        let sim = Arc::new(Mutex::new(sim));
        let mut stats = StatsDb::default();
        let mut summary = SolveSummaryModal::new(&sim, &mut stats, &prefs).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        // Timestamp solve
        try_timestamp(
            &mut summary.timestamp_signature,
            Arc::clone(&summary.digest),
        );
        while !matches!(summary.timestamp_signature, RequestState::Ok(None)) {
            summary.try_recv_timestamp();
            sleep_ms(10);
        }

        // Save from modal when first opened
        let saved_solve = summary.replay.clone();
        verify_solve(&saved_solve);

        // Save from modal when reopened
        let mut summary = SolveSummaryModal::new(&sim, &mut stats, &prefs).unwrap();
        let saved_solve = summary.replay.clone();
        verify_solve(&saved_solve);

        // Save from simulation
        let saved_solve = sim.lock().serialize(true);
        verify_solve(&saved_solve);

        // Reload simulation and save again
        let mut new_sim = PuzzleSimulation::deserialize(&puzzle, &saved_solve);
        verify_solve(&new_sim.serialize(true));
    }

    #[track_caller]
    fn verify_solve(saved_solve: &Solve) {
        let verify_output = hyperpuzzle_log::verify::verify(
            &hyperpuzzle::catalog(),
            &saved_solve,
            hyperpuzzle_log::verify::VerificationOptions::FULL,
        )
        .unwrap();
        if !verify_output.errors.is_empty() {
            eprintln!("{verify_output:?}");
        }
        assert!(verify_output.errors.is_empty());
    }

    fn sleep_ms(milliseconds: u64) {
        std::thread::sleep(std::time::Duration::from_millis(milliseconds));
    }
}
