use std::{str::FromStr, sync::Arc};

use eyre::Result;
use parking_lot::Mutex;
use serde::Deserialize;

use crate::{GITHUB_REPO_NAME, GITHUB_REPO_OWNER};

#[derive(Deserialize, Debug, Clone)]
pub struct Release {
    pub html_url: String,
    pub name: String,
    // many more fields that we can safely ignore :)
}

impl Release {
    pub fn semver(&self) -> Option<semver::Version> {
        self.name
            .strip_prefix('v')
            .unwrap_or(&self.name)
            .parse()
            .ok()
    }
}

lazy_static! {
    pub static ref NEWER_RELEASE: Arc<Mutex<Option<Release>>> = {
        let arc_mutex = Arc::new(Mutex::new(None));
        std::thread::spawn({
            let arc_mutex = Arc::clone(&arc_mutex);
            move || match check_for_update() {
                Ok(newer_release) => *arc_mutex.lock() = newer_release,
                Err(e) => {
                    log::error!("Error checking for updates: {e}");
                }
            }
        });
        arc_mutex
    };
}

fn check_for_update() -> Result<Option<Release>> {
    // https://docs.github.com/en/rest/releases/releases?apiVersion=2022-11-28
    let releases: Vec<Release> = ureq::Agent::new_with_defaults()
        .get(format!(
            "https://api.github.com/repos/{GITHUB_REPO_OWNER}/{GITHUB_REPO_NAME}/releases",
        ))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()?
        .into_body()
        .read_json()?;

    let current_version = semver::Version::from_str(env!("CARGO_PKG_VERSION"))
        .expect("current version is invalid semver");

    let allow_prereleases = !current_version.pre.is_empty();

    Ok(releases
        .into_iter()
        .filter_map(|release| Some((release.semver()?, release)))
        .filter(|(version, _release)| allow_prereleases || version.pre.is_empty())
        .max_by(|(v1, _), (v2, _)| v1.cmp(v2))
        .filter(|(version, _release)| *version > current_version)
        .map(|(_version, release)| release))
}
