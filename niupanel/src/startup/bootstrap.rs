use anyhow::Result;
use migration::{Migrator, MigratorTrait};
use niupanel_common::config::Config;
use niupanel_common::{info, warn};
use niupanel_core::settings::{SYSTEM_SESSION_KEY, SettingsManager};
use sea_orm::{Database, DatabaseConnection};

pub async fn connect_database(database_url: &str) -> Result<DatabaseConnection> {
    info!("Connecting to database...");
    let db = Database::connect(database_url).await?;
    Ok(db)
}

pub async fn run_migrations(db: &DatabaseConnection) -> Result<()> {
    info!("Running migrations...");
    Migrator::up(db, None).await?;
    info!("Migrations applied successfully");
    Ok(())
}

pub async fn init_session_key(db: &DatabaseConnection, config: &mut Config) -> Result<()> {
    let default_key =
        "1PxkXmg3LzINVOa6N9JMPhLeibzrh+zSDzZ7kze41WUUwaBB6T0FhJ+WvtLrbu5in8Yy7AsbP7DwcWmOkADxMg==";

    let resolved_key = SettingsManager::get(db, SYSTEM_SESSION_KEY).await?;

    if resolved_key == default_key && config.session_key == default_key {
        let new_key = nanoid::nanoid!(88);
        SettingsManager::set(db, SYSTEM_SESSION_KEY, &new_key, Some("Security")).await?;
        config.session_key = new_key;
        info!("New secure session key generated and persisted to database");
    } else {
        config.session_key = resolved_key;
        info!("Session key initialized from ENV or database");
    }

    Ok(())
}

pub fn set_global_config(config: Config) {
    if niupanel_common::config::CONFIG.set(config).is_err() {
        warn!("Global config was already set");
    }
}
