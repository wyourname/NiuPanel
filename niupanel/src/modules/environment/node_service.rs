use super::layout::RuntimeKind;
use super::mirrors;
use super::models::{CreateEnvRequest, EnvironmentInfo};
use super::requirements::{self, VERSIONED_PACKAGE_SEPARATORS};
use niupanel_common::constants::settings::{NODE_DEFAULT_PACKAGES, SYSTEM_DEFAULT_NODE_VERSION};
use niupanel_common::error::{AppError, Result};
use niupanel_core::runtime::RuntimeManager;
use niupanel_core::settings::SettingsService;
use niupanel_core::sys::command::CommandExt;
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::environments;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use tokio::process::Command;

pub(super) struct NodeEnvironmentService;

impl NodeEnvironmentService {
    pub(super) async fn list_environments(settings: &SettingsService) -> Vec<EnvironmentInfo> {
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
            for (version, is_fnm_default) in local_versions {
                let is_default = configured_default
                    .as_ref()
                    .map(|default| default == &version)
                    .unwrap_or(is_fnm_default);
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

        envs
    }

    pub(super) async fn list_available_versions() -> Result<String> {
        RuntimeManager::list_available_node_versions().await
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
            let _ = active.insert(db).await;
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
                match RuntimeManager::create_node_environment(
                    version_clone.clone(),
                    tx.clone(),
                    Some(mirrors.clone()),
                )
                .await
                {
                    Ok(_) => {
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
                            match RuntimeManager::open_node_environment(
                                Some(&version_clone),
                                Some(mirrors.clone()),
                            ) {
                                Ok(env) => {
                                    if let Err(e) =
                                        env.install_packages(&requirements, tx.clone()).await
                                    {
                                        let _ = tx.send(format!(
                                            "Node version installed but failed to install packages: {}",
                                            e
                                        ));
                                    }
                                }
                                Err(e) => {
                                    let _ =
                                        tx.send(format!("Failed to load Node environment: {}", e));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(format!("Error creating Node version: {}", e));
                    }
                }
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
        let existing_env = environments::Entity::find()
            .filter(environments::Column::Name.eq(&name))
            .filter(environments::Column::EnvType.eq(RuntimeKind::Node.env_type()))
            .one(db)
            .await?;

        if let Some(env_model) = existing_env {
            let _ = requirements::remove_requirement(
                db,
                env_model,
                &package,
                VERSIONED_PACKAGE_SEPARATORS,
            )
            .await;
        }

        let package_clone = package.clone();
        let version = normalize_node_version(&name)?;
        let mirrors = mirrors::node_install(settings).await;
        task_manager
            .submit_system_task(
                format!("Uninstall {}", package_clone),
                move |tx| async move {
                    let env = match RuntimeManager::open_node_environment(
                        Some(&version),
                        Some(mirrors),
                    ) {
                        Ok(env) => env,
                        Err(e) => {
                            let _ = tx.send(format!("Error loading Node environment: {}", e));
                            return;
                        }
                    };
                    match env.uninstall_package(&package_clone, tx.clone()).await {
                        Ok(_) => {
                            let _ = tx.send("Uninstallation completed successfully.".to_string());
                        }
                        Err(e) => {
                            let _ = tx.send(format!("Error uninstalling package: {}", e));
                        }
                    }
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

    pub(super) async fn set_mirror_source(mirror_url: &str) -> Result<()> {
        let mut cmd = Command::new("npm");
        cmd.arg("config").arg("set").arg("registry").arg(mirror_url);
        cmd.execute_checked("npm config set registry").await?;
        Ok(())
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
