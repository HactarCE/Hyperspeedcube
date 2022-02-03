//! Configuration file
//!
//! For a list of key names, see `VirtualKeyCode` in this file:
//! https://github.com/rust-windowing/winit/blob/master/src/event.rs

use directories::ProjectDirs;
use key_names::KeyMappingCode;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::time::Duration;
use winit::event::{ModifiersState, VirtualKeyCode};

use crate::colors;
use crate::puzzle::commands::CommandSerde;
use crate::puzzle::{traits::*, Command, PuzzleType};

pub(crate) fn get_config<'a>() -> MutexGuard<'a, Config> {
    match CONFIG.try_lock() {
        Ok(config) => config,
        Err(TryLockError::Poisoned(e)) => panic!("config mutex poisoned: {}", e),
        Err(TryLockError::WouldBlock) => panic!("config mutex double-locked"),
    }
}

const CONFIG_FILE_NAME: &str = "hyperspeedcube";
const CONFIG_FILE_EXTENSION: &str = "yaml";

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::load());
    static ref PROJECT_DIRS: Option<ProjectDirs> = ProjectDirs::from("", "", "Hyperspeedcube");
    static ref CONFIG_FILE_PATH: Result<PathBuf, NoConfigPath> = match &*PROJECT_DIRS {
        Some(proj_dirs) => {
            let mut p = proj_dirs.config_dir().to_owned();
            p.push(format!("{}.{}", CONFIG_FILE_NAME, CONFIG_FILE_EXTENSION));
            Ok(p)
        }
        None => Err(NoConfigPath),
    };
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
struct NoConfigPath;
impl fmt::Display for NoConfigPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unable to get config file path")
    }
}
impl Error for NoConfigPath {}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    #[serde(skip)]
    pub needs_save: bool,
    #[serde(skip)]
    pub window_states: WindowStates,

    pub log_file: PathBuf,

    pub gfx: GfxConfig,
    pub view: PerPuzzle<ViewConfig>,
    pub colors: ColorsConfig,
    pub keybinds: PerPuzzle<Vec<Keybind>>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            needs_save: true,
            window_states: Default::default(),

            log_file: PathBuf::from("puzzle.log"),

            gfx: Default::default(),
            view: Default::default(),
            colors: Default::default(),
            keybinds: Default::default(),
        }
    }
}
impl Config {
    pub fn load() -> Self {
        Self::_load().unwrap_or_else(|e| {
            eprintln!("Unable to load config: {}", e);
            if let Ok(config_path) = &*CONFIG_FILE_PATH {
                let datetime = time::OffsetDateTime::now_local()
                    .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
                let mut backup_path = config_path.clone();
                backup_path.pop();
                backup_path.push(format!(
                    "{}_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}_bak.{}",
                    CONFIG_FILE_NAME,
                    datetime.year(),
                    datetime.month() as u8 ,
                    datetime.day(),
                    datetime.hour(),
                    datetime.minute(),
                    datetime.second(),
                    CONFIG_FILE_EXTENSION,
                ));
                if std::fs::rename(config_path, &backup_path).is_ok() {
                    eprintln!(
                        "Backup of old config stored at {}",
                        backup_path.to_str().unwrap_or(
                            "some path with invalid Unicode. Seriously what have you done to your filesystem?"
                        ),
                    );
                }
            }
            eprintln!("Using default config");
            Config::default()
        })
    }
    fn _load() -> Result<Self, Box<dyn Error>> {
        // TODO: use try block
        let path = CONFIG_FILE_PATH.as_ref()?;
        let ret = serde_yaml::from_reader(File::open(path)?)?;
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
        // TODO: use try block
        self.needs_save = false;
        let path = CONFIG_FILE_PATH.as_ref()?;
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        serde_yaml::to_writer(File::create(path)?, self)?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct WindowStates {
    pub graphics: bool,
    pub view: bool,
    pub colors: bool,
    pub keybinds: bool,

    pub about: bool,

    #[cfg(debug_assertions)]
    pub demo: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GfxConfig {
    pub fps: u32,
    pub font_size: f32,
    #[serde(skip)]
    pub lock_font_size: bool,

    pub msaa: Msaa,

    pub label_size: f32, // TODO: remove or move this
}
impl Default for GfxConfig {
    fn default() -> Self {
        Self {
            fps: 60,
            font_size: 17.0,
            lock_font_size: false,

            msaa: Msaa::_8,

            label_size: 24.0,
        }
    }
}
impl GfxConfig {
    /// Returns the duration of one frame based on the configured FPS value.
    pub fn frame_duration(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.fps as f64)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ViewConfig {
    pub theta: f32,
    pub phi: f32,

    pub scale: f32,
    pub fov_3d: f32,
    pub fov_4d: f32,

    pub face_spacing: f32,
    pub sticker_spacing: f32,

    pub enable_outline: bool,
}
impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            theta: 0_f32,
            phi: 0_f32,

            scale: 1.0,
            fov_3d: 30_f32.to_radians(),
            fov_4d: 30_f32.to_radians(),

            face_spacing: 0.0,
            sticker_spacing: 0.0,

            enable_outline: true,
        }
    }
}
impl DeserializePerPuzzle<'_> for ViewConfig {
    type Proxy = Self;

