use chrono::Utc;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_common::version::{
    CORE_RELEASE_MANIFEST_FILE, CORE_TRANSACTIONS_DIR, CoreActivationFailure, CoreActivationReason,
    CoreActivationTransaction, CoreReleaseDescriptor, CoreReleaseManifest, PendingCoreActivation,
    read_core_runtime_state, write_core_runtime_state,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CoreReleaseRecord {
    pub version: String,
    pub active: bool,
    pub previous: bool,
    pub launcher_protocol: u32,
    pub api_contract: u32,
    pub schema_epoch: u32,
    pub schema_revision: u32,
    pub target: String,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CoreReleaseList {
    pub launcher_managed: bool,
    pub active_version: String,
    pub previous_version: Option<String>,
    pub pending_version: Option<String>,
    pub last_failure: Option<CoreActivationFailure>,
    pub releases: Vec<CoreReleaseRecord>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ActivateCoreReleaseRequest {
    #[serde(default)]
    pub confirm_data_loss: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CoreReleaseMutation {
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

fn releases_root() -> Result<PathBuf> {
    Ok(system_root()?.join("releases/core"))
}

fn manifest_descriptor(root: &Path) -> Result<CoreReleaseDescriptor> {
    let manifest: CoreReleaseManifest = serde_json::from_slice(
        &fs::read(root.join(CORE_RELEASE_MANIFEST_FILE)).map_err(AppError::Io)?,
    )
    .map_err(AppError::Json)?;
    let binary = root.join("niupanel");
    if !binary.is_file() {
        return Err(AppError::ValidationError(format!(
            "Core {} 缺少 niupanel 可执行文件",
            manifest.version
        )));
    }
    let installed_at = manifest.built_at.clone().unwrap_or_else(|| {
        fs::metadata(root)
            .and_then(|value| value.modified())
            .map(chrono::DateTime::<Utc>::from)
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|_| Utc::now().to_rfc3339())
    });
    Ok(CoreReleaseDescriptor {
        version: manifest.version,
        path: binary.to_string_lossy().into_owned(),
        launcher_protocol: manifest.launcher_protocol,
        api_contract: manifest.api_contract,
        schema_epoch: manifest.schema_epoch,
        schema_revision: manifest.schema_revision,
        target: manifest.target,
        installed_at,
    })
}

fn installed_descriptors() -> Result<Vec<CoreReleaseDescriptor>> {
    let root = releases_root()?;
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut releases = Vec::new();
    for entry in fs::read_dir(root).map_err(AppError::Io)? {
        let entry = entry.map_err(AppError::Io)?;
        if !entry.file_type().map_err(AppError::Io)?.is_dir() {
            continue;
        }
        match manifest_descriptor(&entry.path()) {
            Ok(release) => releases.push(release),
            Err(error) => tracing::warn!(
                path = %entry.path().display(),
                "Skipping invalid Core release: {error}"
            ),
        }
    }
    Ok(releases)
}

pub fn list_releases() -> Result<CoreReleaseList> {
    let root = system_root()?;
    let runtime = read_core_runtime_state(&root).ok();
    let mut descriptors = installed_descriptors()?;
    if let Some(runtime) = &runtime {
        for release in [
            &runtime.active,
            runtime.previous.as_ref().unwrap_or(&runtime.active),
        ] {
            if !descriptors
                .iter()
                .any(|value| value.version == release.version)
            {
                descriptors.push(release.clone());
            }
        }
    }
    let active_version = runtime
        .as_ref()
        .map(|value| value.active.version.clone())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    let previous_version = runtime.as_ref().and_then(|value| {
        value
            .previous
            .as_ref()
            .map(|release| release.version.clone())
    });
    let pending_version = runtime.as_ref().and_then(|value| {
        value
            .pending
            .as_ref()
            .map(|pending| pending.candidate.version.clone())
    });
    let mut releases = descriptors
        .into_iter()
        .map(|release| CoreReleaseRecord {
            active: release.version == active_version,
            previous: previous_version.as_deref() == Some(release.version.as_str()),
            version: release.version,
            launcher_protocol: release.launcher_protocol,
            api_contract: release.api_contract,
            schema_epoch: release.schema_epoch,
            schema_revision: release.schema_revision,
            target: release.target,
            installed_at: release.installed_at,
        })
        .collect::<Vec<_>>();
    releases.sort_by(|left, right| {
        right
            .active
            .cmp(&left.active)
            .then_with(|| right.previous.cmp(&left.previous))
            .then_with(|| right.version.cmp(&left.version))
    });
    Ok(CoreReleaseList {
        launcher_managed: std::env::var("NIUPANEL_LAUNCHED").as_deref() == Ok("1"),
        active_version,
        previous_version,
        pending_version,
        last_failure: runtime.and_then(|value| value.last_failure),
        releases,
    })
}

fn rollback_snapshot_for(target_version: &str) -> Result<Option<PathBuf>> {
    let root = system_root()?;
    let transactions = root.join(CORE_TRANSACTIONS_DIR);
    if !transactions.is_dir() {
        return Ok(None);
    }
    let mut matches = Vec::new();
    for entry in fs::read_dir(transactions).map_err(AppError::Io)? {
        let entry = entry.map_err(AppError::Io)?;
        if !entry.file_type().map_err(AppError::Io)?.is_file() {
            continue;
        }
        let transaction = serde_json::from_slice::<CoreActivationTransaction>(
            &fs::read(entry.path()).map_err(AppError::Io)?,
        );
        let Ok(transaction) = transaction else {
            continue;
        };
        let snapshot = PathBuf::from(&transaction.snapshot_path);
        if transaction.successful && transaction.from.version == target_version && snapshot.is_dir()
        {
            matches.push((transaction.completed_at, snapshot));
        }
    }
    matches.sort_by(|left, right| right.0.cmp(&left.0));
    Ok(matches.into_iter().next().map(|(_, path)| path))
}

pub fn activate_release(version: &str, confirm_data_loss: bool) -> Result<CoreReleaseMutation> {
    if std::env::var("NIUPANEL_LAUNCHED").as_deref() != Ok("1") {
        return Err(AppError::ValidationError(
            "Core 版本切换需要由 niupanel-launcher 启动当前服务".to_string(),
        ));
    }
    let root = system_root()?;
    let mut runtime = read_core_runtime_state(&root).map_err(AppError::Internal)?;
    if runtime.pending.is_some() {
        return Err(AppError::ValidationError(
            "已有等待激活的 Core 版本".to_string(),
        ));
    }
    if runtime.active.version == version {
        return Err(AppError::ValidationError(format!(
            "Core {version} 已经是当前版本"
        )));
    }
    let candidate = installed_descriptors()?
        .into_iter()
        .find(|release| release.version == version)
        .ok_or_else(|| AppError::NotFound(format!("Core {version} 未安装")))?;
    let database_restore_required = candidate.schema_epoch != runtime.active.schema_epoch;
    let restore_before_start = if database_restore_required {
        if !confirm_data_loss {
            return Err(AppError::ValidationError(
                "跨 schema epoch 回退会恢复旧数据库快照并丢弃升级后的数据，需要显式确认"
                    .to_string(),
            ));
        }
        Some(
            rollback_snapshot_for(version)?
                .ok_or_else(|| {
                    AppError::ValidationError(format!("找不到回到 Core {version} 所需的数据库快照"))
                })?
                .to_string_lossy()
                .into_owned(),
        )
    } else {
        None
    };
    let transaction_id = nanoid::nanoid!(16);
    runtime.pending = Some(PendingCoreActivation {
        transaction_id: transaction_id.clone(),
        candidate,
        reason: CoreActivationReason::Rollback,
        requested_at: Utc::now().to_rfc3339(),
        restore_before_start,
    });
    write_core_runtime_state(&root, &runtime).map_err(AppError::Internal)?;
    Ok(CoreReleaseMutation {
        version: version.to_string(),
        transaction_id,
        database_restore_required,
        message: "Core 版本切换已排队，服务将由 launcher 重启并执行健康检查".to_string(),
    })
}
