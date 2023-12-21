use eyre::eyre;
use serde::Serialize;

const PREFS_KEY: &str = "hyperspeedcube_preferences";

#[derive(Error, Debug, Copy, Clone, PartialEq, Eq)]
pub enum PrefsError {
    #[error("unable to access browser local storage")]
    CannotAccessLocalStorage,
    #[error("no saved preferences")]
    NoSavedPreferences,
}

pub fn user_config_source() -> Result<impl config::Source, PrefsError> {
    Ok(config::File::from_str(
        &local_storage()?
            .get_item(PREFS_KEY)
            .ok()
            .flatten()
            .ok_or(PrefsError::NoSavedPreferences)?,
        super::PREFS_FILE_FORMAT,
    ))
}

pub fn save(prefs_data: &impl Serialize) -> Result<()> {
    let prefs_string = serde_yaml::to_string(prefs_data).map_err(|e| eyre!(e))?;
    local_storage()?
        .set_item(PREFS_KEY, &prefs_string)
        .map_err(|e| eyre!(format!("{e:?}")))
}

pub fn backup_prefs_file() {
    log::warn!("Cannot backup preferences on web")
}

fn local_storage() -> Result<web_sys::Storage, PrefsError> {
    web_sys::window()
        .unwrap()
        .local_storage()
        .unwrap()
        .ok_or(PrefsError::CannotAccessLocalStorage)
}
