use std::sync::Arc;

use hypercubing_leaderboards_client::{AuthFlow, Leaderboards};
use parking_lot::Mutex;

pub const LEADERBOARDS_DOMAIN: &str = if hyperpaths::IS_OFFICIAL_BUILD {
    hypercubing_leaderboards_client::LEADERBOARDS_DOMAIN
} else {
    "http://localhost:3000"
};

#[derive(Debug, Default)]
pub enum LeaderboardsClientState {
    #[default]
    NotSignedIn,
    WaitingForUserAuth {
        url: String,
    },
    FetchingProfileInfo {
        token: String,
    },
    SignedIn(Arc<Leaderboards>),

    Error {
        token: Option<String>,
        error: hypercubing_leaderboards_client::Error,
    },
}

impl LeaderboardsClientState {
    pub fn load() -> Arc<Mutex<Self>> {
        let mut this = Arc::new(Mutex::new(Self::NotSignedIn));
        if let Some(token) = load_token_from_file() {
            this.lock().init_from_token(Arc::clone(&this), token);
        }
        this
    }

    pub fn save(&self) {
        save_token_to_file(match self {
            Self::FetchingProfileInfo { token } => Some(token),
            Self::SignedIn(leaderboards) => Some(leaderboards.token()),
            Self::Error { token, .. } => token.as_deref(),
            _ => None,
        });
    }

    /// Initiates authentication and returns the URL for the user to open.
    ///
    /// If the user is already signed in, they are first signed out.
    pub fn init_auth(&mut self, this: Arc<Mutex<Self>>) -> String {
        self.sign_out();
        let auth_flow = AuthFlow::new(LEADERBOARDS_DOMAIN);
        let url = auth_flow.browser_url().to_string();
        *self = Self::WaitingForUserAuth { url: url.clone() };
        std::thread::spawn(move || match auth_flow.poll_until_done() {
            Ok(token) => this.lock().init_from_token(Arc::clone(&this), token),
            Err(e) => {
                *this.lock() = Self::Error {
                    token: None,
                    error: e,
                }
            }
        });
        url
    }

    pub fn init_from_token(&mut self, this: Arc<Mutex<Self>>, token: String) {
        *self = Self::FetchingProfileInfo {
            token: token.clone(),
        };
        std::thread::spawn(move || {
            match Leaderboards::new(LEADERBOARDS_DOMAIN, token.clone()) {
                Ok(ok) => {
                    let mut this_guard = this.lock();
                    *this_guard = Self::SignedIn(Arc::new(ok));
                    this_guard.save();
                }
                Err(e) => {
                    *this.lock() = Self::Error {
                        token: (!matches!(e, hypercubing_leaderboards_client::Error::BadToken))
                            .then_some(token),
                        error: e,
                    }
                }
            };
        });
    }

    pub fn sign_out(&mut self) {
        if let LeaderboardsClientState::SignedIn(leaderboards) = self {
            let leaderboards = Arc::clone(leaderboards);
            std::thread::spawn(move || leaderboards.sign_out());
        }
        *self = Self::NotSignedIn;
        self.save();
    }
}

fn save_token_to_file(token: Option<&str>) {
    match token {
        Some(token) => {
            let msg = "\
                # This token grants access to your Hypercubing leaderboards account.\n\
                # DO NOT share this file with anyone.\
            ";
            if let Err(e) = std::fs::write(
                hyperpaths::LEADERBOARDS_TOKEN_FILE_NAME,
                format!("{msg}\n{token}\n"),
            ) {
                log::error!("Error saving leaderboards token: {e}");
            }
        }
        None => {
            if std::path::PathBuf::from(hyperpaths::LEADERBOARDS_TOKEN_FILE_NAME).is_file()
                && let Err(e) = std::fs::remove_file(hyperpaths::LEADERBOARDS_TOKEN_FILE_NAME)
            {
                log::error!("Error deleting leaderboards token: {e}");
            }
        }
    }
}

fn load_token_from_file() -> Option<String> {
    let file_contents = std::fs::read_to_string(hyperpaths::LEADERBOARDS_TOKEN_FILE_NAME).ok()?;
    let token = file_contents
        .lines()
        .find(|line| !line.is_empty() && !line.starts_with('#'))?
        .trim();
    if !token.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None; // reject suspicious characters
    }
    Some(token.to_owned())
}