    fn deserialize_from(value: Self::Proxy, _ty: PuzzleType) -> Self {
        value
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ColorsConfig {
    pub opacity: f32,

    pub faces: PerPuzzle<FaceColors>,

    pub background: [f32; 3],
    pub outline: [f32; 3],

    pub label_fg: [f32; 4],
    pub label_bg: [f32; 4],
}
impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            opacity: 1.0,

            faces: PerPuzzle::default(),

            background: colors::DEFAULT_BACKGROUND,
            outline: colors::DEFAULT_OUTLINE,

            label_fg: colors::DEFAULT_LABEL_FG,
            label_bg: colors::DEFAULT_LABEL_BG,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct FaceColors(pub Vec<[f32; 3]>);
impl std::ops::Index<usize> for FaceColors {
    type Output = [f32; 3];

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}
impl std::ops::IndexMut<usize> for FaceColors {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}
impl DeserializePerPuzzle<'_> for FaceColors {
    type Proxy = Self;

    fn deserialize_from(mut face_colors: Self, ty: PuzzleType) -> Self {
        face_colors.0.resize(ty.faces().len(), colors::GRAY);
        face_colors
    }
}

#[derive(Debug, Default, Clone)]
pub struct Keybind {
    pub key: Option<Key>,

    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub logo: bool,

    pub command: Command,
}
impl Serialize for Keybind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        KeybindSerde::from(self).serialize(serializer)
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct KeybindSerde<'a> {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub key: Option<Key>,

    #[serde(skip_serializing_if = "is_false")]
    pub ctrl: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub shift: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub alt: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub logo: bool,

    #[serde(borrow)]
    pub command: CommandSerde<'a>,
}
impl<'a> From<&'a Keybind> for KeybindSerde<'_> {
    fn from(keybind: &'a Keybind) -> Self {
        KeybindSerde {
            key: keybind.key,

            ctrl: keybind.ctrl,
            shift: keybind.shift,
            alt: keybind.alt,
            logo: keybind.logo,

            command: (&keybind.command).into(),
        }
    }
}
impl<'de> DeserializePerPuzzle<'de> for Keybind {
    type Proxy = KeybindSerde<'de>;

