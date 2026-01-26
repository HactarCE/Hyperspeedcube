//! User preferences.
//!
//! For a list of key names, see `VirtualKeyCode` in this file:
//! <https://github.com/rust-windowing/winit/blob/master/src/event.rs>

#![allow(missing_docs)] // too many things to document

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate strum;

use std::collections::BTreeMap;
use std::path::PathBuf;

use bitvec::vec::BitVec;
use eyre::{OptionExt, eyre};
use hyperpuzzle_core::{PerspectiveDim, Puzzle, Rgb};
use serde::{Deserialize, Serialize};

mod animation;
mod colors;
mod filters;
mod image_generator;
mod info;
mod interaction;
mod key;
mod sidebar;
// mod keybinds;
// mod mousebinds;
pub mod persist;
mod presets;
mod schema;
mod serde_impl;
mod styles;
mod view;

pub use animation::*;
pub use colors::*;
pub use filters::*;
pub use image_generator::*;
pub use info::*;
pub use interaction::*;
pub use key::AnyKey;
pub use presets::*;
pub use schema::PrefsConvert;
pub use sidebar::*;
pub use styles::*;
pub use view::*;

const PREFS_FILE_FORMAT: config::FileFormat = config::FileFormat::Yaml;
const DEFAULT_PREFS_STR: &str = include_str!("default.yaml");
pub const DEFAULT_PRESET_NAME: &str = "Default";

lazy_static! {
    static ref DEFAULT_PREFS_RAW: schema::current::Preferences =
        serde_norway::from_str(DEFAULT_PREFS_STR).expect("error loading default preferences");
    pub static ref DEFAULT_PREFS: Preferences =
        Preferences::from_serde(&(), DEFAULT_PREFS_RAW.clone());
}

#[derive(Debug, Default)]
pub struct Preferences {
    pub needs_save: bool,
    pub needs_save_eventually: bool,

    pub eula: bool,
    pub record_time: bool,
    pub online_mode: bool,
    pub check_for_updates: bool,

    pub log_file: Option<PathBuf>,

    pub sidebar: SidebarPreferences,

    pub info: InfoPreferences,

    pub image_generator: ImageGeneratorPreferences,

    pub animation: PresetsList<AnimationPreferences>,
    pub interaction: InteractionPreferences,
    pub styles: StylePreferences,
    pub filter_styles: PresetsList<PieceStyle>,

    pub view_3d: PresetsList<ViewPreferences>,
    pub view_4d: PresetsList<ViewPreferences>,

    pub color_palette: GlobalColorPalette,
    /// Color scheme preferences for each color system.
    pub color_schemes: ColorSchemePreferences,

    /// Filter preferences for each puzzle.
    pub filters: BTreeMap<String, PuzzleFilterPreferences>,

    /// Whether to show experimental puzzles.
    pub show_experimental_puzzles: bool,

    // TODO: remove this when implementing keybinds
    pub keybinds: std::marker::PhantomData<AnyKey>,
}
impl schema::PrefsConvert for Preferences {
    type DeserContext = ();
    type SerdeFormat = schema::current::Preferences;

    fn to_serde(&self) -> Self::SerdeFormat {
        let Self {
            needs_save: _,
            needs_save_eventually: _,
            record_time,
            online_mode,
            check_for_updates,
            eula,
            log_file,
            sidebar,
            info,
            image_generator,
            animation,
            interaction,
            styles,
            filter_styles,
            view_3d,
            view_4d,
            color_palette,
            color_schemes,
            filters,
            show_experimental_puzzles,
            keybinds: _,
        } = self;

        let filters = filters
            .iter()
            .filter(|(_k, v)| !v.is_default())
            .map(|(k, v)| (k.clone(), v.to_serde()))
            .collect();

        schema::current::Preferences {
            record_time: *record_time,
            online_mode: *online_mode,
            check_for_updates: *check_for_updates,
            eula: *eula,
            log_file: log_file.clone(),
            sidebar: sidebar.clone(),
            info: info.clone(),
            image_generator: image_generator.clone(),
            animation: animation.to_serde(),
            interaction: interaction.clone(),
            styles: styles.clone(),
            filter_styles: filter_styles.to_serde(),
            view_3d: view_3d.to_serde(),
            view_4d: view_4d.to_serde(),
            color_palette: color_palette.to_serde(),
            color_schemes: color_schemes.to_serde(),
            filters,
            show_experimental_puzzles: *show_experimental_puzzles,
        }
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        let schema::current::Preferences {
            record_time,
            online_mode,
            check_for_updates,
            eula,
            log_file,
            sidebar,
            info,
            image_generator,
            animation,
            interaction,
            styles,
            filter_styles,
            view_3d,
            view_4d,
            color_palette,
            color_schemes,
            filters,
            show_experimental_puzzles,
        } = value;

        self.record_time = record_time;
        self.online_mode = online_mode;
        self.check_for_updates = check_for_updates;
        self.eula = eula;
        self.log_file = log_file;
        self.sidebar = sidebar;
        self.info = info;
        self.image_generator = image_generator;
        self.styles = styles;

        self.animation.reload_from_serde(ctx, animation);
        self.interaction.reload_from_serde(ctx, interaction);
        self.filter_styles.reload_from_serde(ctx, filter_styles);

        self.view_3d.reload_from_serde(ctx, view_3d);
        self.view_4d.reload_from_serde(ctx, view_4d);

        self.color_palette.reload_from_serde(ctx, color_palette);
        self.color_schemes.reload_from_serde(ctx, color_schemes);

        let defaults = &DEFAULT_PREFS_RAW;
        self.animation
            .set_builtin_presets_from_default_prefs(ctx, &defaults.animation);
        self.filter_styles
            .set_builtin_presets_from_default_prefs(ctx, &defaults.filter_styles);
        self.view_3d
            .set_builtin_presets_from_default_prefs(ctx, &defaults.view_3d);
        self.view_4d
            .set_builtin_presets_from_default_prefs(ctx, &defaults.view_4d);

        self.show_experimental_puzzles = show_experimental_puzzles;

        schema::reload_btreemap(&mut self.filters, &self.filter_styles, filters);
    }
}
impl Preferences {
    /// Loads preferences from `user_config_source`. If loading fails, then the
    /// existing preferences are backed up (if possible) and `backup` (or else
    /// the default preferences) is returned.
    pub fn load(backup: Option<Self>) -> Self {
        lazy_static::initialize(&DEFAULT_PREFS);

        let mut config = config::Config::builder()
            .set_default("version", schema::CURRENT_VERSION)
            .expect("error setting preferences schema version");

        // Load default preferences.
        let default_config_source = config::File::from_str(DEFAULT_PREFS_STR, PREFS_FILE_FORMAT);
        config = config.add_source(default_config_source.clone());

        // Load user preferences.
        match persist::user_config_source() {
            Ok(config_source) => config = config.add_source(config_source),
            Err(e) => log::warn!("Error loading user preferences: {e}"),
        }

        config
            .build()
            .and_then(|c| c.try_deserialize::<schema::AnyVersion>())
            .map(schema::AnyVersion::into_current)
            .map(|value| schema::PrefsConvert::from_serde(&(), value))
            .unwrap_or_else(|e| {
                log::warn!("Error loading preferences: {e}");

                persist::backup_prefs_file();

                // Try backup
                backup
                    .or_else(|| {
                        // Try default config
                        config::Config::builder()
                            .add_source(default_config_source)
                            .build()
                            .ok()?
                            .try_deserialize()
                            .map(|value| schema::PrefsConvert::from_serde(&(), value))
                            .ok()
                    })
                    .unwrap_or_default()
            })
    }

