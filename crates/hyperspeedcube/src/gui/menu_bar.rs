use egui::containers::menu::{MenuButton, MenuConfig};
use hyperpuzzle::ScrambleType;

use super::AppUi;
use crate::L;
use crate::gui::components::PrefsUi;
use crate::gui::ext::ResponseExt;
use crate::gui::markdown::md;
use crate::gui::tabs::UtilityTab;
use crate::gui::util::{MDI_SMALL, hyperlink_to, menu_button_that_stays_open};

pub fn build(ui: &mut egui::Ui, app_ui: &mut AppUi) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            const PROGRAM: &str = concat!("HSC v", env!("CARGO_PKG_VERSION"));
            let version_text = egui::RichText::new(PROGRAM).small();
            let version_button = egui::Button::new(version_text).frame(false);
            if ui.add(version_button).clicked() {
                app_ui.activate_docked_utility(UtilityTab::About);
            }

            #[cfg(target_arch = "wasm32")]
            crate::gui::util::hyperlink_to(ui, L.top_bar.desktop_link, env!("CARGO_PKG_HOMEPAGE"))
                .on_hover_text(L.top_bar.desktop_link_hover);
            #[cfg(not(target_arch = "wasm32"))]
            // Make rustc think that we've used these values.
            let _ = (L.top_bar.desktop_link, L.top_bar.desktop_link_hover);

            egui::warn_if_debug_build(ui);

            ui.separator();

            ui.toggle_value(
                &mut app_ui.is_ui_layout_window_visible,
                mdi!(ui, VIEW_DASHBOARD),
            )
            .on_hover_text(L.top_bar.ui_layout_presets);

            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                app_ui.is_ui_layout_window_visible = false;
            }

            super::layout::build_layout_presets_ui(ui, app_ui);

            super::quick_settings::build_quick_settings_ui(ui, &mut app_ui.app);

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let max_rect = ui.max_rect();
                let ui_builder = egui::UiBuilder::new().max_rect(max_rect);
                let r = ui.scope_builder(ui_builder.clone().sizing_pass().invisible(), |ui| {
                    left_menu_ui(ui, app_ui, false);
                });
                let compact = r.response.rect.width() > max_rect.width();
                ui.scope_builder(ui_builder, |ui| left_menu_ui(ui, app_ui, compact));
            });
        });
    });
}

fn left_menu_ui(ui: &mut egui::Ui, app_ui: &mut AppUi, compact: bool) {
    if compact {
        ui.menu_button(mdi!(ui, MENU, 18), |ui| draw_menu_buttons(ui, app_ui));
    } else {
        draw_menu_buttons(ui, app_ui);
    }

    ui.separator();

    ui.add(crate::gui::components::LeaderboardsUi(
        &app_ui.app.leaderboards,
    ));

    if app_ui.app.prefs.check_for_updates
        && let Some(new_release) = &*crate::update_check::NEWER_RELEASE.lock()
    {
        ui.separator();
        md(ui, L.update.with(&new_release.name, &new_release.html_url));
    }
}

