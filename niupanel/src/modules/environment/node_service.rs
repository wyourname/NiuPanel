use super::layout::RuntimeKind;
use super::mirrors;
use super::models::{CreateEnvRequest, EnvironmentInfo};
use super::requirements::{self, NODE_PACKAGE_SEPARATORS};
use niupanel_common::constants::settings::{NODE_DEFAULT_PACKAGES, SYSTEM_DEFAULT_NODE_VERSION};
use niupanel_common::error::{AppError, Result};
use niupanel_core::runtime::RuntimeManager;
use niupanel_core::script::interpreter::node::NodeVersionCatalog;
use niupanel_core::settings::SettingsService;
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::environments;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

pub(super) struct NodeEnvironmentService;

impl NodeEnvironmentService {
    pub(super) async fn list_environments(
        db: &DatabaseConnection,
        settings: &SettingsService,
    ) -> Result<Vec<EnvironmentInfo>> {
        let mut envs = Vec::new();
        let node = RuntimeKind::Node;
        let configured_default = settings
            .get(SYSTEM_DEFAULT_NODE_VERSION)
            .await
            .ok()
            .and_then(|version| {
                niupanel_core::script::interpreter::node::NodeEnvironment::normalize_version(
                    &version,
                )
            });

        if let Ok(local_versions) = RuntimeManager::list_local_node_versions().await {
            for (version, is_runtime_default) in local_versions {
                let is_default = configured_default
                    .as_ref()
                    .map(|default| default == &version)
                    .unwrap_or(is_runtime_default);
                envs.push(EnvironmentInfo {
                    name: version.clone(),
                    path: RuntimeKind::node_path_label(is_default).to_string(),
                    version: Some(version),
                    env_type: node.env_type().to_string(),
                    is_installed: true,
                    recorded_packages: None,
                });
            }
        }

        Self::append_recorded_environments(db, &mut envs).await?;
        Ok(envs)
    }

    async fn append_recorded_environments(
        db: &DatabaseConnection,
        envs: &mut Vec<EnvironmentInfo>,
    ) -> Result<()> {
        let node = RuntimeKind::Node;
        for recorded in environments::Entity::find()
            .filter(environments::Column::EnvType.eq(node.env_type()))
            .all(db)
            .await?
        {
            let version = normalize_node_version(&recorded.version)?;
            let recorded_packages = recorded
                .requirements
                .as_deref()
                .map(|value| value.lines().filter(|line| !line.trim().is_empty()).count());
            if let Some(installed) = envs.iter_mut().find(|env| env.name == version) {
                installed.recorded_packages = recorded_packages;
            } else {
                envs.push(EnvironmentInfo {
                    name: version.clone(),
                    version: Some(version),
                    path: node.missing_path_label().to_owned(),
                    env_type: node.env_type().to_owned(),
                    is_installed: false,
                    recorded_packages,
                });
            }
        }
        Ok(())
    }

    pub(super) async fn available_version_catalog(
        settings: &SettingsService,
    ) -> Result<NodeVersionCatalog> {
        RuntimeManager::available_node_version_catalog(Some(
            mirrors::node_distribution(settings).await,
        ))
        .await
    }

    pub(super) async fn list_packages(settings: &SettingsService, name: &str) -> Result<String> {
        let version = normalize_node_version(name)?;
        let env = RuntimeManager::open_node_environment(
            Some(&version),
            Some(mirrors::node_install(settings).await),
        )?;
        env.list_packages().await
    }