    pub fn save(&mut self) {
        self.needs_save = false;
        self.needs_save_eventually = false;
        let (tx, _join_handle) = &*persist::PREFS_SAVE_THREAD;
        if let Err(e) = tx.send(persist::PrefsSaveCommand::Save(Box::new(self.to_serde()))) {
            log::error!("Error saving preferences: {e}");
        }
    }
    pub fn block_on_final_save(&mut self) {
        // IIFE to mimic try_block
        let result = (|| -> eyre::Result<()> {
            let (tx, join_handle) = &*persist::PREFS_SAVE_THREAD;
            tx.send(persist::PrefsSaveCommand::Quit)?;
            let join_handle = join_handle
                .lock()
                .take()
                .ok_or_eyre("no thread join handle")?;
            join_handle.join().map_err(|e| eyre!("{e:?}"))?;
            Ok(())
        })();
        if let Err(e) = result {
            log::error!("Error waiting for preferences saving: {e}");
        }
    }

    pub fn perspective_view_presets(&self, dim: PerspectiveDim) -> &PresetsList<ViewPreferences> {
        match dim {
            PerspectiveDim::Dim3D => &self.view_3d,
            PerspectiveDim::Dim4D => &self.view_4d,
        }
    }
    pub fn perspective_view_presets_mut(
        &mut self,
        dim: PerspectiveDim,
    ) -> &mut PresetsList<ViewPreferences> {
        match dim {
            PerspectiveDim::Dim3D => &mut self.view_3d,
            PerspectiveDim::Dim4D => &mut self.view_4d,
        }
    }

    pub fn filters_mut(&mut self, puzzle: &Puzzle) -> &mut PuzzleFilterPreferences {
        self.filters.entry(puzzle.meta.id.clone()).or_default()
    }

    pub fn background_color(&self, dark_mode: bool) -> Rgb {
        match dark_mode {
            true => self.styles.dark_background_color,
            false => self.styles.light_background_color,
        }
    }

    pub fn first_custom_style(&self) -> Option<PresetRef> {
        Some(self.filter_styles.user_presets().next()?.new_ref())
    }
    pub fn base_style(&self, style_ref: &Option<PresetRef>) -> PieceStyle {
        style_ref
            .as_ref()
            .and_then(|p| self.filter_styles.get(&p.name()))
            .map(|preset| preset.value)
            .unwrap_or(self.styles.default)
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct PieceFilter {
    /// Hexadecimal-encoded bitstring of which pieces are visible.
    #[serde(with = "crate::serde_impl::hex_bitvec")]
    pub visible_pieces: BitVec,
    /// Opacity of hidden pieces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden_opacity: Option<f32>,
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    #[test]
    fn test_yaml_preserves_map_order() {
        let mut m = IndexMap::<String, usize>::new();
        m.insert("tenpo".to_string(), 0);
        m.insert("mute".to_string(), 1);
        m.insert("la".to_string(), 2);
        m.insert("mi".to_string(), 3);
        m.insert("toki".to_string(), 4);
        m.insert("pona".to_string(), 5);
        let serialized = serde_norway::to_string(&m).unwrap();
        let deserialized: IndexMap<String, usize> = serde_norway::from_str(&serialized).unwrap();
        for (i, &v) in deserialized.values().enumerate() {
            assert_eq!(i, v);
        }
    }
}
