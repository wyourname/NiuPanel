use super::layout::RuntimeKind;
use super::models::{CreateEnvRequest, EnvironmentInfo, InstallPackageRequest};
use super::node_service::NodeEnvironmentService;
use super::python_service::PythonEnvironmentService;
use super::shell_service::ShellEnvironmentService;
use niupanel_common::error::Result;
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
        envs.extend(NodeEnvironmentService::list_environments(db, settings).await?);
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
        let payload = CreateEnvRequest {
            version: payload.normalized_version("python")?,
        };
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
        payload.validate()?;
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
        let payload = CreateEnvRequest {
            version: payload.normalized_version("node")?,
        };
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
        payload.validate()?;
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
        payload.validate()?;
        ShellEnvironmentService::install_packages(db, task_manager, payload).await
    }

    pub async fn uninstall_shell_package(
        db: &DatabaseConnection,
        task_manager: &TaskManagerService,
        package: &str,
    ) -> Result<i32> {
        ShellEnvironmentService::uninstall_package(db, task_manager, package).await
    }

    pub async fn set_mirror_source(
        settings: &SettingsService,
        env_type: &str,
        mirror_url: &str,
    ) -> Result<()> {
        super::mirror_settings::MirrorSettings::save_package_source(settings, env_type, mirror_url)
            .await
    }
}
