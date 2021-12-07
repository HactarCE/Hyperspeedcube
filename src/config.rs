use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

pub(crate) fn get_config<'a>() -> MutexGuard<'a, Config> {
    CONFIG.lock().unwrap()
}

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::load());
    static ref PROJECT_DIRS: Option<ProjectDirs> = ProjectDirs::from("", "", "Hyperspeedcube");
    static ref CONFIG_FILE_PATH: Result<PathBuf, NoConfigPath> = match &*PROJECT_DIRS {
        Some(proj_dirs) => {
            let mut p = proj_dirs.config_dir().to_owned();
            p.push("hyperspeedcube.json");
            Ok(p)
        }
        None => Err(NoConfigPath),
    };
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
struct NoConfigPath;
impl fmt::Display for NoConfigPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unable to get config file path.")
    }
}
impl Error for NoConfigPath {}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct Config {
    #[serde(skip)]
    pub needs_save: bool,

    // pub ctrl: CtrlConfig,
    pub gfx: GfxConfig,
    // pub hist: HistoryConfig,
    // pub keys: KeyConfig,
    // pub mouse: MouseConfig,
    // pub sim: SimConfig,
}
impl Config {
    pub fn load() -> Self {
        Self::_load().unwrap_or_else(|e| {
            eprintln!("Unable to load config: {}", e);
            eprintln!("Using default config");
            Config::default()
        })
    }
    fn _load() -> Result<Self, Box<dyn Error>> {
        let path = CONFIG_FILE_PATH.as_ref()?;
        let ret = serde_json::from_reader(File::open(path)?)?;
        Ok(ret)
    }

    pub fn save(&mut self) {
        if self.needs_save {
            if let Err(e) = self._save() {
                eprintln!("Unable to save config: {}", e);
            }
        }
    }
    fn _save(&mut self) -> Result<(), Box<dyn Error>> {
        self.needs_save = false;
        let path = CONFIG_FILE_PATH.as_ref()?;
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        serde_json::to_writer_pretty(File::create(path)?, self)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GfxConfig {
    pub dpi: f64,
    pub fps: u32,
    pub font_size: f32,

    pub msaa: Msaa,

    pub theta: f32,
    pub phi: f32,

    pub fov_3d: f32,
    pub fov_4d: f32,
    pub scale: f32,

    pub face_spacing: f32,
    pub sticker_spacing: f32,

    pub opacity: f32,

    pub label_size: f32,
}
impl Default for GfxConfig {
    fn default() -> Self {
        Self {
            dpi: 1.0,
            fps: 60,
            font_size: 16.0,

            msaa: Msaa::_8,

            theta: 35_f32.to_radians(),
            phi: -40_f32.to_radians(),

            fov_3d: 30_f32.to_radians(),
            fov_4d: 30_f32.to_radians(),
            scale: 2.0,

            face_spacing: 0.7,
            sticker_spacing: 0.5,

            opacity: 1.0,

            label_size: 24.0,
        }
    }
}
impl GfxConfig {
    /// Returns the duration of one frame based on the configured FPS value.
    pub fn frame_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(1.0 / self.fps as f64)
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Msaa {
    Off = 0,
    _2 = 2,
    _4 = 4,
    _8 = 8,
}
impl fmt::Display for Msaa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Msaa::Off => write!(f, "Off"),
            Msaa::_2 => write!(f, "2x"),
            Msaa::_4 => write!(f, "4x"),
            Msaa::_8 => write!(f, "8x"),
        }
    }
}
