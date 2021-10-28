use std::fmt;
use std::sync::{Mutex, MutexGuard};

pub fn get_config<'a>() -> MutexGuard<'a, Config> {
    CONFIG.lock().unwrap()
}

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::default());
}

#[derive(Debug, Default)]
pub struct Config {
    // pub ctrl: CtrlConfig,
    pub gfx: GfxConfig,
    // pub hist: HistoryConfig,
    // pub keys: KeyConfig,
    // pub mouse: MouseConfig,
    // pub sim: SimConfig,
}

#[derive(Debug)]
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
}
impl Default for GfxConfig {
    fn default() -> Self {
        Self {
            dpi: 1.0,
            fps: 60,
            font_size: 16.0,

            msaa: Msaa::_8,

            theta: 35_f32.to_radians(),
            phi: 45_f32.to_radians(),

            fov_3d: 30_f32.to_radians(),
            fov_4d: 30_f32.to_radians(),
            scale: 1.0,

            face_spacing: 0.7,
            sticker_spacing: 0.6,

            opacity: 1.0,
        }
    }
}
impl GfxConfig {
    /// Returns the duration of one frame based on the configured FPS value.
    pub fn frame_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(1.0 / self.fps as f64)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
