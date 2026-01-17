use std::sync::Arc;

use egui::AtomExt;
use egui::containers::menu::{MenuButton, MenuConfig};
use hyperpuzzle::ScrambleType;

use super::{AppUi, Tab};
use crate::L;
use crate::gui::components::{PrefsUi, show_leaderboards_ui};
use crate::gui::ext::ResponseExt;
use crate::gui::markdown::md;
use crate::leaderboards::LeaderboardsClientState;

lazy_static! {
    static ref NEWER_RELEASE: Option<self_update::update::Release> = check_for_update()
        .unwrap_or_else(|e| {
            log::error!("error checking for updates: {e}");
            None
        });
}

fn check_for_update() -> Result<Option<self_update::update::Release>, self_update::errors::Error> {
    let release_list = self_update::backends::github::ReleaseList::configure()
        .repo_owner(crate::GITHUB_REPO_OWNER)
        .repo_name(crate::GITHUB_REPO_NAME)
        .build()?
        .fetch()?;
    let current_version = self_update::cargo_crate_version!();
    let mut latest_version = current_version.to_string();
    let mut latest_release = None;
    for r in release_list {
        if self_update::version::bump_is_greater(&latest_version, &r.version).unwrap_or(false) {
            latest_version = r.version.clone();
            latest_release = Some(r);
        }
    }
    Ok(latest_release)
}

pub fn build(ui: &mut egui::Ui, app_ui: &mut AppUi) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            const PROGRAM: &str = concat!("HSC v", env!("CARGO_PKG_VERSION"));
            let version_text = egui::RichText::new(PROGRAM).small();
            let version_button = egui::Button::new(version_text).frame(false);
            if ui.add(version_button).clicked() {
                app_ui.set_tab_state(&Tab::About, !app_ui.has_tab(&Tab::About));
            }

            #[cfg(target_arch = "wasm32")]
            ui.hyperlink_to(L.top_bar.desktop_link, env!("CARGO_PKG_HOMEPAGE"))
                .on_hover_text(L.top_bar.desktop_link_hover);
            #[cfg(not(target_arch = "wasm32"))]
            // Make rustc think that we've used these values.
            let _ = (L.top_bar.desktop_link, L.top_bar.desktop_link_hover);

            egui::warn_if_debug_build(ui);

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if ui.available_width() < width_of_all_menu_buttons(ui) {
                    ui.menu_button(L.menu.title, |ui| draw_menu_buttons(ui, app_ui));
                } else {
                    draw_menu_buttons(ui, app_ui);
                }

                ui.separator();

                show_leaderboards_ui(ui, &app_ui.app.leaderboards);

                if app_ui.app.prefs.check_for_updates
                    && let Some(new_release) = &*NEWER_RELEASE
                {
                    ui.separator();
                    ui.hyperlink_to(
                        format!("Update to v{}", new_release.version),
                        format!(
                            "https://github.com/{}/{}/releases",
                            crate::GITHUB_REPO_OWNER,
                            crate::GITHUB_REPO_NAME,
                        ),
                    );
                }
            });
        });
    });
}

