use eyre::Result;
use serde::Serialize;

pub fn user_config_source() -> Result<impl config::Source> {
    Ok(config::File::from(crate::paths::prefs_file()?))
}

pub fn save(prefs_data: &impl Serialize) -> Result<()> {
    let path = crate::paths::prefs_file()?;
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    serde_yml::to_writer(std::fs::File::create(path)?, prefs_data)?;
    Ok(())
}

pub fn backup_prefs_file() {
    let Ok(prefs_path) = crate::paths::prefs_file() else {
        return;
    };

    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let Ok(backup_path) = crate::paths::backup_prefs_file_path(now) else {
        return;
    };

    if std::fs::rename(prefs_path, &backup_path).is_ok() {
        log::info!(
            "Backup of old preferences stored at {}",
            backup_path.display(),
        );
    }
}
