//! Hypercubing leaderboards authentication.

use std::{fmt, time::Duration};

use base64::prelude::*;
use chrono::NaiveDate;
use rand::{SeedableRng, seq::IndexedRandom};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use ureq::{
    Agent, RequestBuilder,
    config::Config,
    http::StatusCode,
    typestate::{WithBody, WithoutBody},
};

/// Domain for the official Hypercubing leaderboards.
///
/// For security, this must begin with `https://`.
pub const LEADERBOARDS_DOMAIN: &str = "https://lb.hypercubing.xyz";

/// Length of secret code used for authentication. Longer is more secure.
const SECRET_CODE_LEN: usize = 64;

/// Long-poll timeout in seconds.
const LONG_POLL_TIMEOUT: Duration = Duration::from_mins(5);
/// Request timeout in seconds.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

fn get(domain: &str, path: &str, token: Option<&str>) -> RequestBuilder<WithoutBody> {
    let mut req = default_agent(REQUEST_TIMEOUT, domain).get(format!("{domain}{path}"));
    if let Some(token) = token {
        req = req.header("Cookie", format!("token={token}"));
    }
    req
}
fn post(domain: &str, path: &str, token: Option<&str>) -> RequestBuilder<WithBody> {
    let mut req = default_agent(REQUEST_TIMEOUT, domain).post(format!("{domain}{path}"));
    if let Some(token) = token {
        req = req.header("Cookie", format!("token={token}"));
    }
    req
}
fn default_agent(timeout: Duration, domain: &str) -> Agent {
    Config::builder()
        .timeout_global(Some(timeout))
        .https_only(domain.starts_with("https"))
        .build()
        .into()
}

/// Error type used for leaderboard requests.
#[derive(Debug)]
pub enum Error {
    /// HTTPS request error.
    Ureq(ureq::Error),
    /// Unknown response.
    UnknownResponse(ureq::http::Response<ureq::Body>),
    /// Authentication timed out.
    AuthTimeout,
    /// Token expired.
    BadToken,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Ureq(error) => write!(f, "{error}"),
            Error::UnknownResponse(response) => {
                write!(f, "unknown response: {}", response.status())
            }
            Error::AuthTimeout => write!(f, "auth timeout"),
            Error::BadToken => write!(f, "bad token"),
        }
    }
}
impl From<ureq::Error> for Error {
    fn from(value: ureq::Error) -> Self {
        Self::Ureq(value)
    }
}

/// Info about the signed-in leaderboards user.
#[derive(Serialize, Deserialize, Debug)]
pub struct UserInfo {
    /// Leaderboards user ID.
    pub id: i32,
    /// Leaderboards username.
    pub name: Option<String>,
    /// Email address for authentication and contacting, if one is set.
    pub email: Option<String>,
    /// Email ID for authentication and contacting, if one is set.
    pub discord_id: Option<u64>,
    /// Discord username, if known.
    pub discord_username: Option<String>,
    /// Discord nickname, if known.
    pub discord_nickname: Option<String>,
    /// Discord avatar URL, if known.
    pub discord_avatar_url: Option<String>,
    /// Whether the user is a leaderboard moderator.
    pub moderator: bool,
}
impl UserInfo {
    /// Leaderboards username, or a fallback if there isn't one set.
    pub fn display_name(&self) -> String {
        let Self { id, name, .. } = self;
        name.clone().unwrap_or_else(|| format!("User #{id}"))
    }
}

/// Authentication flow.
pub struct AuthFlow {
    /// URL for the user to open in a browser to authenticate.
    browser_url: String,

    poll_url: String,
    poll_body: serde_json::Value,
}
impl AuthFlow {
    /// Initiates a new PKCE authentication flow, which allows the user to sign
    /// into the leaderboards using their browser.
    ///
    /// This method does not block, and in fact does not send any network
    /// requests.
    pub fn new(domain: &str) -> Self {
        let secret_code = random_b64_string(SECRET_CODE_LEN);
        let hash = sha2::Sha256::digest(&secret_code);
        let base64_encoded_hash = BASE64_URL_SAFE.encode(hash);

        let browser_url = format!("{domain}/submit-pkce?hash={base64_encoded_hash}");
        let poll_url = format!("{domain}/poll-pkce");

        Self {
            browser_url,

            poll_url,
            poll_body: serde_json::json!({
                "secret_code": secret_code,
            }),
        }
    }

