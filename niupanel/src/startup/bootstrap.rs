use anyhow::{Result, bail};
use migration::{Migrator, MigratorTrait};
use niupanel_common::config::Config;
use niupanel_common::{info, warn};
use niupanel_core::settings::{SYSTEM_SESSION_KEY, SettingsManager};
use sea_orm::{Database, DatabaseConnection};
use std::path::{Path, PathBuf};

pub async fn connect_database(database_url: &str) -> Result<DatabaseConnection> {
    info!("Connecting to database...");
    let db = Database::connect(database_url).await?;
    harden_sqlite_permissions(database_url)?;
    Ok(db)
}

fn sqlite_database_path(database_url: &str) -> Option<PathBuf> {
    let raw_path = database_url.strip_prefix("sqlite://")?.split('?').next()?;
    if raw_path.is_empty() || raw_path == ":memory:" {
        return None;
    }
    Some(Path::new(raw_path).to_path_buf())
}

fn harden_sqlite_permissions(database_url: &str) -> Result<()> {
    let Some(database_path) = sqlite_database_path(database_url) else {
        return Ok(());
    };
    if !database_path.exists() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&database_path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub async fn run_migrations(db: &DatabaseConnection) -> Result<()> {
    info!("Running migrations...");
    Migrator::up(db, None).await?;
    info!("Migrations applied successfully");
    Ok(())
}

pub async fn init_session_key(db: &DatabaseConnection, config: &mut Config) -> Result<()> {
    const LEGACY_INSECURE_KEY: &str =
        "1PxkXmg3LzINVOa6N9JMPhLeibzrh+zSDzZ7kze41WUUwaBB6T0FhJ+WvtLrbu5in8Yy7AsbP7DwcWmOkADxMg==";

    let resolved_key = SettingsManager::get(db, SYSTEM_SESSION_KEY).await?;

    if !is_secure_session_key(&resolved_key) || resolved_key == LEGACY_INSECURE_KEY {
        if let Ok(explicit_key) = std::env::var("NIU_SESSION_KEY")
            && !explicit_key.is_empty()
            && !is_secure_session_key(&explicit_key)
        {
            bail!("NIU_SESSION_KEY must contain at least 64 bytes");
        }
        // Config::from_env creates a random candidate when SESSION_KEY is
        // absent. Persist it so the key remains stable across restarts.
        // Existing installations carrying the old public key are rotated.
        let new_key = if !is_secure_session_key(&config.session_key)
            || config.session_key == LEGACY_INSECURE_KEY
        {
            nanoid::nanoid!(88)
        } else {
            config.session_key.clone()
        };
        SettingsManager::set(db, SYSTEM_SESSION_KEY, &new_key, Some("Security")).await?;
        config.session_key = new_key;
        info!("Secure installation-specific session key generated and persisted");
    } else {
        config.session_key = resolved_key;
        info!("Session key initialized from ENV or database");
    }

    Ok(())
}

fn is_secure_session_key(value: &str) -> bool {
    value.as_bytes().len() >= 64
}

pub fn set_global_config(config: Config) {
    if niupanel_common::config::CONFIG.set(config).is_err() {
        warn!("Global config was already set");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_file_backed_sqlite_paths() {
        assert_eq!(
            sqlite_database_path("sqlite://data/database/niupanel.db?mode=rwc"),
            Some(PathBuf::from("data/database/niupanel.db"))
        );
        assert_eq!(sqlite_database_path("sqlite://:memory:"), None);
        assert_eq!(sqlite_database_path("postgres://localhost/niupanel"), None);
    }

    #[test]
    fn session_key_requires_at_least_64_bytes() {
        assert!(!is_secure_session_key(""));
        assert!(!is_secure_session_key("replace-with-a-random-session-key"));
        assert!(is_secure_session_key(&"x".repeat(64)));
    }
}
