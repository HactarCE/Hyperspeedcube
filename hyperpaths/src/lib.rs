#![allow(missing_docs)]

use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use directories::ProjectDirs;
use eyre::{OptionExt, Result};

#[macro_use]
extern crate lazy_static;

/// Whether this is an official build of the software (as opposed to a local
/// build).
pub const IS_OFFICIAL_BUILD: bool = std::option_env!("HSC_OFFICIAL_BUILD").is_some();

const PREFS_FILE_NAME: &str = "hsc2-prefs";
const PREFS_FILE_EXTENSION: &str = "yaml";

const STATS_FILE_NAME: &str = "hsc2-stats";
const STATS_FILE_EXTENSION: &str = "kdl";

const SOLVES_DIR_NAME: &str = "solves";

const LUA_DIR_NAME: &str = "lua";

lazy_static! {
    static ref PATHS: Option<AppPaths> = app_paths();
}

fn get() -> Result<&'static AppPaths> {
    PATHS.as_ref().ok_or_eyre("no paths")
}

/// Returns the user preferences file.
pub fn prefs_file() -> Result<&'static Path> {
    Ok(&get()?.prefs_file)
}
/// Returns the user statistics file.
pub fn stats_file() -> Result<&'static Path> {
    Ok(&get()?.stats_file)
}
/// Returns the directory containing autosaved solves.
pub fn solves_dir() -> Result<&'static Path> {
    Ok(&get()?.solves_dir)
}
/// Returns the filename for an autosaved solve.
pub fn solve_autosave_file(
    puzzle_id: &str,
    timestamp: &str,
    stm: u64,
) -> Result<(PathBuf, String)> {
    let puzzle_dirname = puzzle_id.replace(':', "~");
    let filename = format!("{timestamp}_stm{stm}.hsc").replace(":", "_");
    Ok((
        solves_dir()?.join(&puzzle_dirname).join(&filename),
        format!("{puzzle_dirname}/{filename}"),
    ))
}
/// Returns the directory containing Lua files such as user puzzle definitions.
pub fn lua_dir() -> Result<&'static Path> {
    Ok(&get()?.lua_dir)
}
pub fn crash_report_dir() -> Result<&'static Path> {
    Ok(&get()?.crash_report_dir)
}

/// Renames a file to create a backup. Emits a log message indicating success or
/// failure.
pub fn move_to_backup_file(original: &Path) {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let backup_path = backup_path(original, now);

    match std::fs::rename(original, &backup_path) {
        Ok(()) => {
            log::info!(
                "backup of {} stored at {}",
                original.display(),
                backup_path.display(),
            );
        }
        Err(e) => {
            if original.is_file() {
                log::error!("error backing up {}: {e}", original.display());
            }
        }
    }
}
fn backup_path(original: &Path, now: time::OffsetDateTime) -> PathBuf {
    let mut ret = original.to_owned();

    let stem = match ret.file_stem() {
        Some(stem) => stem.to_string_lossy().into_owned(),
        None => "unknown".to_string(),
    };
    let extension = match ret.extension() {
        Some(extension) => extension.to_string_lossy().into_owned(),
        None => "txt".to_string(),
    };

    ret.pop();

    ret.push(format!(
        "{stem}_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}_bak.{extension}",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
    ));

    ret
}

/// Paths to external files read by Hyperspeedcube.
struct AppPaths {
    /// Path to the Hyperspeedcube user preferences file.
    pub prefs_file: PathBuf,
    /// Path to the Hyperspeedcube user statistics file.
    pub stats_file: PathBuf,
    /// Path to the Hyperspeedcube autosaved solves directory.
    pub solves_dir: PathBuf,
    /// Path to the Hyperspeedcube Lua directory.
    pub lua_dir: PathBuf,
    /// Path to Hyperspeedcube crash reports.
    pub crash_report_dir: PathBuf,
}

/// Returns the app paths.
///
/// - For dev builds, uses the project directory.
/// - For official release builds in portable mode (the default on Windows &
///   Linux), uses the directory of the current executable.
/// - For official release builds in nonportable mode (the default on macOS),
///   uses the system directories.
///
/// If the preferred behavior (portable vs. nonportable) fails, then this
/// function falls back on the other.
fn app_paths() -> Option<AppPaths> {
    match is_nonportable() {
        true => nonportable_paths().or_else(portable_paths),
        false => portable_paths().or_else(nonportable_paths),
    }
}

fn nonportable_paths() -> Option<AppPaths> {
    match ProjectDirs::from("", "", "Hyperspeedcube") {
        Some(dirs) => {
            log::info!("Using nonportable paths");
            Some(AppPaths {
                prefs_file: dirs
                    .config_dir()
                    .join(format!("{PREFS_FILE_NAME}.{PREFS_FILE_EXTENSION}")),
                stats_file: dirs
                    .data_dir()
                    .join(format!("{STATS_FILE_NAME}.{STATS_FILE_EXTENSION}")),
                solves_dir: dirs.data_dir().join(SOLVES_DIR_NAME),
                lua_dir: dirs.data_dir().join(LUA_DIR_NAME),
                crash_report_dir: dirs.cache_dir().to_path_buf(),
            })
        }
        None => {
            log::error!("Error getting nonportable directories");
            None
        }
    }
}

fn portable_paths() -> Option<AppPaths> {
    match portable_dir() {
        Some(dir) => {
            log::info!("Using portable paths");
            Some(AppPaths {
                prefs_file: dir.join(format!("{PREFS_FILE_NAME}.{PREFS_FILE_EXTENSION}")),
                stats_file: dir.join(format!("{STATS_FILE_NAME}.{STATS_FILE_EXTENSION}")),
                solves_dir: dir.join(SOLVES_DIR_NAME),
                lua_dir: dir.join(LUA_DIR_NAME),
                crash_report_dir: dir,
            })
        }
        None => {
            log::error!("Error getting portable directory");
            None
        }
    }
}

fn portable_dir() -> Option<PathBuf> {
    if crate::IS_OFFICIAL_BUILD {
        // `/hyperspeedcube.exe`
        let exe_path = env::current_exe().ok()?.canonicalize().ok()?;
        Some(exe_path.parent()?.to_path_buf())
    } else {
        // `/hyperpaths/`
        Some(
            PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
                .ok()?
                .parent()?
                .to_path_buf(),
        )
    }
}

fn is_nonportable() -> bool {
    if crate::IS_OFFICIAL_BUILD && cfg!(target_os = "macos") {
        // If we are in a macOS app package, then we are always nonportable
        // because macOS doesn't allow storing files in the same directory as
        // the executable.
        true
    } else if let Some(mut p) = portable_dir() {
        // Otherwise, check whether the `nonportable` file exists in the same
        // directory as the executable.
        p.push("nonportable");
        p.exists()
    } else {
        false
    }
}
