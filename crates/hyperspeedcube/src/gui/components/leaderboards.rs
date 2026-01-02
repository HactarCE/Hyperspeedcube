use std::sync::Arc;

use egui::AtomExt;
use hypercubing_leaderboards_client::Leaderboards;
use parking_lot::Mutex;

use crate::gui::markdown::md;
use crate::gui::util::text_width;
use crate::leaderboards::{LEADERBOARDS_DOMAIN, LeaderboardsClientState};

pub fn show_leaderboards_ui(
    ui: &mut egui::Ui,
    leaderboards_state: &Arc<Mutex<LeaderboardsClientState>>,
) -> egui::Response {
    ui.horizontal(|ui| {
        let mut lb = leaderboards_state.lock();
        let mut wants_sign_out = false;
        let leaderboards_msg = match &*lb {
            LeaderboardsClientState::NotSignedIn => {
                ui.menu_button("Leaderboards sign-in", |ui| {
                    ui.set_max_width(ui.spacing().menu_width / 2.0);
                    md(
                        ui,
                        format!(
                            "Hyperspeedcube is integrated with the \
                             [Hypercubing leaderboards]({LEADERBOARDS_DOMAIN}).\n\
                             \n\
                             If you sign in, you can automatically upload your \
                             fastest speedsolves and shortest solutions to the \
                             leaderboards.",
                        ),
                    );
                    if ui.link("Sign into the leaderboards").clicked() {
                        let url = lb.init_auth(Arc::clone(&leaderboards_state));
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                    }
                });
            }
            LeaderboardsClientState::WaitingForUserAuth { url } => {
                ui.spinner();
                ui.push_id("waiting_for_auth", |ui| {
                    ui.menu_button("Waiting for authentication ...", |ui| {
                        ui.hyperlink_to("Sign in using browser", url);
                        wants_sign_out |= ui.button("Cancel").clicked();
                    });
                });
            }
            LeaderboardsClientState::FetchingProfileInfo { token } => {
                ui.spinner();
                ui.push_id("fetching_info", |ui| {
                    ui.menu_button("Fetching profile info ...", |ui| {
                        wants_sign_out |= ui.button("Cancel").clicked();
                    });
                });
            }
            LeaderboardsClientState::SignedIn(lb) => {
                let user_info = lb.user_info();
                let mut menu_button_label = egui::Atoms::new(
                    user_info
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("User #{}", user_info.id)),
                );
                if let Some(avatar_url) = &user_info.discord_avatar_url {
                    menu_button_label.push_left(
                        egui::Image::new(egui::ImageSource::Uri(avatar_url.into()))
                            .corner_radius(f32::INFINITY) // circle
                            .atom_size(egui::Vec2::splat(ui.spacing().interact_size.y)),
                    );
                }
                ui.push_id("signed_in", |ui| {
                    ui.menu_button(menu_button_label, |ui: &mut egui::Ui| {
                        ui.hyperlink_to("My profile", lb.profile_url());
                        ui.hyperlink_to("My submissions", lb.submissions_url());
                        ui.hyperlink_to("Settings", lb.settings_url());
                        ui.separator();
                        wants_sign_out |= ui.button("Sign out").clicked();
                    });
                });
            }
            LeaderboardsClientState::Error { token, error } => {
                let error_msg = error.to_string();
                ui.push_id("error", |ui| {
                    ui.menu_button("Error (click for details)", |ui| {
                        ui.label(error_msg);
                        if ui.button("Try again").clicked() {
                            lb.sign_out();
                        }
                    });
                });
            }
        };
        if wants_sign_out {
            lb.sign_out();
        }
    })
    .response
}
