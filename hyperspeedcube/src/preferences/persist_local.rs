// TODO: use [`crate::paths`] here

use std::path::PathBuf;

use directories::ProjectDirs;
use eyre::Result;
use serde::Serialize;

const PREFS_FILE_NAME: &str = "hyperspeedcube";
const PREFS_FILE_EXTENSION: &str = "yaml";

// File paths
lazy_static! {
    static ref LOCAL_DIR: Result<PathBuf, PrefsError> = (|| Some(
        // IIFE to mimic `try_block`
        std::env::current_exe()
            .ok()?
            .canonicalize()
            .ok()?
            .parent()?
            .to_owned()
    ))()
    .ok_or(PrefsError::NoExecutablePath);
    static ref NONPORTABLE: bool = {
        if crate::IS_OFFICIAL_BUILD && cfg!(target_os = "macos") {
            // If we are in a macOS app package, then we are always nonportable
            // because macOS doesn't allow storing files in the same directory
            // as the executable.
            true
        } else if let Ok(mut p) = LOCAL_DIR.clone() {
            // If not, check if the `nonportable` file exists in the same
            // directory as the executable.
            p.push("nonportable");
            p.exists()
        } else {
            false
        }
    };
    static ref PROJECT_DIRS: Option<ProjectDirs> = ProjectDirs::from("", "", "Hyperspeedcube");
    static ref PREFS_FILE_PATH: Result<PathBuf, PrefsError> = {
        let mut p = if *NONPORTABLE {
            log::info!("Using non-portable preferences path");
            match &*PROJECT_DIRS {
                Some(proj_dirs) => proj_dirs.config_dir().to_owned(),
                None => return Err(PrefsError::NoPreferencesPath),
            }
        } else {
            log::info!("Using portable preferences path");
            LOCAL_DIR.clone()?
        };
        p.push(format!("{}.{}", PREFS_FILE_NAME, PREFS_FILE_EXTENSION));
        Ok(p)
    };
}

#[derive(thiserror::Error, Debug, Copy, Clone, PartialEq, Eq)]
pub enum PrefsError {
    #[error("unable to get executable file path")]
    NoExecutablePath,
    #[error("unable to get preferences file path")]
    NoPreferencesPath,
}

pub fn user_config_source() -> Result<impl config::Source, PrefsError> {
    PREFS_FILE_PATH
        .clone()
        .map(|path| config::File::from(path.as_ref()))
}

pub fn save(prefs_data: &impl Serialize) -> Result<()> {
    let path = PREFS_FILE_PATH.as_ref()?;
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    serde_yaml::to_writer(std::fs::File::create(path)?, prefs_data)?;
    Ok(())
}

pub fn backup_prefs_file() {
    if let Ok(prefs_path) = &*PREFS_FILE_PATH {
        let mut backup_path = prefs_path.clone();
        backup_path.pop();

        let now =
            time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
        backup_path.push(format!(
            "{}_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}_bak.{}",
            PREFS_FILE_NAME,
            now.year(),
            now.month() as u8,
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
            PREFS_FILE_EXTENSION,
        ));

        if std::fs::rename(prefs_path, &backup_path).is_ok() {
            log::info!(
                "Backup of old preferences stored at {}",
                backup_path.display(),
            );
        }
    }
}
