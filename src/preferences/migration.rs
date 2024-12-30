use config::{Config, ConfigError};
use std::collections::{BTreeMap, BTreeSet};

use super::*;

pub(super) const LATEST_VERSION: u32 = 1;

pub(super) fn try_deserialize(c: Config) -> Result<Preferences, ConfigError> {
    let version: u32 = match c.get_int("version") {
        Ok(n) => n.try_into().unwrap_or(0),
        Err(ConfigError::NotFound(_)) => 0,
        Err(e) => return Err(e),
    };
    if version < LATEST_VERSION {
        log::info!(
            "Migrating preferences from v{version} to v{}",
            migration::LATEST_VERSION,
        );
        persist::backup_prefs_file();
    }
    Ok(match version {
        0 => c.try_deserialize::<v0::PrefsCompat>()?.into(),
        1 => c.try_deserialize::<v1::PrefsCompat>()?,
        _ => c.try_deserialize::<Preferences>()?,
    })
}

/// Compatibility layer for deserializing older versions of the preferences
/// format.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum PrefsCompat {
    /// v0.9.x to present
    V1 {
        #[serde(rename = "version")]
        _version: monostate::MustBe!(1),
        #[serde(flatten)]
        remaining: Box<v1::PrefsCompat>,
    },
    /// v0.8.x
    V0 {
        #[serde(flatten)]
        remaining: Box<v0::PrefsCompat>,
    },
}
impl From<PrefsCompat> for Preferences {
    fn from(p: PrefsCompat) -> Self {
        match p {
            PrefsCompat::V1 { remaining, .. } => *remaining,
            PrefsCompat::V0 { remaining } => (*remaining).into(),
        }
    }
}
impl PrefsCompat {
    pub fn is_latest(&self) -> bool {
        self.version() == LATEST_VERSION
    }
    pub fn version(&self) -> u32 {
        match self {
            Self::V1 { .. } => 1,
            Self::V0 { .. } => 0,
        }
    }
}

mod v1 {
    use super::*;

    pub type PrefsCompat = Preferences;
}

mod v0 {
    use super::*;

    #[derive(Deserialize, Debug, Default)]
    #[serde(default)]
    pub struct PrefsCompat {
        view_3d: WithPresets<ViewPreferences>,
        view_4d: WithPresets<ViewPreferences>,

        piece_filters: PerPuzzle<BTreeMap<String, String>>,

        puzzle_keybinds: PerPuzzleFamily<Vec<Keybind<PuzzleCommand>>>,

        #[serde(flatten)]
        remaining: v1::PrefsCompat,
    }
    impl From<PrefsCompat> for v1::PrefsCompat {
        fn from(p: PrefsCompat) -> Self {
            Self {
                view_3d: p.view_3d.into(),
                view_4d: p.view_4d.into(),

                piece_filters: PerPuzzle {
                    map: p
                        .piece_filters
                        .map
                        .into_iter()
                        .map(|(puzzle_type, presets)| {
                            (puzzle_type, convert_piece_filter_preset_list(presets))
                        })
                        .collect(),
                    default: convert_piece_filter_preset_list(p.piece_filters.default),
                },

                puzzle_keybinds: PerPuzzleFamily {
                    map: p
                        .puzzle_keybinds
                        .map
                        .into_iter()
                        .map(|(puzzle_family, keybinds)| {
                            (puzzle_family, convert_puzzle_keybind_set(keybinds))
                        })
                        .collect(),
                    default: Default::default(),
                },

                ..p.remaining
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Default)]
    #[serde(default)]
    pub struct WithPresets<T: Default> {
        #[serde(flatten)]
        pub current: T,
        pub active_preset: Option<String>,
        pub presets: BTreeMap<String, T>,
    }

    pub fn convert_piece_filter_preset_list(
        presets: BTreeMap<String, String>,
    ) -> Vec<Preset<PieceFilter>> {
        presets
            .into_iter()
            .map(|(name, visible_pieces_string)| Preset {
                preset_name: name,
                value: PieceFilter {
                    visible_pieces: crate::serde_impl::hex_bitvec::b16_string_to_bitvec(
                        &visible_pieces_string,
                    ),
                    hidden_opacity: None,
                },
            })
            .collect()
    }

    pub fn convert_puzzle_keybind_set(keybinds: Vec<Keybind<PuzzleCommand>>) -> PuzzleKeybindSets {
        PuzzleKeybindSets {
            active: "default".to_string(),
            sets: vec![Preset {
                preset_name: "default".to_string(),
                value: KeybindSet {
                    includes: BTreeSet::new(),
                    keybinds,
                },
            }],
        }
    }
}
impl<T: Default + Clone> From<v0::WithPresets<T>> for WithPresets<T> {
    fn from(p: v0::WithPresets<T>) -> Self {
        WithPresets {
            current: p.current,
            active_preset: p.active_preset.and_then(|preset_name| {
                let value = p.presets.get(&preset_name)?.clone();
                Some(Preset { preset_name, value })
            }),
            presets: p
                .presets
                .into_iter()
                .map(|(name, value)| Preset {
                    preset_name: name,
                    value,
                })
                .collect(),
        }
    }
}
