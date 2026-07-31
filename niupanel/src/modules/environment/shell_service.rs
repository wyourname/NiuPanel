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
        let packages = payload.packages.clone();
        let shell = RuntimeKind::Shell;
        let name = RuntimeKind::shell_name().to_string();
        let existing_env = environments::Entity::find()
            .filter(environments::Column::Name.eq(&name))
            .one(db)
            .await?;

        requirements::merge_or_insert_requirements(
            db,
            existing_env,
            &name,
            shell.env_type(),
            &shell.record_version(&name),
            &packages,
        )
        .await?;

        task_manager
            .submit_system_task("Install Shell packages".to_string(), move |tx| async move {
                let mut cmd = Command::new("apt-get");
                cmd.arg("install").arg("-y");
                cmd.env("DEBIAN_FRONTEND", "noninteractive");
                for package in packages {
                    cmd.arg(package);
                }

                let desc = "apt-get install";
                cmd.execute_with_streaming(None, desc, tx.clone()).await?;
                let _ = tx.send("Installation completed successfully.".to_string());
                Ok(())
            })
            .await
    }

    pub(super) async fn uninstall_package(db: &DatabaseConnection, package: &str) -> Result<()> {
        let mut cmd = Command::new("apt-get");
        cmd.arg("remove").arg("-y").arg(package);
        cmd.execute_checked("apt-get remove").await?;

        let name = RuntimeKind::shell_name().to_string();
        let existing_env = environments::Entity::find()
            .filter(environments::Column::Name.eq(&name))
            .one(db)
            .await?;

        if let Some(env_model) = existing_env {
            requirements::remove_requirement(db, env_model, package, SHELL_PACKAGE_SEPARATORS)
                .await?;
        }
        Ok(())
    }
}
