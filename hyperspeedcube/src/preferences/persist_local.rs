use eyre::Result;
use serde::Serialize;

const PREFS_FILE_NAME: &str = "hsc2-prefs";
const PREFS_FILE_EXTENSION: &str = "yaml";

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

    let mut backup_path = prefs_path.to_owned();
    backup_path.pop();

    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    backup_path.push(format!(
        "{}_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}_bak.{}",
        PREFS_FILE_NAME,
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        PREFS_FILE_EXTENSION,
    ));

    if std::fs::rename(prefs_path, &backup_path).is_ok() {
        log::info!(
            "Backup of old preferences stored at {}",
            backup_path.display(),
        );
    }
}
