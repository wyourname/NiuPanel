use super::layout::RuntimeKind;
use super::models::{EnvironmentInfo, InstallPackageRequest};
use super::requirements::{self, SHELL_PACKAGE_SEPARATORS};
use niupanel_common::error::{AppError, Result};
use niupanel_core::sys::command::CommandExt;
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::environments;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashSet;
use tokio::process::Command;
use tokio::sync::Mutex;

static APT_LOCK: Mutex<()> = Mutex::const_new(());

pub(super) struct ShellEnvironmentService;

impl ShellEnvironmentService {
    pub(super) fn environment_info() -> EnvironmentInfo {
        EnvironmentInfo {
            name: RuntimeKind::shell_name().to_string(),
            path: RuntimeKind::Shell.missing_path_label().to_string(),
            version: None,
            env_type: RuntimeKind::Shell.env_type().to_string(),
            is_installed: true,
            recorded_packages: None,
        }
    }

    pub(super) async fn list_packages() -> Result<String> {
        let mut cmd_manual = Command::new("apt-mark");
        cmd_manual.arg("showmanual");
        let output_manual = cmd_manual
            .output()
            .await
            .map_err(|e| AppError::ProcessStart {
                command: "apt-mark showmanual".to_string(),
                source: e,
            })?;
        if !output_manual.status.success() {
            let stderr = String::from_utf8_lossy(&output_manual.stderr).to_string();
            return Err(AppError::ProcessFailed {
                command: "apt-mark showmanual".to_string(),
                exit_code: output_manual.status.code(),
                stderr,
            });
        }

        let manual_stdout = String::from_utf8_lossy(&output_manual.stdout);
        let manual_set: HashSet<&str> = manual_stdout
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();

        let mut cmd_versions = Command::new("dpkg-query");
        cmd_versions.arg("-W").arg("-f=${Package} ${Version}\n");
        let output_versions = cmd_versions
            .output()
            .await
            .map_err(|e| AppError::ProcessStart {
                command: "dpkg-query -W".to_string(),
                source: e,
            })?;
        if !output_versions.status.success() {
            let stderr = String::from_utf8_lossy(&output_versions.stderr).to_string();
            return Err(AppError::ProcessFailed {
                command: "dpkg-query -W".to_string(),
                exit_code: output_versions.status.code(),
                stderr,
            });
        }

        let versions_stdout = String::from_utf8_lossy(&output_versions.stdout);
        let packages: Vec<serde_json::Value> = versions_stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let name = parts[0].trim();
                    let version = parts[1].trim();
                    if manual_set.contains(name) {
                        return Some(serde_json::json!({ "name": name, "version": version }));
                    }
                }
                None
            })
            .collect();

        serde_json::to_string(&packages)
            .map_err(|e| AppError::Generic(format!("Failed to serialize packages: {}", e)))
    }

    pub(super) async fn install_packages(
        db: &DatabaseConnection,
        task_manager: &TaskManagerService,
        payload: InstallPackageRequest,
    ) -> Result<i32> {
        let packages = payload.packages;
        let db = db.clone();
        task_manager
            .submit_system_task_with_metadata("安装 Linux 依赖".to_string(), Some(serde_json::json!({"kind": "environment-packages", "env_type": RuntimeKind::Shell.env_type(), "env_name": RuntimeKind::shell_name(), "operation": "install", "packages": packages})), move |tx| async move {
                let _guard = APT_LOCK.lock().await;
                let _ = tx.send("正在更新 Linux 软件包索引…".to_owned());
                Command::new("apt-get")
                    .args(["-o", "DPkg::Lock::Timeout=120", "update", "--error-on=any"])
                    .execute_with_streaming(None, "apt-get update", tx.clone())
                    .await?;
                Command::new("apt-get")
                    .args(["-o", "DPkg::Lock::Timeout=120", "install", "-y", "--"])
                    .args(&packages)
                    .env("DEBIAN_FRONTEND", "noninteractive")
                    .execute_with_streaming(None, "apt-get install", tx.clone())
                    .await?;
                let name = RuntimeKind::shell_name();
                let existing_env = environments::Entity::find()
                    .filter(environments::Column::Name.eq(name))
                    .one(&db)
                    .await?;
                requirements::merge_or_insert_requirements(
                    &db,
                    existing_env,
                    name,
                    RuntimeKind::Shell.env_type(),
                    &RuntimeKind::Shell.record_version(name),
                    &packages,
                )
                .await?;
                let _ = tx.send("Linux 依赖安装完成。".to_owned());
                Ok(())
            })
            .await
    }

    pub(super) async fn uninstall_package(
        db: &DatabaseConnection,
        task_manager: &TaskManagerService,
        package: &str,
    ) -> Result<i32> {
        let db = db.clone();
        let package = package.to_owned();
        task_manager
            .submit_system_task_with_metadata(format!("卸载 Linux 依赖 {package}"), Some(serde_json::json!({"kind": "environment-packages", "env_type": RuntimeKind::Shell.env_type(), "env_name": RuntimeKind::shell_name(), "operation": "uninstall", "packages": [package]})), move |tx| async move {
                let _guard = APT_LOCK.lock().await;
                Command::new("apt-get")
                    .args(["-o", "DPkg::Lock::Timeout=120", "remove", "-y", "--"])
                    .arg(&package)
                    .env("DEBIAN_FRONTEND", "noninteractive")
                    .execute_with_streaming(None, "apt-get remove", tx.clone())
                    .await?;
                if let Some(env_model) = environments::Entity::find()
                    .filter(environments::Column::Name.eq(RuntimeKind::shell_name()))
                    .one(&db)
                    .await?
                {
                    requirements::remove_requirement(
                        &db,
                        env_model,
                        &package,
                        SHELL_PACKAGE_SEPARATORS,
                    )
                    .await?;
                }
                let _ = tx.send("Linux 依赖卸载完成。".to_owned());
                Ok(())
            })
            .await
    }
}
