//! User preferences.
//!
//! For a list of key names, see `VirtualKeyCode` in this file:
//! https://github.com/rust-windowing/winit/blob/master/src/event.rs

use directories::ProjectDirs;
use enum_map::EnumMap;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::{mpsc, Mutex};
use std::time::Duration;

mod colors;
mod gfx;
mod info;
mod keybinds;
mod view;

use crate::commands::{Command, PuzzleCommand};
use crate::puzzle::PuzzleType;
pub use colors::ColorPreferences;
pub use gfx::{GfxPreferences, Msaa};
pub use info::InfoPreferences;
pub use keybinds::{Key, KeyCombo, Keybind};
pub use view::ViewPreferences;

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

lazy_static! {
    static ref RX: Mutex<mpsc::Receiver<DebouncedEvent>> = {
        let (tx, rx) = mpsc::channel();
        match Watcher::new(tx, Duration::from_secs_f64(0.5)) {
            Ok(w) => *WATCHER.lock().unwrap() = Some(w),
            Err(e) => eprintln!("Error initializing preferences file watcher: {}", e),
        }
        unwatch_during(|| ()); // Start watching

        Mutex::new(rx)
    };
    static ref WATCHER: Mutex<Option<RecommendedWatcher>> = Mutex::new(None);
}
fn unwatch_during<T>(f: impl FnOnce() -> T) -> T {
    if let Some(path) = PREFS_FILE_PATH.as_ref().ok().and_then(|p| p.parent()) {
        if let Ok(mut w) = WATCHER.lock() {
            if let Some(w) = &mut *w {
                let _ = w.unwatch(path);
                let ret = f();
                if let Err(e) = w.watch(path, RecursiveMode::NonRecursive) {
                    eprintln!("Error initializing preferences file watcher: {}", e);
                }
                return ret;
            }
        }
    }
    f()
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

    pub gfx: GfxPreferences,
    pub info: InfoPreferences,
    pub view: PerPuzzle<ViewPreferences>,
    pub colors: ColorPreferences,

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
            let result = unwatch_during(|| -> anyhow::Result<()> {
                let path = PREFS_FILE_PATH.as_ref()?;
                if let Some(p) = path.parent() {
                    std::fs::create_dir_all(p)?;
                }
                serde_yaml::to_writer(std::fs::File::create(path)?, self)?;
                Ok(())
            });
            if let Err(e) = result {
                eprintln!("Error saving preferences: {}", e);
            }
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(transparent)]
pub struct PerPuzzle<T>(EnumMap<PuzzleType, T>);
impl<'de, T: Default + DeserializePerPuzzle<'de>> Default for PerPuzzle<T>
where
    T::Proxy: Default,
{
    fn default() -> Self {
        Self(
            PuzzleType::ALL
                .iter()
                .map(|&puzzle_type| {
                    let default = T::deserialize_from(T::Proxy::default(), puzzle_type);
                    (puzzle_type, default)
                })
                .collect(),
        )
    }
}
impl<'de, T: DeserializePerPuzzle<'de>> Deserialize<'de> for PerPuzzle<T>
where
    Self: Default,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<T>(PhantomData<T>);
        impl<'de, T: DeserializePerPuzzle<'de>> de::Visitor<'de> for Visitor<T>
        where
            PerPuzzle<T>: Default,
        {
            type Value = PerPuzzle<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map containing a value per puzzle type")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut ret = PerPuzzle::default();
                while let Some(puzzle_type) = map.next_key()? {
                    ret[puzzle_type] = T::deserialize_from(map.next_value()?, puzzle_type);
                }
                Ok(ret)
            }
        }

        deserializer.deserialize_map(Visitor(PhantomData))
    }
}
impl<T> std::ops::Index<PuzzleType> for PerPuzzle<T> {
    type Output = T;

    fn index(&self, puzzle_type: PuzzleType) -> &Self::Output {
        &self.0[puzzle_type]
    }
}
impl<T> std::ops::IndexMut<PuzzleType> for PerPuzzle<T> {
    fn index_mut(&mut self, puzzle_type: PuzzleType) -> &mut Self::Output {
        &mut self.0[puzzle_type]
    }
}

pub trait DeserializePerPuzzle<'de> {
    type Proxy: Deserialize<'de>;

    fn deserialize_from(value: Self::Proxy, ty: PuzzleType) -> Self;
}

fn is_false(x: &bool) -> bool {
    !x
}
