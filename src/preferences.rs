//! User preferences.
//!
//! For a list of key names, see `VirtualKeyCode` in this file:
//! https://github.com/rust-windowing/winit/blob/master/src/event.rs

use directories::ProjectDirs;
use enum_map::EnumMap;
use key_names::KeyMappingCode;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use send_wrapper::SendWrapper;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io::Write;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::{mpsc, Mutex, MutexGuard, TryLockError};
use std::time::Duration;
use winit::event::{ModifiersState, VirtualKeyCode};

use crate::colors;
use crate::puzzle::commands::CommandSerde;
use crate::puzzle::{traits::*, Command, PuzzleType};

const PREFS_FILE_NAME: &str = "hyperspeedcube";
const PREFS_FILE_EXTENSION: &str = "yaml";
const PREFS_FILE_FORMAT: config::FileFormat = config::FileFormat::Yaml;
const DEFAULT_PREFS: &str = include_str!("../resources/default.yaml");

lazy_static! {
    static ref PREFERENCES: Mutex<Preferences> = Mutex::new(Preferences::load(None));
    static ref PROJECT_DIRS: Option<ProjectDirs> = ProjectDirs::from("", "", "Hyperspeedcube");
    static ref PREFS_FILE_PATH: Result<PathBuf, NoPreferencesPath> = match &*PROJECT_DIRS {
        Some(proj_dirs) => {
            let mut p = proj_dirs.config_dir().to_owned();
            p.push(format!("{}.{}", PREFS_FILE_NAME, PREFS_FILE_EXTENSION));
            Ok(p)
        }
        None => Err(NoPreferencesPath),
    };
}

lazy_static! {
    static ref RX: Mutex<mpsc::Receiver<DebouncedEvent>> = {
        let (tx, rx) = mpsc::channel();
        match Watcher::new(tx, Duration::from_secs_f64(0.5)) {
            Ok(w) => *WATCHER.lock().unwrap() = Some(w),
            Err(e) => eprintln!("Error initializing preferences file watcher: {}", e),
        }
        unwatch_during(|| ());

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

pub(crate) fn get_prefs<'a>() -> MutexGuard<'a, Preferences> {
    match PREFERENCES.try_lock() {
        Ok(mut prefs) => {
            let rx = RX.lock().unwrap();
            while let Ok(event) = rx.try_recv() {
                match event {
                    DebouncedEvent::Create(path)
                    | DebouncedEvent::Write(path)
                    | DebouncedEvent::Rename(_, path) => {
                        if let Ok(prefs_path) = &*PREFS_FILE_PATH {
                            if path == *prefs_path {
                                eprintln!("Reloading preferences from file");
                                *prefs = Preferences::load(Some(&*prefs));
                            }
                        }
                    }
                    _ => (),
                }
            }

            prefs
        }
        Err(TryLockError::Poisoned(e)) => panic!("preferences mutex poisoned: {}", e),
        Err(TryLockError::WouldBlock) => panic!("preferences mutex double-locked"),
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
struct NoPreferencesPath;
impl fmt::Display for NoPreferencesPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unable to get preferences file path")
    }
}
impl Error for NoPreferencesPath {}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Preferences {
    #[serde(skip)]
    pub needs_save: bool,
    #[serde(skip)]
    pub window_states: WindowStates,

    pub log_file: PathBuf,

    pub gfx: GfxPreferences,
    pub view: PerPuzzle<ViewPreferences>,
    pub colors: ColorPreferences,
    pub keybinds: PerPuzzle<Vec<Keybind>>,
}
impl Default for Preferences {
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
impl Preferences {
    pub fn load(backup: Option<&Self>) -> Self {
        let mut config = config::Config::new();

        // Load default preferences.
        let default_config_source = config::File::from_str(DEFAULT_PREFS, PREFS_FILE_FORMAT);
        let _ = config.merge(default_config_source.clone());

        // Load user preferences.
        match &*PREFS_FILE_PATH {
            Ok(path) => {
                let _ = config.merge(config::File::from(path.as_ref()));
            }
            Err(e) => eprintln!("Error loading user preferences: {}", e),
        }

        config.try_into::<Self>().unwrap_or_else(|e| {
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
                    datetime.month() as u8 ,
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
            backup.cloned()
            // Try just default config
.            or_else(||
                config::Config::new().with_merged(default_config_source).unwrap().try_into().ok()
            ).unwrap_or_else(Preferences::default)
        })
    }

    pub fn save(&mut self) {
        if self.needs_save {
            if let Err(e) = self._save() {
                eprintln!("Error saving preferences: {}", e);
            }
        }
    }
    fn _save(&mut self) -> Result<(), Box<dyn Error>> {
        // TODO: use try block
        self.needs_save = false;
        let path = PREFS_FILE_PATH.as_ref()?;
        unwatch_during(|| {
            if let Some(p) = path.parent() {
                std::fs::create_dir_all(p)?;
            }
            serde_yaml::to_writer(std::fs::File::create(path)?, self)?;
            Ok(())
        })
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GfxPreferences {
    pub fps: u32,
    pub font_size: f32,
    #[serde(skip)]
    pub lock_font_size: bool,

    pub msaa: Msaa,

    pub label_size: f32, // TODO: remove or move this
}
impl Default for GfxPreferences {
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
impl GfxPreferences {
    /// Returns the duration of one frame based on the configured FPS value.
    pub fn frame_duration(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.fps as f64)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ViewPreferences {
    pub theta: f32,
    pub phi: f32,

    pub scale: f32,
    pub fov_3d: f32,
    pub fov_4d: f32,

    pub face_spacing: f32,
    pub sticker_spacing: f32,

    pub enable_outline: bool,
}
impl Default for ViewPreferences {
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
impl DeserializePerPuzzle<'_> for ViewPreferences {
    type Proxy = Self;

    fn deserialize_from(value: Self::Proxy, _ty: PuzzleType) -> Self {
        value
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ColorPreferences {
    pub opacity: f32,

    pub faces: PerPuzzle<FaceColors>,

    pub background: [f32; 3],
    pub outline: [f32; 3],

    pub label_fg: [f32; 4],
    pub label_bg: [f32; 4],
}
impl Default for ColorPreferences {
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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
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

    fn index(&self, puz_type: PuzzleType) -> &Self::Output {
        &self.0[puz_type]
    }
}
impl<T> std::ops::IndexMut<PuzzleType> for PerPuzzle<T> {
    fn index_mut(&mut self, puz_type: PuzzleType) -> &mut Self::Output {
        &mut self.0[puz_type]
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
