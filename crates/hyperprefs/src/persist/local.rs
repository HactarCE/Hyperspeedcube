use eyre::Result;
use serde::Serialize;

pub fn user_config_source() -> Result<impl config::Source> {
    Ok(config::File::from(hyperpaths::prefs_file()?))
}

pub fn save(prefs_data: &impl Serialize) -> Result<()> {
    let path = hyperpaths::prefs_file()?;
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    serde_norway::to_writer(std::fs::File::create(path)?, prefs_data)?;
    Ok(())
}

pub fn backup_prefs_file() {
    if let Ok(path) = hyperpaths::prefs_file() {
        hyperpaths::move_to_backup_file(path);
    }
}
