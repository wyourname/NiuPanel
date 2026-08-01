use chrono::Utc;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_common::panel_runtime::{
    PanelActivationFailure, PendingPanelActivation, enqueue_panel_activation,
    latest_snapshot_for_release, list_panel_releases, list_panel_transactions,
    read_panel_runtime_state, verify_panel_release,
};
use niupanel_common::version::PanelActivationReason;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PanelReleaseRecord {
    pub version: String,
    pub active: bool,
    pub previous: bool,
    pub rollback_available: bool,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PanelReleaseList {
    pub launcher_managed: bool,
    pub active_version: String,
    pub previous_version: Option<String>,
    pub pending_version: Option<String>,
    pub last_failure: Option<PanelActivationFailure>,
    pub releases: Vec<PanelReleaseRecord>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RollbackPanelReleaseRequest {
    pub confirm_data_loss: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PanelReleaseMutation {
    pub version: String,
    pub transaction_id: String,
    pub database_restore_required: bool,
    pub message: String,
}

fn system_root() -> Result<PathBuf> {
    let path = &Config::global().system_dir;
    if path.is_absolute() {
        Ok(path.clone())
    } else {
        Ok(std::env::current_dir().map_err(AppError::Io)?.join(path))
    }
}

fn release_files_present(release: &niupanel_common::panel_runtime::PanelReleaseDescriptor) -> bool {
    Path::new(&release.core.path).is_file()
        && Path::new(&release.web_path).join("index.html").is_file()
}

fn release_is_activatable(
    release: &niupanel_common::panel_runtime::PanelReleaseDescriptor,
) -> bool {
    release_files_present(release) && verify_panel_release(release).is_ok()
}

pub async fn list_releases() -> Result<PanelReleaseList> {
    let root = system_root()?;
    let runtime = read_panel_runtime_state(&root)
        .await
        .map_err(AppError::Internal)?;
    let active_version = runtime.active.version.clone();
    let previous_version = runtime
        .previous
        .as_ref()
        .map(|release| release.version.clone());
    let pending_version = runtime
        .pending
        .as_ref()
        .map(|pending| pending.candidate.version.clone());
    let rollback_versions = list_panel_transactions(&root)
        .await
        .map_err(AppError::Internal)?
        .into_iter()
        .filter(|transaction| transaction.successful)
        .filter(|transaction| Path::new(&transaction.snapshot_path).is_dir())
        .map(|transaction| transaction.from_version)
        .collect::<std::collections::HashSet<_>>();
    let mut releases = list_panel_releases(&root)
        .await
        .map_err(AppError::Internal)?
        .into_iter()
        .filter(release_files_present)
        .map(|release| PanelReleaseRecord {
            active: release.version == active_version,
            previous: previous_version.as_deref() == Some(release.version.as_str()),
            rollback_available: rollback_versions.contains(&release.version),
            version: release.version,
            installed_at: release.installed_at,
        })
        .collect::<Vec<_>>();
    releases.sort_by(|left, right| {
        right
            .active
            .cmp(&left.active)
            .then_with(|| right.previous.cmp(&left.previous))
            .then_with(|| right.installed_at.cmp(&left.installed_at))
    });
    Ok(PanelReleaseList {
        launcher_managed: std::env::var("NIUPANEL_LAUNCHED").as_deref() == Ok("1"),
        active_version,
        previous_version,
        pending_version,
        last_failure: runtime.last_failure,
        releases,
    })
}

async fn rollback_snapshot_for(root: &Path, target_version: &str) -> Result<PathBuf> {
    latest_snapshot_for_release(root, target_version)
        .await
        .map_err(AppError::Internal)?
        .ok_or_else(|| {
            AppError::ValidationError(format!(
                "找不到回退到 Panel {target_version} 所需的数据库快照"
            ))
        })
}

pub async fn rollback_release(
    version: &str,
    confirm_data_loss: bool,
) -> Result<PanelReleaseMutation> {
    if std::env::var("NIUPANEL_LAUNCHED").as_deref() != Ok("1") {
        return Err(AppError::ValidationError(
            "Panel 回退需要由 niupanel-launcher 启动当前服务".to_string(),
        ));
    }
    if !confirm_data_loss {
        return Err(AppError::ValidationError(
            "回退会同时恢复该版本的数据库快照并丢弃之后的数据，需要显式确认".to_string(),
        ));
    }
    let root = system_root()?;
    let runtime = read_panel_runtime_state(&root)
        .await
        .map_err(AppError::Internal)?;
    if runtime.pending.is_some() {
        return Err(AppError::ValidationError(
            "已有等待激活的 Panel 版本".to_string(),
        ));
    }
    if runtime.active.version == version {
        return Err(AppError::ValidationError(format!(
            "Panel {version} 已经是当前版本"
        )));
    }
    let expected_active_version = runtime.active.version.clone();
    let candidate = list_panel_releases(&root)
        .await
        .map_err(AppError::Internal)?
        .into_iter()
        .find(|release| release.version == version)
        .filter(release_is_activatable)
        .ok_or_else(|| AppError::NotFound(format!("Panel {version} 未安装或文件不完整")))?;
    let snapshot = rollback_snapshot_for(&root, version).await?;
    let transaction_id = nanoid::nanoid!(16);
    let pending = PendingPanelActivation {
        transaction_id: transaction_id.clone(),
        candidate,
        reason: PanelActivationReason::Rollback,
        requested_at: Utc::now().to_rfc3339(),
        activation_snapshot_path: None,
        restore_before_start: Some(snapshot.to_string_lossy().into_owned()),
    };
    enqueue_panel_activation(&root, &expected_active_version, pending)
        .await
        .map_err(AppError::ValidationError)?;
    Ok(PanelReleaseMutation {
        version: version.to_string(),
        transaction_id,
        database_restore_required: true,
        message: "Panel 回退已排队，Launcher 将恢复数据库并执行健康检查".to_string(),
    })
}
