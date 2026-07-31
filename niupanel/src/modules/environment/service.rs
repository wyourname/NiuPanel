use super::layout::RuntimeKind;
use super::models::{CreateEnvRequest, EnvironmentInfo, InstallPackageRequest};
use super::node_service::NodeEnvironmentService;
use super::python_service::PythonEnvironmentService;
use super::shell_service::ShellEnvironmentService;
use niupanel_common::error::{AppError, Result};
use niupanel_core::environment::{self, InstallRuntimePackagesRequest};
use niupanel_core::settings::SettingsService;
use niupanel_core::task_manager::service::TaskManagerService;
use sea_orm::DatabaseConnection;

pub struct EnvironmentService;

impl EnvironmentService {
    pub async fn list_environments(
        db: &DatabaseConnection,
        settings: &SettingsService,
    ) -> Result<Vec<EnvironmentInfo>> {
        let mut envs = PythonEnvironmentService::list_environments(db).await?;
        envs.extend(NodeEnvironmentService::list_environments(settings).await);
        envs.push(ShellEnvironmentService::environment_info());

        Ok(envs)
    }

    pub async fn list_available_versions(settings: &SettingsService) -> Result<String> {
        let python_versions = PythonEnvironmentService::list_available_versions(settings)
            .await
            .unwrap_or_default();
        let node_catalog = NodeEnvironmentService::available_version_catalog(settings)
            .await
            .unwrap_or_default();

        let combined = serde_json::json!({
            RuntimeKind::Python.env_type(): python_versions,
            RuntimeKind::Node.env_type(): node_catalog.versions,
            "node_recommended_lts": node_catalog.recommended_lts.unwrap_or_default()
        });
        Ok(serde_json::to_string(&combined).unwrap_or_default())
    }

    pub async fn create_python_env(
        db: &DatabaseConnection,
        settings: &SettingsService,
        task_manager: &TaskManagerService,
        payload: CreateEnvRequest,
    ) -> Result<(i32, String)> {
        PythonEnvironmentService::create_env(db, settings, task_manager, payload).await
    }

    pub async fn delete_python_env(db: &DatabaseConnection, name: &str) -> Result<()> {
        PythonEnvironmentService::delete_env(db, name).await
    }

    pub async fn list_python_packages(settings: &SettingsService, name: &str) -> Result<String> {
        PythonEnvironmentService::list_packages(settings, name).await
    }

    pub async fn install_python_packages(
        db: &DatabaseConnection,
        settings: &SettingsService,
        task_manager: &TaskManagerService,
        name: String,
        payload: InstallPackageRequest,
    ) -> Result<i32> {
        environment::install_python_packages(
            db,
            settings,
            task_manager,
            InstallRuntimePackagesRequest {
                env_name: name,
                packages: payload.packages,
            },
        )
        .await
    }

    pub async fn uninstall_python_package(
        db: &DatabaseConnection,
        task_manager: &TaskManagerService,
        name: String,
        package: String,
    ) -> Result<i32> {
        PythonEnvironmentService::uninstall_package(db, task_manager, name, package).await
    }

    pub async fn list_node_packages(settings: &SettingsService, name: &str) -> Result<String> {
        NodeEnvironmentService::list_packages(settings, name).await
    }

    pub async fn create_node_env(
        db: &DatabaseConnection,
        settings: &SettingsService,
        task_manager: &TaskManagerService,
        payload: CreateEnvRequest,
    ) -> Result<i32> {
        NodeEnvironmentService::create_env(db, settings, task_manager, payload).await
    }

    pub async fn delete_node_env(db: &DatabaseConnection, name: &str) -> Result<String> {
        NodeEnvironmentService::delete_env(db, name).await
    }

    pub async fn install_node_packages(
        db: &DatabaseConnection,
        settings: &SettingsService,
        task_manager: &TaskManagerService,
        name: String,
        payload: InstallPackageRequest,
    ) -> Result<i32> {
        environment::install_node_packages(
            db,
            settings,
            task_manager,
            InstallRuntimePackagesRequest {
                env_name: name,
                packages: payload.packages,
            },
        )
        .await
    }

    pub async fn uninstall_node_package(
        db: &DatabaseConnection,
        settings: &SettingsService,
        task_manager: &TaskManagerService,
        name: String,
        package: String,
    ) -> Result<i32> {
        NodeEnvironmentService::uninstall_package(db, settings, task_manager, name, package).await
    }

    pub async fn set_node_default(settings: &SettingsService, name: &str) -> Result<String> {
        NodeEnvironmentService::set_default(settings, name).await
    }

    pub async fn list_shell_packages() -> Result<String> {
        ShellEnvironmentService::list_packages().await
    }

    pub async fn install_shell_packages(
        db: &DatabaseConnection,
        task_manager: &TaskManagerService,
        payload: InstallPackageRequest,
    ) -> Result<i32> {
        ShellEnvironmentService::install_packages(db, task_manager, payload).await
    }

    pub async fn uninstall_shell_package(db: &DatabaseConnection, package: &str) -> Result<()> {
        ShellEnvironmentService::uninstall_package(db, package).await
    }

    pub async fn set_mirror_source(env_type: &str, mirror_url: &str) -> Result<()> {
        match RuntimeKind::from_env_type(env_type) {
            Some(RuntimeKind::Python) => {
                PythonEnvironmentService::set_mirror_source(mirror_url).await?;
            }
            Some(RuntimeKind::Node) => {
                NodeEnvironmentService::set_mirror_source(mirror_url).await?;
            }
            Some(RuntimeKind::Shell) | None => {
                return Err(AppError::Generic(
                    "Unsupported environment type for mirror setting".to_string(),
                ));
            }
        }

        Ok(())
    }
}
