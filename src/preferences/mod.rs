//! User preferences.
//!
//! For a list of key names, see `VirtualKeyCode` in this file:
//! https://github.com/rust-windowing/winit/blob/master/src/event.rs

use directories::ProjectDirs;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::{hash_map, HashMap};
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::path::PathBuf;

mod colors;
mod gfx;
mod info;
mod interaction;
mod keybinds;
mod outlines;
mod view;

use crate::commands::{Command, PuzzleCommand};
use crate::puzzle::{traits::*, PuzzleTypeEnum};
pub use colors::*;
pub use gfx::*;
pub use info::*;
pub use interaction::*;
pub use keybinds::*;
pub use outlines::*;
pub use view::*;

const PREFS_FILE_NAME: &str = "hyperspeedcube";
const PREFS_FILE_EXTENSION: &str = "yaml";
const PREFS_FILE_FORMAT: config::FileFormat = config::FileFormat::Yaml;
const DEFAULT_PREFS_STR: &str = include_str!("default.yaml");

lazy_static! {
    pub static ref DEFAULT_PREFS: Preferences =
        serde_yaml::from_str(DEFAULT_PREFS_STR).unwrap_or_default();
}

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
        if let Ok(mut p) = LOCAL_DIR.clone() {
            p.push("nonportable");
            p.exists()
        } else {
            false
        }
    };
    static ref PROJECT_DIRS: Option<ProjectDirs> = ProjectDirs::from("", "", "Hyperspeedcube");
    static ref PREFS_FILE_PATH: Result<PathBuf, PrefsError> = {
        let mut p = if *NONPORTABLE {
            println!("Using non-portable preferences path");
            match &*PROJECT_DIRS {
                Some(proj_dirs) => proj_dirs.config_dir().to_owned(),
                None => return Err(PrefsError::NoPreferencesPath),
            }
        } else {
            println!("Using portable preferences path");
            LOCAL_DIR.clone()?
        };
        p.push(format!("{}.{}", PREFS_FILE_NAME, PREFS_FILE_EXTENSION));
        Ok(p)
    };

}

#[derive(Display, Debug, Copy, Clone, PartialEq, Eq)]
enum PrefsError {
    #[strum(serialize = "unable to get executable file path")]
    NoExecutablePath,
    #[strum(serialize = "unable to get preferences file path")]
    NoPreferencesPath,
}
impl Error for PrefsError {}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Preferences {
    #[serde(skip)]
    pub needs_save: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<PathBuf>,

    pub info: InfoPreferences,

    pub gfx: GfxPreferences,
    pub view: PerPuzzle<ViewPreferences>,
    pub outlines: OutlinePreferences,
    pub colors: ColorPreferences,
    pub interaction: InteractionPreferences,

    pub general_keybinds: Vec<Keybind<Command>>,
    pub puzzle_keybinds: PerPuzzle<Vec<Keybind<PuzzleCommand>>>,
}
impl Preferences {
    pub fn load(backup: Option<&Self>) -> Self {
        let mut config = config::Config::builder();

        // Load default preferences.
        let default_config_source = config::File::from_str(DEFAULT_PREFS_STR, PREFS_FILE_FORMAT);
        config = config.add_source(default_config_source.clone());

        // Load user preferences.
        match &*PREFS_FILE_PATH {
            Ok(path) => config = config.add_source(config::File::from(path.as_ref())),
            Err(e) => eprintln!("Error loading user preferences: {}", e),
        }

        // TODO: use try block (including the word "IIFE" here because I'll
        // search for that)
        config
            .build()
            .and_then(|c| c.try_deserialize::<Self>())
            .unwrap_or_else(|e| {
                eprintln!("Error loading preferences: {}", e);
                if let Ok(prefs_path) = &*PREFS_FILE_PATH {
                    let datetime = time::OffsetDateTime::now_local()
                        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
                    let mut backup_path = prefs_path.clone();
                    backup_path.pop();
                    backup_path.push(format!(
                        "{}_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}_bak.{}",
                        PREFS_FILE_NAME,
                        datetime.year(),
                        datetime.month() as u8,
                        datetime.day(),
                        datetime.hour(),
                        datetime.minute(),
                        datetime.second(),
                        PREFS_FILE_EXTENSION,
                    ));
                    if std::fs::rename(prefs_path, &backup_path).is_ok() {
                        eprintln!(
                            "Backup of old preferences stored at {}",
                            backup_path.to_str().unwrap_or(
                                "some path with invalid Unicode. Seriously, what have you done to your filesystem?"
                            ),
                        );
                    }
                }

                // Try backup
                backup
                    .cloned()
                    // Try just default config
                    .or_else(|| {
                        config::Config::builder()
                            .add_source(default_config_source)
                            .build()
                            .ok()?
                            .try_deserialize()
                            .ok()
                    })
                    .unwrap_or_default()
            })
    }

    pub fn save(&mut self) {
        if self.needs_save {
            self.needs_save = false;
            let result = (|| -> anyhow::Result<()> {
                // IIFE to mimic try block
                let path = PREFS_FILE_PATH.as_ref()?;
                if let Some(p) = path.parent() {
                    std::fs::create_dir_all(p)?;
                }
                serde_yaml::to_writer(std::fs::File::create(path)?, self)?;
                Ok(())
            })();
            if let Err(e) = result {
                eprintln!("Error saving preferences: {}", e);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(transparent)]
pub struct PerPuzzle<T> {
    map: HashMap<String, T>,
    #[serde(skip)]
    default: T,
}
impl<T: Default> std::ops::Index<PuzzleTypeEnum> for PerPuzzle<T> {
    type Output = T;

    fn index(&self, puzzle_type: PuzzleTypeEnum) -> &Self::Output {
        self.get(puzzle_type).unwrap_or(&self.default)
    }
}
impl<T: Default> std::ops::IndexMut<PuzzleTypeEnum> for PerPuzzle<T> {
    fn index_mut(&mut self, puzzle_type: PuzzleTypeEnum) -> &mut Self::Output {
        self.entry(puzzle_type).or_default()
    }
}
impl<T> PerPuzzle<T> {
    fn entry<'a>(&'a mut self, puzzle_type: PuzzleTypeEnum) -> hash_map::Entry<'a, String, T> {
        self.map
            .entry(puzzle_type.family_internal_name().to_owned())
    }
    fn get(&self, puzzle_type: PuzzleTypeEnum) -> Option<&T> {
        self.map.get(puzzle_type.family_internal_name())
    }
}

fn is_false(x: &bool) -> bool {
    !x
}