    /// Returns a URL for the user to open in their browser.
    pub fn browser_url(&self) -> &str {
        &self.browser_url
    }

    /// Polls to see whether the user has submitted an auth token. Returns the
    /// token if successful, or `None` if still waiting.
    ///
    /// **This method blocks and should be run on a background thread.**
    pub fn poll(&self) -> Result<Option<String>, Error> {
        let response = default_agent(LONG_POLL_TIMEOUT, &self.poll_url)
            .post(&self.poll_url)
            .send_json(&self.poll_body)?;

        match response.status() {
            StatusCode::UNAUTHORIZED => Err(Error::AuthTimeout), // probably timeout
            StatusCode::NO_CONTENT => Ok(None),                  // keep polling
            StatusCode::OK => Ok(Some(response.into_body().read_to_string()?)), // success! this is a token
            _ => Err(Error::UnknownResponse(response)),                         // other response
        }
    }

    /// Repeatedly polls to see whether the user has submitted an auth token.
    /// Returns the token if successful.
    ///
    /// **This method blocks and should be run on a background thread.**
    pub fn poll_until_done(&self) -> Result<String, Error> {
        loop {
            match self.poll()? {
                Some(token) => break Ok(token),
                None => continue,
            }
        }
    }
}

/// Handle to the leaderboards.
#[derive(Serialize, Deserialize, Debug)]
pub struct Leaderboards {
    domain: String,
    token: String,
    token_expiry: Option<NaiveDate>,
    user_info: UserInfo,
}
impl Leaderboards {
    /// Signs into the leaderboards using an existing token.
    ///
    /// **This method blocks and should be run on a background thread.**
    pub fn new(domain: &str, token: String) -> Result<Self, Error> {
        let domain = domain.to_owned();

        // IIFE to mimic try_block
        let token_expiry =
            (|| NaiveDate::from_epoch_days(token.split_once('_')?.0.parse().ok()?))();

        let response = get(&domain, "/self-info", Some(&token)).call()?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(Error::BadToken);
        }

        let user_info = response.into_body().read_json()?;

        Ok(Self {
            domain,
            token,
            token_expiry,
            user_info,
        })
    }

    /// Returns the authentication token. **This should be kept secret.**
    ///
    /// This can be used to re-authenticate with the leaderboards after
    /// restarting the program.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Returns the date the token expires, or `None` if it cannot be parsed
    /// from the token, which should not happen.
    ///
    /// This is approximate; the token may expire any time during the UTC day.
    pub fn token_expiry(&self) -> Option<NaiveDate> {
        self.token_expiry
    }

    /// Returns cached user info.
    pub fn user_info(&self) -> &UserInfo {
        &self.user_info
    }

    /// Invalidates the current token.
    ///
    /// **This method blocks and should be run on a background thread.**
    pub fn sign_out(&self) -> Result<(), Error> {
        self.req_get("/sign-out").call()?;
        Ok(())
    }

    fn req_get(&self, path: &str) -> ureq::RequestBuilder<WithoutBody> {
        get(&self.domain, path, Some(&self.token))
    }
    fn req_post(&self, path: &str) -> ureq::RequestBuilder<WithBody> {
        post(&self.domain, path, Some(&self.token))
    }

    /// Returns the URL of the user's leaderboard profile.
    pub fn profile_url(&self) -> String {
        format!("{}/solver?id={}", self.domain, self.user_info.id)
    }
    /// Returns the URL of the user's submissions.
    pub fn submissions_url(&self) -> String {
        format!("{}/my-submissions", self.domain)
    }
    /// Returns the URL of the user settings page.
    pub fn settings_url(&self) -> String {
        format!("{}/settings", self.domain)
    }
}

const BASE64_URL_SAFE_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Returns a crypto-safe random base-64 string of the specified length.
#[allow(clippy::unwrap_used)]
pub fn random_b64_string(len: usize) -> String {
    let mut rng = rand::rngs::StdRng::from_os_rng();
    (0..len)
        .map(|_| *BASE64_URL_SAFE_ALPHABET.choose(&mut rng).unwrap() as char)
        .collect()
}
