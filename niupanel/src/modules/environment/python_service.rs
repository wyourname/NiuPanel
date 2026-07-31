use super::layout::RuntimeKind;
use super::mirrors;
use super::models::{CreateEnvRequest, EnvironmentInfo};
use super::requirements::{self, VERSIONED_PACKAGE_SEPARATORS};
use niupanel_common::error::{AppError, Result};
use niupanel_core::runtime::RuntimeManager;
use niupanel_core::settings::SettingsService;
use niupanel_core::sys::command::CommandExt;
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::environments;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashSet;
use tokio::fs;
use tokio::process::Command;

pub(super) struct PythonEnvironmentService;

impl PythonEnvironmentService {
    pub(super) async fn list_environments(db: &DatabaseConnection) -> Result<Vec<EnvironmentInfo>> {
        let python = RuntimeKind::Python;
        let mut envs = Vec::new();
        let mut physical_names = HashSet::new();

        let base_path = python.runtime_dir();
        if base_path.exists() {
            let mut entries = fs::read_dir(base_path).await.map_err(AppError::Io)?;
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(file_type) = entry.file_type().await {
                    if file_type.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if python.is_managed_name(&name) {
                            let version = python.version_from_name(&name);
                            physical_names.insert(name.clone());
                            envs.push(EnvironmentInfo {
                                name: name.clone(),
                                path: entry.path().to_string_lossy().to_string(),
                                version,
                                env_type: python.env_type().to_string(),
                                is_installed: true,
                                recorded_packages: None,
                            });
                        }
                    }
                }
            }
        }

        let recorded_envs = environments::Entity::find()
            .filter(environments::Column::EnvType.eq(python.env_type()))
            .all(db)
            .await?;

        for rec in recorded_envs {
            if !physical_names.contains(&rec.name) {
                envs.push(EnvironmentInfo {
                    name: rec.name.clone(),
                    path: python.missing_path_label().to_string(),
                    version: Some(rec.version),
                    env_type: python.env_type().to_string(),
                    is_installed: false,
                    recorded_packages: rec.requirements.map(|s| s.lines().count()),
                });
            } else if let Some(env) = envs.iter_mut().find(|env| env.name == rec.name) {
                env.recorded_packages = rec.requirements.map(|s| s.lines().count());
            }
        }

        Ok(envs)
    }

    pub(super) async fn list_available_versions(settings: &SettingsService) -> Result<String> {
        RuntimeManager::list_available_python_versions(Some(
            mirrors::python_install(settings).await,
        ))
        .await
    }

    pub(super) async fn create_env(
        db: &DatabaseConnection,
        settings: &SettingsService,
        task_manager: &TaskManagerService,
        payload: CreateEnvRequest,
    ) -> Result<(i32, String)> {
        let python = RuntimeKind::Python;
        let venv_name = python.env_name(&payload.version);
        let venv_path = python.env_path(&venv_name);

        if venv_path.exists() {
            return Err(AppError::Generic("Environment already exists".to_string()));
        }

        let recorded_env = environments::Entity::find()
            .filter(environments::Column::Name.eq(&venv_name))
            .one(db)
            .await?;
        let restored_requirements = recorded_env.and_then(|model| model.requirements);

        let mirrors = mirrors::python_install_and_index(settings).await;
        let default_packages = settings
            .get("system.python_default_packages")
            .await
            .unwrap_or_default()
            .replace(',', "\n");

        let version_clone = payload.version.clone();
        let job_id = task_manager
            .submit_system_task(
                format!("Create Python Env {}", payload.version),
                move |tx| async move {
                    let python = RuntimeKind::Python;
                    let venv_path = python.env_path(&python.env_name(&version_clone));
                    let mirrors_install = mirrors.clone();
                    RuntimeManager::create_python_environment(
                        venv_path.clone(),
                        Some(&version_clone),
                        tx.clone(),
                        Some(mirrors),
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

                    let env = RuntimeManager::open_python_environment(
                        venv_path,
                        None,
                        Some(mirrors_install),
                    )
                    .await?;
                    env.install_requirements(&requirements_to_install, false, tx.clone())
                        .await?;
                    let _ = tx.send(
                        "Installation and packages setup completed successfully.".to_string(),
                    );
                    Ok(())
                },
            )
            .await?;

        Ok((job_id, venv_name))
    }

    pub(super) async fn delete_env(db: &DatabaseConnection, name: &str) -> Result<()> {
        let venv_path = RuntimeKind::Python.env_path(name);
        if venv_path.exists() {
            fs::remove_dir_all(venv_path).await.map_err(AppError::Io)?;
        }

        environments::Entity::delete_many()
            .filter(environments::Column::Name.eq(name))
            .exec(db)
            .await?;
        Ok(())
    }

    pub(super) async fn list_packages(settings: &SettingsService, name: &str) -> Result<String> {
        let venv_path = RuntimeKind::Python.env_path(name);
        if !venv_path.exists() {
            return Err(AppError::NotFound("Environment not found".to_string()));
        }

        let mirrors = mirrors::python_index(settings).await;
        let env = RuntimeManager::open_python_environment(venv_path, None, Some(mirrors)).await?;
        env.list_packages().await
    }

    pub(super) async fn uninstall_package(
        db: &DatabaseConnection,
        task_manager: &TaskManagerService,
        name: String,
        package: String,
    ) -> Result<i32> {
        let venv_path = RuntimeKind::Python.env_path(&name);
        if !venv_path.exists() {
            return Err(AppError::NotFound("Environment not found".to_string()));
        }

        let existing_env = environments::Entity::find()
            .filter(environments::Column::Name.eq(&name))
            .one(db)
            .await?;

        if let Some(env_model) = existing_env {
            requirements::remove_requirement(db, env_model, &package, VERSIONED_PACKAGE_SEPARATORS)
                .await?;
        }

        let env = RuntimeManager::open_python_environment(venv_path, None, None).await?;
        let name_clone = name.clone();
        let package_clone = package.clone();
        task_manager
            .submit_system_task(
                format!("Uninstall {} from {}", package_clone, name_clone),
                move |tx| async move {
                    env.uninstall_package(&package_clone, tx.clone()).await?;
                    let _ = tx.send("Uninstallation completed successfully.".to_string());
                    Ok(())
                },
            )
            .await
    }

    pub(super) async fn set_mirror_source(mirror_url: &str) -> Result<()> {
        let mut cmd = Command::new("pip");
        cmd.arg("config")
            .arg("set")
            .arg("global.index-url")
            .arg(mirror_url);
        cmd.execute_checked("pip config set mirror").await?;
        Ok(())
    }
}