fn width_of_all_menu_buttons(ui: &mut egui::Ui) -> f32 {
    [
        L.menu.file.title,
        L.menu.edit.title,
        L.menu.scramble.title,
        L.menu.settings.title,
        L.menu.tools.title,
        L.menu.puzzles.title,
        L.menu.help.title,
        #[cfg(debug_assertions)]
        L.menu.debug.title,
    ]
    .iter()
    .map(|text| menu_button_width(ui, text))
    .sum()
}
fn draw_menu_buttons(ui: &mut egui::Ui, app_ui: &mut AppUi) {
    fn show_tab_toggle(ui: &mut egui::Ui, app_ui: &mut AppUi, tab: Tab) {
        let mut open = app_ui.has_tab(&tab);
        if ui.checkbox(&mut open, tab.menu_name()).clicked() {
            app_ui.set_tab_state(&tab, open);
        }
    }

    ui.menu_button(L.menu.file.title, |ui| {
        if ui.button(L.menu.file.open).clicked()
            && app_ui.confirm_discard(L.confirm_discard.open_another_file)
        {
            app_ui.app.open_file();
        }
        ui.separator();
        let (has_puzzle, has_replay) = app_ui
            .app
            .active_puzzle
            .with_sim(|sim| (true, sim.has_replay()))
            .unwrap_or((false, false));
        let save_buttons_scope = ui.add_enabled_ui(has_puzzle, |ui| {
            if ui.button(L.menu.file.save_log).clicked() {
                app_ui.app.save_file(false);
            }
            if ui.button(L.menu.file.save_log_as).clicked() {
                app_ui.app.save_file_as(false);
            }
            ui.add_enabled_ui(has_replay, |ui| {
                if ui.button(L.menu.file.save_replay).clicked() {
                    app_ui.app.save_file(true);
                }
                if ui.button(L.menu.file.save_replay_as).clicked() {
                    app_ui.app.save_file_as(true);
                }
            });
            ui.separator();
            if ui.button(L.menu.file.copy_hsc_log).clicked()
                && let Some(copy_text) = app_ui.app.serialize_puzzle_log(false)
            {
                ui.ctx().copy_text(copy_text);
            }
            ui.add_enabled_ui(has_replay, |ui| {
                if ui.button(L.menu.file.copy_hsc_replay).clicked()
                    && let Some(copy_text) = app_ui.app.serialize_puzzle_log(true)
                {
                    ui.ctx().copy_text(copy_text);
                }
            });
        });

        if save_buttons_scope.response.contains_pointer() {
            egui::Tooltip::always_open(
                ui.ctx().clone(),
                ui.layer_id(),
                save_buttons_scope.response.id,
                egui::PopupAnchor::Position(
                    save_buttons_scope.response.rect.right_top() + egui::vec2(10.0, 0.0),
                ),
            )
            .gap(0.0)
            .show(|ui| {
                md(ui, L.help.log_vs_replay);
                if has_puzzle && !has_replay {
                    ui.colored_label(ui.visuals().warn_fg_color, L.help.cant_save_replay);
                }
            });
        }

        ui.separator();
        if ui.button(L.menu.file.exit).clicked() && app_ui.confirm_discard(L.confirm_discard.exit) {
            todo!("exit, but not on web");
        }
    });

    menu_button_that_stays_open(L.menu.edit.title, ui, |ui| {
        let undo_button = egui::Button::new(L.menu.edit.undo_twist);
        if ui.add_enabled(app_ui.app.has_undo(), undo_button).clicked() {
            app_ui.app.undo();
        }
        let redo_button = egui::Button::new(L.menu.edit.redo_twist);
        if ui.add_enabled(app_ui.app.has_redo(), redo_button).clicked() {
            app_ui.app.redo();
        }
        ui.separator();
        if ui.button(L.menu.edit.reset_puzzle).clicked()
            && app_ui.confirm_discard(L.confirm_discard.reset_puzzle)
        {
            ui.close();
            app_ui.app.reset_puzzle();
        }
    });
    menu_button_that_stays_open(L.menu.scramble.title, ui, |ui| {
        let can_scramble = app_ui
            .app
            .active_puzzle
            .with_view(|view| view.puzzle().can_scramble())
            .unwrap_or(false);
        let full_scramble_button = egui::Button::new(L.menu.scramble.full);
        if ui.add_enabled(can_scramble, full_scramble_button).clicked() {
            ui.close();
            if app_ui.confirm_discard(L.confirm_discard.scramble) {
                app_ui.app.scramble(ScrambleType::Full);
            }
        }
        ui.separator();
        let scramble_1_button = egui::Button::new(L.menu.scramble.one);
        if ui.add_enabled(can_scramble, scramble_1_button).clicked() {
            ui.close();
            if app_ui.confirm_discard(L.confirm_discard.scramble) {
                app_ui.app.scramble(ScrambleType::Partial(1));
            }
        }
        let scramble_2_button = egui::Button::new(L.menu.scramble.two);
        if ui.add_enabled(can_scramble, scramble_2_button).clicked() {
            ui.close();
            if app_ui.confirm_discard(L.confirm_discard.scramble) {
                app_ui.app.scramble(ScrambleType::Partial(2));
            }
        }
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::Scrambler);
    });
    menu_button_that_stays_open(L.menu.settings.title, ui, |ui| {
        show_tab_toggle(ui, app_ui, Tab::Colors);
        show_tab_toggle(ui, app_ui, Tab::Styles);
        show_tab_toggle(ui, app_ui, Tab::View);
        show_tab_toggle(ui, app_ui, Tab::Animations);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::Interaction);
        show_tab_toggle(ui, app_ui, Tab::Keybinds);
        show_tab_toggle(ui, app_ui, Tab::Mousebinds);
        ui.separator();

        let mut changed = false;
        let mut prefs_ui = PrefsUi {
            ui,
            current: &mut app_ui.app.prefs,
            defaults: None,
            changed: &mut changed,
        };
        prefs_ui.checkbox(&L.prefs.record_time, access!(.record_time));
        prefs_ui.checkbox(&L.prefs.online_mode, access!(.online_mode));
        app_ui.app.prefs.needs_save |= changed;

        // TODO: add "auto" mode that follows OS
        egui::global_theme_preference_buttons(ui);
    });
    menu_button_that_stays_open(L.menu.tools.title, ui, |ui| {
        show_tab_toggle(ui, app_ui, Tab::Camera);
        show_tab_toggle(ui, app_ui, Tab::PieceFilters);
        show_tab_toggle(ui, app_ui, Tab::Timer);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::Macros);
        show_tab_toggle(ui, app_ui, Tab::MoveInput);
        show_tab_toggle(ui, app_ui, Tab::Timeline);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::PuzzleControls);
        show_tab_toggle(ui, app_ui, Tab::ModifierKeys);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::Scrambler);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::ImageGenerator);
    });
    menu_button_that_stays_open(L.menu.puzzles.title, ui, |ui| {
        show_tab_toggle(ui, app_ui, Tab::PuzzleCatalog);
        show_tab_toggle(ui, app_ui, Tab::PuzzleInfo);

        ui.separator();

        let r = ui.checkbox(
            &mut app_ui.app.prefs.show_experimental_puzzles,
            L.menu.puzzles.show_experimental,
        );
        app_ui.app.prefs.needs_save |= r.changed();

        ui.menu_button(L.menu.puzzles.custom, |ui| {
            if let Ok(hps_dir) = hyperpaths::hps_dir()
                && ui
                    .button(L.menu.puzzles.show_hps_dir.label)
                    .on_i18n_hover_explanation(&L.menu.puzzles.show_hps_dir)
                    .clicked()
            {
                ui.close();
                crate::open_dir(hps_dir);
            }
            #[cfg(not(target_arch = "wasm32"))]
            if ui
                .button(L.menu.puzzles.extract_hps.label)
                .on_i18n_hover_explanation(&L.menu.puzzles.extract_hps)
                .clicked()
            {
                ui.close();
                if let Some(mut dir_path) = rfd::FileDialog::new()
                    .set_title(L.menu.puzzles.extract_hps.label)
                    .pick_folder()
                {
                    dir_path.push("hps");
                    match hyperpuzzlescript::extract_builtin_files(&dir_path) {
                        Ok(()) => crate::open_dir(&dir_path),
                        Err(e) => log::error!("Error extracting built-in HPS files: {e}"),
                    }
                }
            }

            show_tab_toggle(ui, app_ui, Tab::HpsLogs);
            show_tab_toggle(ui, app_ui, Tab::DevTools);
        });
    });
    menu_button_that_stays_open(L.menu.help.title, ui, |ui| {
        ui.heading(L.menu.help.guides);
        let _ = ui.button("Welcome");
        show_tab_toggle(ui, app_ui, Tab::About);
        ui.separator();
        show_tab_toggle(ui, app_ui, Tab::KeybindsReference);
    });
    #[cfg(debug_assertions)]
    menu_button_that_stays_open(L.menu.debug.title, ui, |ui| {
        show_tab_toggle(ui, app_ui, Tab::Debug);
    });
}

fn menu_button_width(ui: &egui::Ui, text: &str) -> f32 {
    super::util::text_width(ui, text)
        + ui.spacing().button_padding.x * 2.0
        + ui.spacing().item_spacing.x
}

fn menu_button_that_stays_open<'a, R>(
    atoms: impl egui::IntoAtoms<'a>,
    ui: &mut egui::Ui,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Response, Option<egui::InnerResponse<R>>) {
    MenuButton::new(atoms)
        .config(MenuConfig::default().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside))
        .ui(ui, content)
}
