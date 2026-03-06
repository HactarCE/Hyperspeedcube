use std::hash::Hash;
use std::sync::Arc;

use egui::{AtomExt, IntoAtoms};
use hypercubing_leaderboards_client::Leaderboards;
use parking_lot::Mutex;

use crate::L;
use crate::gui::markdown::md;
use crate::gui::util::{MDI_SMALL_SIZE, hyperlink_to, text_width};
use crate::leaderboards::{LEADERBOARDS_DOMAIN, LeaderboardsClientState};

pub struct LeaderboardsUi<'a>(pub &'a Arc<Mutex<LeaderboardsClientState>>);

impl egui::Widget for LeaderboardsUi<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut lb = self.0.lock();
        let mut wants_sign_out = false;
        let r = match &*lb {
            LeaderboardsClientState::NotSignedIn => {
                let menu_button_label = if hyperpaths::IS_OFFICIAL_BUILD {
                    L.leaderboards.sign_in
                } else {
                    L.leaderboards.sign_in_localhost
                };
                menu_button(ui, "not_signed_in", false, menu_button_label, |ui| {
                    ui.set_max_width(ui.spacing().menu_width / 2.0);
                    md(ui, L.leaderboards.sign_in_desc.with(LEADERBOARDS_DOMAIN));
                    if ui.link(L.leaderboards.actions.sign_in).clicked() {
                        let url = lb.init_auth(Arc::clone(self.0));
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                    }
                    if !hyperpaths::IS_OFFICIAL_BUILD {
                        md(ui, L.leaderboards.sign_in_desc_localhost);
                    }
                })
            }
            LeaderboardsClientState::WaitingForUserAuth { url } => {
                let menu_button_label = L.leaderboards.loading.waiting_for_auth;
                menu_button(ui, "waiting_for_auth", true, menu_button_label, |ui| {
                    hyperlink_to(ui, L.leaderboards.actions.sign_in_using_browser, url);
                    wants_sign_out |= ui.button(L.cancel).clicked();
                })
            }
            LeaderboardsClientState::FetchingProfileInfo { token } => {
                let menu_button_label = L.leaderboards.loading.fetching_profile_info;
                menu_button(ui, "fetching_info", true, menu_button_label, |ui| {
                    wants_sign_out |= ui.button(L.cancel).clicked();
                })
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
                menu_button(ui, "signed_in", false, menu_button_label, |ui| {
                    hyperlink_to(ui, L.leaderboards.links.profile, lb.profile_url());
                    hyperlink_to(ui, L.leaderboards.links.submissions, lb.submissions_url());
                    hyperlink_to(ui, L.leaderboards.links.settings, lb.settings_url());
                    ui.separator();
                    wants_sign_out |= ui.button(L.leaderboards.actions.sign_out).clicked();
                })
            }
            LeaderboardsClientState::Error { token, error } => {
                let error_msg = error.to_string();
                let menu_button_label = L.leaderboards.error_button;
                menu_button(ui, "error", false, menu_button_label, |ui| {
                    ui.strong(L.leaderboards.error_message);
                    ui.label(error_msg);
                    if ui.button(L.try_again).clicked() {
                        lb.sign_out();
                    }
                })
            }
        };
        if wants_sign_out {
            lb.sign_out();
        }
        r
    }
}

fn menu_button<'a>(
    ui: &mut egui::Ui,
    id_salt: impl Hash,
    spinner: bool,
    label: impl egui::IntoAtoms<'a>,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    ui.push_id(id_salt, |ui| {
        let atoms = match spinner {
            true => {
                let spinner_atom =
                    egui::Atom::custom(egui::Id::new("spinner_atom"), MDI_SMALL_SIZE);
                (spinner_atom, label).into_atoms()
            }
            false => label.into_atoms(),
        };
        let (r, _) = egui::containers::menu::MenuButton::new(atoms)
            .config(
                egui::containers::menu::MenuConfig::new()
                    .style(egui::style::StyleModifier::default()),
            )
            .ui(ui, add_contents);
        if spinner {
            let spinner_rect = egui::Align2::LEFT_CENTER.align_size_within_rect(
                MDI_SMALL_SIZE,
                r.rect.shrink2(ui.spacing().button_padding),
            );
            ui.place(spinner_rect, egui::Spinner::new());
        }
        r
    })
    .inner
}
