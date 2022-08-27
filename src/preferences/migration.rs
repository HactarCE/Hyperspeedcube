use std::collections::BTreeMap;

use super::*;

pub(super) const LATEST_VERSION: u32 = 1;

/// Compatibility layer for deserializing older versions of the preferences
/// format.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub(super) enum PrefsCompat {
    /// v0.9.x to present
    V1 {
        #[serde(rename = "version")]
        _version: monostate::MustBe!(1),
        #[serde(flatten)]
        remaining: v1::PrefsCompat,
    },
    /// v0.8.x
    V0 {
        #[serde(flatten)]
        remaining: v0::PrefsCompat,
    },
}
impl From<PrefsCompat> for Preferences {
    fn from(p: PrefsCompat) -> Self {
        match p {
            PrefsCompat::V1 { remaining, .. } => remaining.into(),
            PrefsCompat::V0 { remaining } => remaining.into(),
        }
    }
}
impl PrefsCompat {
    pub fn is_latest(&self) -> bool {
        self.version() == LATEST_VERSION
    }
    pub fn version(&self) -> u32 {
        match self {
            PrefsCompat::V1 { .. } => 1,
            PrefsCompat::V0 { .. } => 0,
        }
    }
}

pub(super) mod v1 {
    use super::*;

    pub type PrefsCompat = Preferences;
}

pub(super) mod v0 {
    use super::*;

    #[derive(Deserialize, Debug, Default)]
    #[serde(default)]
    pub struct PrefsCompat {
        view_3d: WithPresets<ViewPreferences>,
        view_4d: WithPresets<ViewPreferences>,

        piece_filters: PerPuzzle<BTreeMap<String, String>>,

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

                ..p.remaining
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Default)]
    #[serde(default)]
    pub(super) struct WithPresets<T: Default> {
        #[serde(flatten)]
        pub current: T,
        pub active_preset: Option<String>,
        pub presets: BTreeMap<String, T>,
    }

    pub(super) fn convert_piece_filter_preset_list(
        presets: BTreeMap<String, String>,
    ) -> Vec<Preset<PieceFilter>> {
        presets
            .into_iter()
            .map(|(name, visible_pieces)| Preset {
                preset_name: name,
                value: PieceFilter {
                    visible_pieces,
                    hidden_opacity: None,
                },
            })
            .collect()
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
