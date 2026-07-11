pub mod registry;
pub mod update;

use crate::event_bus::{EventBus, SystemEvent, SystemNotification};
pub use niupanel_common::constants::defaults;
pub use niupanel_common::constants::settings::*;
use niupanel_common::error::Result;
use niupanel_entity::settings;
pub use registry::{SETTINGS_REGISTRY, SettingDef};
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use std::collections::HashMap;

// --- Settings Service (Stateful) ---

#[derive(Clone)]
pub struct SettingsService {
    db: sea_orm::DatabaseConnection,
    event_bus: EventBus,
}

impl SettingsService {
    pub fn new(db: sea_orm::DatabaseConnection, event_bus: EventBus) -> Self {
        Self { db, event_bus }
    }

    /// Gets a setting value with resolution: ENV > DB > Default.
    pub async fn get(&self, key: &str) -> Result<String> {
        SettingsManager::get(&self.db, key).await
    }

    /// Gets multiple settings values with resolution.
    pub async fn get_many(&self, keys: &[&str]) -> Result<HashMap<String, String>> {
        SettingsManager::get_many(&self.db, keys).await
    }

    /// Sets a setting value in DB with validation and publishes an event.
    pub async fn set(&self, key: &str, value: &str, category: Option<&str>) -> Result<()> {
        SettingsManager::set(&self.db, key, value, category).await?;

        // Publish event
        self.event_bus
            .publish(SystemEvent::System(SystemNotification::SettingChanged {
                key: key.to_string(),
                value: value.to_string(),
            }));

        Ok(())
    }

    /// Gets all public settings for display.
    pub async fn get_all_public(&self) -> Result<Vec<settings::Model>> {
        SettingsManager::get_all_public(&self.db).await
    }
}

// --- Static Settings Manager (Stateless) ---

pub struct SettingsManager;

impl SettingsManager {
    /// Validates a single setting value based on its key.
    pub fn validate(key: &str, value: &str) -> Result<()> {
        if let Some(def) = SETTINGS_REGISTRY.get(key) {
            (def.validator)(value)
        } else {
            Ok(())
        }
    }

    /// Gets a setting value with resolution: ENV > DB > Default.
    pub async fn get<C>(db: &C, key: &str) -> Result<String>
    where
        C: ConnectionTrait,
    {
        let def = SETTINGS_REGISTRY.get(key);

        // 1. Try ENV override
        if let Some(d) = def {
            if let Some(env_var) = d.env_var {
                if let Ok(val) = std::env::var(env_var) {
                    if !val.is_empty() {
                        return Ok(val);
                    }
                }
            }
        }

        // 2. Try DB
        let setting = settings::Entity::find_by_id(key).one(db).await?;
        if let Some(s) = setting {
            if !s.value.is_empty() {
                return Ok(s.value);
            }
        }

        // 3. Try Default
        if let Some(d) = def {
            Ok(d.default_value.clone())
        } else {
            Ok(String::new())
        }
    }

    /// Gets multiple settings values with resolution.
    pub async fn get_many<C>(db: &C, keys: &[&str]) -> Result<HashMap<String, String>>
    where
        C: ConnectionTrait,
    {
        let mut result = HashMap::new();
        let mut db_keys = Vec::new();

        for key in keys {
            let def = SETTINGS_REGISTRY.get(*key);

            // 1. Try ENV
            let mut found_in_env = false;
            if let Some(d) = def {
                if let Some(env_var) = d.env_var {
                    if let Ok(val) = std::env::var(env_var) {
                        if !val.is_empty() {
                            result.insert(key.to_string(), val);
                            found_in_env = true;
                        }
                    }
                }
            }

            if !found_in_env {
                db_keys.push(key.to_string());
            }
        }

        if !db_keys.is_empty() {
            use sea_orm::{ColumnTrait, QueryFilter};
            let found_settings = settings::Entity::find()
                .filter(settings::Column::Key.is_in(db_keys.clone()))
                .all(db)
                .await?;

            for s in found_settings {
                if !s.value.is_empty() {
                    result.insert(s.key, s.value);
                }
            }
        }

        // Fill missing with defaults
        for key in keys {
            if !result.contains_key(*key) {
                let val = if let Some(def) = SETTINGS_REGISTRY.get(*key) {
                    def.default_value.clone()
                } else {
                    String::new()
                };
                result.insert(key.to_string(), val);
            }
        }

        Ok(result)
    }

    /// Sets a setting value in DB with validation.
    pub async fn set<C>(db: &C, key: &str, value: &str, category: Option<&str>) -> Result<()>
    where
        C: ConnectionTrait,
    {
        Self::validate(key, value)?;

        let existing = settings::Entity::find_by_id(key).one(db).await?;

        // Determine category: usage provided, or registry default, or existing, or "System"
        let cat = if let Some(c) = category {
            c.to_string()
        } else if let Some(def) = SETTINGS_REGISTRY.get(key) {
            def.category.to_string()
        } else if let Some(e) = &existing {
            e.category.clone()
        } else {
            "System".to_string()
        };

        if let Some(model) = existing {
            let mut active: settings::ActiveModel = model.into();
            active.value = Set(value.to_string());
            active.category = Set(cat);
            active.update(db).await?;
        } else {
            let active = settings::ActiveModel {
                key: Set(key.to_string()),
                value: Set(value.to_string()),
                category: Set(cat),
                ..Default::default()
            };
            active.insert(db).await?;
        }
        Ok(())
    }

    /// Gets all settings for display (filtering secrets if needed)
    pub async fn get_all_public<C>(db: &C) -> Result<Vec<settings::Model>>
    where
        C: ConnectionTrait,
    {
        let all_settings = settings::Entity::find().all(db).await?;
        let mut result = Vec::new();

        // Use registry to filter secrets
        for s in all_settings {
            if let Some(def) = SETTINGS_REGISTRY.get(s.key.as_str()) {
                if !def.is_secret {
                    result.push(s);
                }
            } else {
                // If not in registry, assume public for backward compatibility or custom settings
                result.push(s);
            }
        }

        Ok(result)
    }
}