    pub(super) async fn create_env(
        db: &DatabaseConnection,
        settings: &SettingsService,
        task_manager: &TaskManagerService,
        payload: CreateEnvRequest,
    ) -> Result<i32> {
        let mirrors = mirrors::node_install(settings).await;
        let version = payload.version.clone();
        let existing_env = environments::Entity::find()
            .filter(environments::Column::Name.eq(&version))
            .filter(environments::Column::EnvType.eq(RuntimeKind::Node.env_type()))
            .one(db)
            .await?;

        if existing_env.is_none() {
            let active = environments::ActiveModel {
                name: Set(version.clone()),
                env_type: Set(RuntimeKind::Node.env_type().to_string()),
                version: Set(version.clone()),
                ..Default::default()
            };
            active.insert(db).await?;
        }
        let restored_requirements = existing_env.and_then(|model| model.requirements);
        let default_packages = settings
            .get(NODE_DEFAULT_PACKAGES)
            .await
            .unwrap_or_default()
            .replace(',', "\n");

        let version_clone = version.clone();
        task_manager
            .submit_system_task(format!("Install Node {}", version), move |tx| async move {
                RuntimeManager::create_node_environment(
                    version_clone.clone(),
                    tx.clone(),
                    Some(mirrors.clone()),
                )
                .await?;

                let mut requirements_to_install = default_packages;
                if let Some(restored) = restored_requirements {
                    if !restored.trim().is_empty() {
                        let _ = tx.send(format!(
                            "[System] Found recorded dependencies, restoring: \n{}",
                            restored
                        ));
                        requirements_to_install =
                            format!("{}\n{}", requirements_to_install, restored);
                    }
                }

                let requirements: Vec<String> = requirements_to_install
                    .lines()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect();

                if !requirements.is_empty() {
                    let env = RuntimeManager::open_node_environment(
                        Some(&version_clone),
                        Some(mirrors.clone()),
                    )?;
                    env.install_packages(&requirements, tx.clone()).await?;
                }
                Ok(())
            })
            .await
    }

    pub(super) async fn delete_env(db: &DatabaseConnection, name: &str) -> Result<String> {
        let version = RuntimeKind::Node.record_version(name);

        RuntimeManager::uninstall_node_version(&version).await?;
        environments::Entity::delete_many()
            .filter(environments::Column::Name.eq(name))
            .filter(environments::Column::EnvType.eq(RuntimeKind::Node.env_type()))
            .exec(db)
            .await?;
        Ok(version)
    }

    pub(super) async fn uninstall_package(
        db: &DatabaseConnection,
        settings: &SettingsService,
        task_manager: &TaskManagerService,
        name: String,
        package: String,
    ) -> Result<i32> {
        let db = db.clone();
        let package_clone = package.clone();
        let version = normalize_node_version(&name)?;
        let mirrors = mirrors::node_install(settings).await;
        task_manager
            .submit_system_task_with_metadata(
                format!("Uninstall {}", package_clone),
                Some(serde_json::json!({"kind": "environment-packages", "env_type": "node", "env_name": name, "operation": "uninstall", "packages": [package]})),
                move |tx| async move {
                    let env = RuntimeManager::open_node_environment(Some(&version), Some(mirrors))?;
                    env.uninstall_package(&package_clone, tx.clone()).await?;
                    if let Some(env_model) = environments::Entity::find()
                        .filter(environments::Column::Name.eq(&name))
                        .filter(environments::Column::EnvType.eq(RuntimeKind::Node.env_type()))
                        .one(&db)
                        .await?
                    {
                        requirements::remove_requirement(
                            &db,
                            env_model,
                            &package_clone,
                            NODE_PACKAGE_SEPARATORS,
                        )
                        .await?;
                    }
                    let _ = tx.send("Uninstallation completed successfully.".to_string());
                    Ok(())
                },
            )
            .await
    }

    pub(super) async fn set_default(settings: &SettingsService, name: &str) -> Result<String> {
        let version = RuntimeKind::Node.record_version(name);

        RuntimeManager::set_node_default(&version).await?;
        settings
            .set(SYSTEM_DEFAULT_NODE_VERSION, &version, Some("System"))
            .await?;
        Ok(version)
    }
}

fn normalize_node_version(name: &str) -> Result<String> {
    RuntimeKind::Node
        .version_from_name(name)
        .and_then(|version| {
            niupanel_core::script::interpreter::node::NodeEnvironment::normalize_version(&version)
        })
        .ok_or_else(|| AppError::ValidationError("Node version must be specified".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Database, Schema};

    #[tokio::test]
    async fn missing_node_runtime_remains_visible_for_restore() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        for statement in [
            schema.create_table_from_entity(environments::Entity),
            schema.create_table_from_entity(niupanel_entity::settings::Entity),
        ] {
            db.execute_raw(backend.build(&statement)).await.unwrap();
        }
        environments::ActiveModel {
            name: Set("987.6.5".to_owned()),
            version: Set("987.6.5".to_owned()),
            env_type: Set("node".to_owned()),
            requirements: Set(Some("axios\n\n@types/node".to_owned())),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let mut result = Vec::new();
        NodeEnvironmentService::append_recorded_environments(&db, &mut result)
            .await
            .unwrap();
        let missing = result.iter().find(|env| env.name == "987.6.5").unwrap();
        assert!(!missing.is_installed);
        assert_eq!(missing.recorded_packages, Some(2));
    }
}