    fn deserialize_from(keybind: KeybindSerde<'de>, ty: PuzzleType) -> Self {
        Self {
            key: keybind.key,

            ctrl: keybind.ctrl,
            shift: keybind.shift,
            alt: keybind.alt,
            logo: keybind.logo,

            command: Command::deserialize_from(keybind.command, ty),
        }
    }
}
impl fmt::Display for Keybind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mods = key_names::mods_prefix_string(self.shift, self.ctrl, self.alt, self.logo);
        write!(f, "{}", mods)?;

        match self.key {
            Some(Key::Sc(sc)) => write!(f, "{}", key_names::key_name(sc)),
            // TODO: virtual key code names aren't platform-aware and might not
            // match scancode names
            Some(Key::Vk(vk)) => match vk {
                VirtualKeyCode::Key1 => write!(f, "1"),
                VirtualKeyCode::Key2 => write!(f, "2"),
                VirtualKeyCode::Key3 => write!(f, "3"),
                VirtualKeyCode::Key4 => write!(f, "4"),
                VirtualKeyCode::Key5 => write!(f, "5"),
                VirtualKeyCode::Key6 => write!(f, "6"),
                VirtualKeyCode::Key7 => write!(f, "7"),
                VirtualKeyCode::Key8 => write!(f, "8"),
                VirtualKeyCode::Key9 => write!(f, "9"),
                VirtualKeyCode::Key0 => write!(f, "0"),
                VirtualKeyCode::Scroll => write!(f, "ScrollLock"),
                VirtualKeyCode::Back => write!(f, "Backspace"),
                VirtualKeyCode::Return => write!(f, "Enter"),
                VirtualKeyCode::Capital => write!(f, "CapsLock"),
                other => write!(f, "{:?}", other),
            },
            None => write!(f, "(no key set)"),
        }
    }
}
impl Keybind {
    pub fn new(key: Option<Key>, mods: ModifiersState, command: Command) -> Self {
        let mut ret = Self {
            key,

            ctrl: mods.ctrl(),
            shift: mods.shift(),
            alt: mods.alt(),
            logo: mods.logo(),

            command,
        };
        ret.validate_keybind();
        ret
    }
    pub fn validate_keybind(&mut self) {
        if let Some(key) = self.key {
            use KeyMappingCode as Sc;
            use VirtualKeyCode as Vk;

            // Remove redundant modifiers.
            match key {
                Key::Sc(Sc::ControlLeft | Sc::ControlRight) => self.ctrl = false,
                Key::Sc(Sc::ShiftLeft | Sc::ShiftRight) => self.shift = false,
                Key::Sc(Sc::AltLeft | Sc::AltRight) => self.alt = false,
                Key::Sc(Sc::MetaLeft | Sc::MetaRight) => self.logo = false,

                Key::Vk(Vk::LControl | Vk::RControl) => self.ctrl = false,
                Key::Vk(Vk::LShift | Vk::RShift) => self.shift = false,
                Key::Vk(Vk::LAlt | Vk::RAlt) => self.alt = false,
                Key::Vk(Vk::LWin | Vk::RWin) => self.logo = false,

                _ => (),
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    /// OS-independent "key mapping code" which corresponds to OS-dependent
    /// scan code (i.e., physical location of key on keyboard).
    #[serde(with = "crate::serde_impl::KeyMappingCodeSerde")]
    Sc(KeyMappingCode),
    /// OS-independent "virtual key code" (i.e., semantic meaning of key on
    /// keyboard, taking into account the current layout).
    Vk(VirtualKeyCode),
}
impl Key {
    pub fn is_shift(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::ShiftLeft | Sc::ShiftRight) => true,
            Self::Vk(Vk::LShift | Vk::RShift) => true,
            _ => false,
        }
    }
    pub fn is_ctrl(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::ControlLeft | Sc::ControlRight) => true,
            Self::Vk(Vk::LControl | Vk::RControl) => true,
            _ => false,
        }
    }
    pub fn is_alt(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::AltLeft | Sc::AltRight) => true,
            Self::Vk(Vk::LAlt | Vk::RAlt) => true,
            _ => false,
        }
    }
    pub fn is_logo(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::MetaLeft | Sc::MetaRight) => true,
            Self::Vk(Vk::LWin | Vk::RWin) => true,
            _ => false,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(transparent)]
pub struct PerPuzzle<T>(HashMap<PuzzleType, T>);
impl<'de, T: DeserializePerPuzzle<'de>> Default for PerPuzzle<T>
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

    fn index(&self, puz_type: PuzzleType) -> &Self::Output {
        self.0.get(&puz_type).unwrap()
    }
}
impl<T> std::ops::IndexMut<PuzzleType> for PerPuzzle<T> {
    fn index_mut(&mut self, puz_type: PuzzleType) -> &mut Self::Output {
        self.0.get_mut(&puz_type).unwrap()
    }
}

pub trait DeserializePerPuzzle<'de> {
    type Proxy: Deserialize<'de>;

    fn deserialize_from(value: Self::Proxy, ty: PuzzleType) -> Self;
}

impl<'de> DeserializePerPuzzle<'de> for Vec<Keybind> {
    type Proxy = Vec<KeybindSerde<'de>>;

    fn deserialize_from(value: Vec<KeybindSerde<'de>>, ty: PuzzleType) -> Self {
        value
            .into_iter()
            .filter(|keybind| keybind.key.is_some())
            .map(|keybind| Keybind::deserialize_from(keybind, ty))
            .collect()
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Msaa {
    #[serde(rename = "0")]
    Off = 0,
    #[serde(rename = "2")]
    _2 = 2,
    #[serde(rename = "4")]
    _4 = 4,
    #[serde(other, rename = "8")]
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

fn is_false(x: &bool) -> bool {
    !x
}
