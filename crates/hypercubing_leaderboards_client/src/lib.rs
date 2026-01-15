//! Hypercubing leaderboards authentication.

use std::borrow::Cow;
use std::time::Duration;

use base64::prelude::*;
use chrono::{DateTime, NaiveDate, Utc};
use rand::SeedableRng;
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use ureq::config::Config;
use ureq::http::StatusCode;
use ureq::typestate::{WithBody, WithoutBody};
use ureq::{Agent, RequestBuilder};

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
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("{0}")]
    Ureq(#[from] ureq::Error),
    #[error("unknown response: {}", .0.status())]
    UnknownResponse(ureq::http::Response<ureq::Body>),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("auth timeout")]
    AuthTimeout,
    #[error("bad token")]
    BadToken,
    #[error("internal error: {0}")]
    Internal(&'static str),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Ureq(ureq::Error::Json(e))
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

            // success! this is a token
            StatusCode::OK => Ok(Some(response.into_body().read_to_string()?)),

            _ => Err(Error::UnknownResponse(response)), // other response
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

    /// Submits a solve to be auto-verified and returns a URL to the submission.
    ///
    /// **This method blocks and should be run on a background thread.**
    pub fn submit_solve_to_auto_verify(
        &self,
        submission: AutoVerifySubmission,
    ) -> Result<String, Error> {
        let AutoVerifySubmission {
            program_abbr,
            solver_notes,
            computer_assisted,
            will_upload_video,
            log_file_name,
            log_file_contents,
        } = submission;
        let temp_dir = tempfile::tempdir()?;
        let file_path = temp_dir.path().join(log_file_name);
        std::fs::write(&file_path, log_file_contents)?;
        let resp = self.req_post("/submit-solve-to-autoverify").send(
            ureq::unversioned::multipart::Form::new()
                .text("program_abbr", &program_abbr)
                .text("solver_notes", solver_notes.trim())
                .text("computer_assisted", &computer_assisted.to_string())
                .text("will_upload_video", &will_upload_video.to_string())
                .file("log_file", file_path)?,
        )?;

        let mut body = resp.into_body();
        let json = body.read_json::<serde_json::Value>()?;
        let solve_submission_url = json
            .get("url")
            .unwrap_or_default()
            .as_str()
            .unwrap_or_default();

        drop(temp_dir);
        Ok(solve_submission_url.to_string())
    }

    /// Returns PBs for a specific puzzle for the current user.
    pub fn get_pbs(&self, request: &PersonalBestRequest) -> Result<PersonalBests, Error> {
        Ok(self
            .req_get("/api/solver-pbs")
            .query_pairs(json_map_to_query_pairs(request)?)
            .call()?
            .into_body()
            .read_json()?)
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

/// Solve submission to the leaderboards to be automatically verified.
pub struct AutoVerifySubmission {
    /// Abbreviation for the program used.
    pub program_abbr: String,
    /// Solver notes (optional).
    pub solver_notes: String,
    /// Whether the solve is a computer-assisted FMC solve.
    pub computer_assisted: bool,
    /// Whether the solver intends to upload a video and add it to the
    /// submission.
    pub will_upload_video: bool,
    /// Log file name.
    pub log_file_name: String,
    /// Log file contents.
    pub log_file_contents: String,
}

/// Parameters for requesting puzzle PBs.
///
/// `puzzle_id` and `hsc_puzzle_id` are mutually exclusive; one of them is
/// required. All other fields are optional. `target_user` is assumed to be the
/// current user.
#[derive(Serialize, Debug, Default, Clone)]
pub struct PersonalBestRequest {
    /// Leaderboards ID for the puzzle. If `None`, use `hsc_puzzle_id` instead.
    pub puzzle_id: Option<i32>,
    /// HSC2 ID for the puzzle. If `None`, use `puzzle_id` instead.
    pub hsc_puzzle_id: Option<String>,

    /// User whose PBs to fetch. If `None`, use the currently logged-in user.
    pub target_user: Option<i32>,

    /// Flags: average
    pub average: bool,
    /// Flags: blindfolded
    pub blind: bool,
    /// Flags: filters (if `None`, use the default for the puzzle)
    pub filters: Option<bool>,
    /// Flags: macros (if `None`, use the default for the puzzle)
    pub macros: Option<bool>,
    /// Flags: one-handed
    pub one_handed: bool,

    /// Whether to only consider verified (approved) solves. If `None`, then
    /// unverified solves are included as well.
    pub require_verified: bool,
}

/// Personal best solves in a category.
#[derive(Deserialize, Debug, Default, Clone)]
pub struct PersonalBests {
    /// Speed PB
    pub speed: Option<Solve>,
    /// FMC PB
    pub fmc: Option<Solve>,
    /// Computer-assisted FMC PB
    pub fmcca: Option<Solve>,
}

/// Solve on the leaderboards.
#[derive(Deserialize, Debug, Clone)]
pub struct Solve {
    /// Numeric ID of the solve.
    pub id: i32,
    /// Absolute URL to the solve page.
    pub url: String,
    /// Date that the solve was performed.
    pub solve_date: DateTime<Utc>,
    /// Move count, if known if authorized to be shown.
    pub move_count: Option<i32>,
    /// Speedsolve duration in centiseconds, if known and if authorized to be
    /// shown.
    pub speed_cs: Option<i32>,
    /// FMC verification status. `None` if not unverified, `Some(false)` if
    /// rejected, `Some(true)` if approved.
    pub fmc_verified: Option<bool>,
    /// Speed verification status. `None` if not unverified, `Some(false)` if
    /// rejected, `Some(true)` if approved.
    pub speed_verified: Option<bool>,
}

fn json_map_to_query_pairs<T: serde::Serialize>(
    value: &T,
) -> Result<Vec<(String, Cow<'static, str>)>, Error> {
    let mut query_pairs: Vec<(String, Cow<'static, str>)> = vec![];
    let json_value = serde_json::to_value(value)?;
    for (k, v) in json_value
        .as_object()
        .ok_or(Error::Internal("expected JSON object"))?
    {
        let value_string = match v {
            serde_json::Value::Null => continue, // skip
            serde_json::Value::Bool(false) => "false".into(),
            serde_json::Value::Bool(true) => "true".into(),
            serde_json::Value::Number(number) => number.to_string().into(),
            serde_json::Value::String(s) => s.clone().into(),
            serde_json::Value::Array(_) => {
                return Err(Error::Internal("expected JSON primitive; got array"));
            }
            serde_json::Value::Object(_) => {
                return Err(Error::Internal("expected JSON primitive; got object"));
            }
        };
        query_pairs.push((k.clone(), value_string));
    }
    Ok(query_pairs)
}