fn draw_menu_buttons(ui: &mut egui::Ui, app_ui: &mut AppUi) {
    fn show_tab_toggle(ui: &mut egui::Ui, app_ui: &mut AppUi, tab: UtilityTab) {
        let mut open = app_ui.is_docked_utility_open(tab);
        if ui
            .checkbox(&mut open, (tab.icon(&ui, MDI_SMALL), tab.menu_name()))
            .clicked()
        {
            app_ui.toggle_docked_utility(tab);
        }
    }

    ui.menu_button(L.menu.file.title, |ui| {
        if ui.button(L.menu.file.open).clicked()
            && app_ui.confirm_discard_active_puzzle(L.confirm_discard.open_another_file)
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

        #[cfg(not(target_arch = "wasm32"))]
        {
            ui.separator();
            if ui.button(L.menu.file.exit).clicked()
                && app_ui.confirm_discard_all_puzzles(L.confirm_discard.exit)
            {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    });

    menu_button_that_stays_open(ui, L.menu.edit.title, |ui| {
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
            && app_ui.confirm_discard_active_puzzle(L.confirm_discard.reset_puzzle)
        {
            ui.close();
            app_ui.app.reset_puzzle();
        }
    });
    menu_button_that_stays_open(ui, L.menu.scramble.title, |ui| {
        let can_scramble = app_ui
            .app
            .active_puzzle
            .with_view(|view| view.puzzle().can_scramble)
            .unwrap_or(false);
        let full_scramble_button = egui::Button::new(L.menu.scramble.full);
        if ui.add_enabled(can_scramble, full_scramble_button).clicked() {
            ui.close();
            if app_ui.confirm_discard_active_puzzle(L.confirm_discard.scramble) {
                app_ui.app.scramble(ScrambleType::Full);
            }
        }
        ui.separator();
        let scramble_1_button = egui::Button::new(L.menu.scramble.one);
        if ui.add_enabled(can_scramble, scramble_1_button).clicked() {
            ui.close();
            if app_ui.confirm_discard_active_puzzle(L.confirm_discard.scramble) {
                app_ui.app.scramble(ScrambleType::Partial(1));
            }
        }
        let scramble_2_button = egui::Button::new(L.menu.scramble.two);
        if ui.add_enabled(can_scramble, scramble_2_button).clicked() {
            ui.close();
            if app_ui.confirm_discard_active_puzzle(L.confirm_discard.scramble) {
                app_ui.app.scramble(ScrambleType::Partial(2));
            }
        }
        ui.separator();
        show_tab_toggle(ui, app_ui, UtilityTab::Scrambler);
    });
    menu_button_that_stays_open(ui, L.menu.settings.title, |ui| {
        show_tab_toggle(ui, app_ui, UtilityTab::Colors);
        show_tab_toggle(ui, app_ui, UtilityTab::Styles);
        show_tab_toggle(ui, app_ui, UtilityTab::View);
        show_tab_toggle(ui, app_ui, UtilityTab::Animation);
        ui.separator();
        show_tab_toggle(ui, app_ui, UtilityTab::Interaction);
        show_tab_toggle(ui, app_ui, UtilityTab::Keybinds);
        show_tab_toggle(ui, app_ui, UtilityTab::Mousebinds);
        ui.separator();

        let mut changed = false;
        let mut prefs_ui = PrefsUi {
            ui,
            current: &mut app_ui.app.prefs,
            defaults: None,
            changed: &mut changed,
        };
        prefs_ui.checkbox(&L.prefs.online_mode, access!(.online_mode));
        prefs_ui.checkbox(&L.prefs.check_for_updates, access!(.check_for_updates));
        egui::global_theme_preference_buttons(prefs_ui.ui);

        app_ui.app.prefs.needs_save |= changed;
    });
    menu_button_that_stays_open(ui, L.menu.tools.title, |ui| {
        show_tab_toggle(ui, app_ui, UtilityTab::PieceFilters);
        show_tab_toggle(ui, app_ui, UtilityTab::Macros);
        show_tab_toggle(ui, app_ui, UtilityTab::MoveInput);
        ui.separator();
        show_tab_toggle(ui, app_ui, UtilityTab::Timer);
        show_tab_toggle(ui, app_ui, UtilityTab::KeybindsReference);
        ui.separator();
        show_tab_toggle(ui, app_ui, UtilityTab::Timeline);
        show_tab_toggle(ui, app_ui, UtilityTab::Scrambler);
        show_tab_toggle(ui, app_ui, UtilityTab::ImageGenerator);
    });
    menu_button_that_stays_open(ui, L.menu.puzzles.title, |ui| {
        show_tab_toggle(ui, app_ui, UtilityTab::Catalog);
        show_tab_toggle(ui, app_ui, UtilityTab::PuzzleInfo);
        show_tab_toggle(ui, app_ui, UtilityTab::HpsLogs);
        show_tab_toggle(ui, app_ui, UtilityTab::DevTools);

        ui.separator();

        let r = ui.checkbox(
            &mut app_ui.app.prefs.show_experimental_puzzles,
            L.menu.puzzles.show_experimental,
        );
        app_ui.app.prefs.needs_save |= r.changed();

        ui.separator();

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
                    Err(e) => crate::error_dialog(L.error_dialog.extracting_hps_files, e),
                }
            }
        }
    });
    menu_button_that_stays_open(ui, L.menu.help.title, |ui| {
        ui.heading(L.menu.help.guides);
        let _ = ui.button("Welcome");
        show_tab_toggle(ui, app_ui, UtilityTab::About);
        ui.separator();
        show_tab_toggle(ui, app_ui, UtilityTab::KeybindsReference);
    });
    if *crate::IS_PRERELEASE {
        menu_button_that_stays_open(ui, L.menu.debug.title, |ui| {
            show_tab_toggle(ui, app_ui, UtilityTab::Debug);
        });
    }
}
