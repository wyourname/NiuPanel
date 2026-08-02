use anyhow::{Result, bail};
use migration::{Migrator, MigratorTrait};
use niupanel_common::config::Config;
use niupanel_common::{info, warn};
use niupanel_core::settings::{SYSTEM_SESSION_KEY, SettingsManager};
use sea_orm::sqlx::sqlite::{SqliteJournalMode, SqliteSynchronous};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_DATABASE_MAX_CONNECTIONS: u64 = 1;
const DEFAULT_SQLITE_BUSY_TIMEOUT_MS: u64 = 5_000;

pub async fn connect_database(database_url: &str) -> Result<DatabaseConnection> {
    info!("Connecting to database...");
    let mut options = ConnectOptions::new(database_url.to_owned());

    if database_url.starts_with("sqlite:") {
        let max_connections = env_number(
            "DATABASE_MAX_CONNECTIONS",
            DEFAULT_DATABASE_MAX_CONNECTIONS,
            1,
            32,
        ) as u32;
        let busy_timeout_ms = env_number(
            "SQLITE_BUSY_TIMEOUT_MS",
            DEFAULT_SQLITE_BUSY_TIMEOUT_MS,
            1,
            120_000,
        );
        let use_wal = sqlite_database_path(database_url).is_some();

        options
            .max_connections(max_connections)
            .min_connections(1)
            .sqlx_logging(false)
            .map_sqlx_sqlite_opts(move |sqlite| {
                let sqlite = sqlite
                    .foreign_keys(true)
                    .synchronous(SqliteSynchronous::Full)
                    .busy_timeout(Duration::from_millis(busy_timeout_ms));
                if use_wal {
                    sqlite.journal_mode(SqliteJournalMode::Wal)
                } else {
                    sqlite
                }
            });
    }

    let db = Database::connect(options).await?;
    harden_sqlite_permissions(database_url)?;
    Ok(db)
}

fn env_number(name: &str, default: u64, min: u64, max: u64) -> u64 {
    let Ok(raw) = std::env::var(name) else {
        return default;
    };

    match parse_bounded_number(&raw, min, max) {
        Some(value) => value,
        None => {
            warn!("Ignoring invalid {name}={raw:?}; using {default}");
            default
        }
    }
}

fn parse_bounded_number(raw: &str, min: u64, max: u64) -> Option<u64> {
    let value = raw.trim().parse::<u64>().ok()?;
    (min..=max).contains(&value).then_some(value)
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
    use sea_orm::{ConnectionTrait, DbBackend, Statement};

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

    #[test]
    fn accepts_only_bounded_database_tuning_values() {
        assert_eq!(parse_bounded_number("1", 1, 32), Some(1));
        assert_eq!(parse_bounded_number(" 16 ", 1, 32), Some(16));
        assert_eq!(parse_bounded_number("0", 1, 32), None);
        assert_eq!(parse_bounded_number("33", 1, 32), None);
        assert_eq!(parse_bounded_number("invalid", 1, 32), None);
    }

    #[tokio::test]
    async fn file_sqlite_uses_durable_wal_settings() {
        let temp = tempfile::tempdir().unwrap();
        let database_path = temp.path().join("runtime.db");
        let database_url = format!("sqlite://{}?mode=rwc", database_path.display());
        let db = connect_database(&database_url).await.unwrap();

        let journal_mode = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "PRAGMA journal_mode".to_string(),
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get_by_index::<String>(0)
            .unwrap();
        let synchronous = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "PRAGMA synchronous".to_string(),
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get_by_index::<i64>(0)
            .unwrap();
        let foreign_keys = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "PRAGMA foreign_keys".to_string(),
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get_by_index::<i64>(0)
            .unwrap();

        assert_eq!(journal_mode, "wal");
        assert_eq!(synchronous, 2);
        assert_eq!(foreign_keys, 1);
    }
}
