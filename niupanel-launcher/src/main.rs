use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use niupanel_common::panel_runtime::{
    PanelActivationFailure, PanelActivationTransaction, PanelReleaseDescriptor, PanelRuntimeState,
    PendingPanelActivation, commit_panel_activation, directory_digest, initialize_runtime,
    latest_snapshot_for_release, list_panel_releases, read_panel_runtime_state,
    runtime_database_path, verify_panel_release, write_panel_runtime_state,
};
use niupanel_common::version::{
    API_CONTRACT_VERSION, CORE_RELEASE_MANIFEST_FILE, CoreReleaseDescriptor, CoreReleaseManifest,
    RELEASE_PROTOCOL_VERSION, SCHEMA_EPOCH, SCHEMA_REVISION, WEB_RELEASE_MANIFEST_FILE,
    WebReleaseManifest, read_core_release_manifest, read_web_release_manifest,
};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};

const HEALTH_TIMEOUT: Duration = Duration::from_secs(45);
const ACTIVATION_PROBATION: Duration = Duration::from_secs(15);
const RESTART_DELAY: Duration = Duration::from_secs(2);

mod config;
mod database;
mod process;
mod releases;

use config::*;
use database::*;
use process::*;
use releases::*;

#[tokio::main]
async fn main() -> Result<()> {
    let config = LauncherConfig::from_env()?;
    fs::create_dir_all(&config.system_root).with_context(|| {
        format!(
            "Failed to create system directory {}",
            config.system_root.display()
        )
    })?;

    let mut state = ensure_panel_runtime(&config).await?;
    recover_missing_active(&config, &mut state).await?;

    install_shutdown_watcher(config.shutdown.clone());
    let mut bundled_reconciled = false;

    loop {
        if config.shutdown.load(Ordering::Relaxed) {
            return Ok(());
        }
        state = read_panel_runtime_state(&config.system_root)
            .await
            .map_err(|error| anyhow!(error))?;
        recover_missing_active(&config, &mut state).await?;
        if !bundled_reconciled && state.pending.is_none() {
            reconcile_bundled_release(&config, &mut state).await?;
            bundled_reconciled = true;
        }
        if let Some(pending) = state.pending.clone() {
            match activate_candidate(&config, &mut state, &pending).await? {
                Some(mut child) => {
                    wait_for_active_child(&config, &mut child).await?;
                }
                None => {
                    if config.shutdown.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                }
            }
            continue;
        }

        let mut child = spawn_core(&config, &state.active)?;
        wait_for_active_child(&config, &mut child).await?;
        if config.shutdown.load(Ordering::Relaxed) {
            return Ok(());
        }
        tokio::time::sleep(RESTART_DELAY).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use niupanel_common::version::PanelActivationReason;

    fn test_config(root: &Path, database_path: PathBuf) -> LauncherConfig {
        LauncherConfig {
            system_root: root.join("system"),
            bundled_binary: root.join("niupanel"),
            bundled_manifest_root: root.to_path_buf(),
            bundled_web_dir: root.join("web"),
            bootstrap_panel_version: None,
            bootstrap_core_version: None,
            bootstrap_web_version: None,
            working_dir: root.to_path_buf(),
            health_addr: "127.0.0.1:1".into(),
            database_path,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    fn panel_release(
        root: &Path,
        panel_version: &str,
        core_version: &str,
        core_path: &Path,
    ) -> PanelReleaseDescriptor {
        let web_root = root.join("web");
        fs::create_dir_all(&web_root).unwrap();
        fs::write(web_root.join("index.html"), b"panel web").unwrap();
        PanelReleaseDescriptor {
            version: panel_version.into(),
            core: CoreReleaseDescriptor {
                version: core_version.into(),
                path: core_path.to_string_lossy().into_owned(),
                launcher_protocol: RELEASE_PROTOCOL_VERSION,
                api_contract: 1,
                schema_epoch: 1,
                schema_revision: 30,
                target: "linux-test".into(),
                installed_at: Utc::now().to_rfc3339(),
            },
            core_digest: core_path
                .parent()
                .and_then(|path| directory_digest(path).ok())
                .unwrap_or_else(|| "0".repeat(64)),
            web_version: "2.0.1".into(),
            web_path: web_root.to_string_lossy().into_owned(),
            web_digest: directory_digest(&web_root).unwrap(),
            installed_at: Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn sqlite_url_resolves_relative_database_path() {
        let root = Path::new("/srv/niupanel");
        let path = sqlite_path(root, "sqlite://data/database/panel.db?mode=rwc").unwrap();
        assert_eq!(path, root.join("data/database/panel.db"));
    }

    #[test]
    fn database_snapshot_restores_main_and_wal_files() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("data/panel.db");
        fs::create_dir_all(database.parent().unwrap()).unwrap();
        fs::write(&database, b"database-v1").unwrap();
        fs::write(format!("{}-wal", database.display()), b"wal-v1").unwrap();
        let config = test_config(temp.path(), database.clone());
        let snapshot = create_database_snapshot(&config, "tx-test").unwrap();
        fs::write(&database, b"database-v2").unwrap();
        fs::remove_file(format!("{}-wal", database.display())).unwrap();
        restore_database_snapshot(&config, &snapshot).unwrap();
        assert_eq!(fs::read(&database).unwrap(), b"database-v1");
        assert_eq!(
            fs::read(format!("{}-wal", database.display())).unwrap(),
            b"wal-v1"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn failed_candidate_restores_database_and_clears_pending_state() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("data/panel.db");
        fs::create_dir_all(database.parent().unwrap()).unwrap();
        fs::write(&database, b"before-activation").unwrap();
        let candidate_root = temp.path().join("system/releases/panel/0.9.0");
        let candidate_binary = candidate_root.join("core/niupanel");
        fs::create_dir_all(candidate_binary.parent().unwrap()).unwrap();
        fs::write(
            &candidate_binary,
            format!(
                "#!/bin/sh\nprintf 'candidate-write' > '{}'\nexit 1\n",
                database.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&candidate_binary, fs::Permissions::from_mode(0o755)).unwrap();
        let active = panel_release(
            &temp.path().join("system/releases/panel/0.8.0"),
            "0.8.0",
            "0.8.0",
            &temp
                .path()
                .join("system/releases/panel/0.8.0/core/niupanel"),
        );
        let candidate = panel_release(&candidate_root, "0.9.0", "0.9.0", &candidate_binary);
        let pending = PendingPanelActivation {
            transaction_id: "failed-candidate-test".into(),
            candidate,
            reason: PanelActivationReason::Update,
            requested_at: Utc::now().to_rfc3339(),
            activation_snapshot_path: None,
            restore_before_start: None,
        };
        let mut state = PanelRuntimeState {
            protocol: RELEASE_PROTOCOL_VERSION,
            active,
            previous: None,
            pending: Some(pending.clone()),
            last_failure: None,
        };
        let config = test_config(temp.path(), database.clone());
        write_panel_runtime_state(&config.system_root, &state)
            .await
            .unwrap();

        let child = activate_candidate(&config, &mut state, &pending)
            .await
            .unwrap();

        assert!(child.is_none());
        assert_eq!(fs::read(&database).unwrap(), b"before-activation");
        let persisted = read_panel_runtime_state(&config.system_root).await.unwrap();
        assert!(persisted.pending.is_none());
        assert_eq!(
            persisted.last_failure.unwrap().version,
            pending.candidate.version
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn interrupted_activation_reuses_the_original_snapshot() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("data/panel.db");
        fs::create_dir_all(database.parent().unwrap()).unwrap();
        fs::write(&database, b"before-activation").unwrap();
        let candidate_root = temp.path().join("system/releases/panel/0.9.0");
        let candidate_binary = candidate_root.join("core/niupanel");
        fs::create_dir_all(candidate_binary.parent().unwrap()).unwrap();
        fs::write(
            &candidate_binary,
            format!(
                "#!/bin/sh\nprintf 'candidate-write' > '{}'\nexit 1\n",
                database.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&candidate_binary, fs::Permissions::from_mode(0o755)).unwrap();
        let active = panel_release(
            &temp.path().join("system/releases/panel/0.8.0"),
            "0.8.0",
            "0.8.0",
            &temp
                .path()
                .join("system/releases/panel/0.8.0/core/niupanel"),
        );
        let candidate = panel_release(&candidate_root, "0.9.0", "0.9.0", &candidate_binary);
        let config = test_config(temp.path(), database.clone());
        let snapshot = create_database_snapshot(&config, "interrupted-test").unwrap();
        fs::write(&database, b"partially-migrated").unwrap();
        let pending = PendingPanelActivation {
            transaction_id: "interrupted-test".into(),
            candidate,
            reason: PanelActivationReason::Update,
            requested_at: Utc::now().to_rfc3339(),
            activation_snapshot_path: Some(snapshot.to_string_lossy().into_owned()),
            restore_before_start: None,
        };
        let mut state = PanelRuntimeState {
            protocol: RELEASE_PROTOCOL_VERSION,
            active,
            previous: None,
            pending: Some(pending.clone()),
            last_failure: None,
        };
        write_panel_runtime_state(&config.system_root, &state)
            .await
            .unwrap();

        let child = activate_candidate(&config, &mut state, &pending)
            .await
            .unwrap();

        assert!(child.is_none());
        assert_eq!(fs::read(&database).unwrap(), b"before-activation");
        let persisted = read_panel_runtime_state(&config.system_root).await.unwrap();
        assert!(persisted.pending.is_none());
        assert_eq!(
            persisted.last_failure.unwrap().transaction_id,
            "interrupted-test"
        );
    }

    #[tokio::test]
    async fn newer_bundled_panel_uses_the_normal_activation_queue() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("data/panel.db");
        let mut config = test_config(temp.path(), database);
        config.bootstrap_panel_version = Some("0.8.2".into());
        config.bootstrap_core_version = Some("0.8.2".into());
        config.bootstrap_web_version = Some("2.0.2".into());
        fs::write(&config.bundled_binary, b"bundled core").unwrap();
        fs::create_dir_all(&config.bundled_web_dir).unwrap();
        fs::write(config.bundled_web_dir.join("index.html"), b"bundled web").unwrap();

        let active_root = config.system_root.join("releases/panel/0.8.1");
        let active_binary = active_root.join("core/niupanel");
        fs::create_dir_all(active_binary.parent().unwrap()).unwrap();
        fs::write(&active_binary, b"active core").unwrap();
        let active = panel_release(&active_root, "0.8.1", "0.8.1", &active_binary);
        initialize_runtime(&config.system_root, &active)
            .await
            .unwrap();
        let mut state = read_panel_runtime_state(&config.system_root).await.unwrap();

        reconcile_bundled_release(&config, &mut state)
            .await
            .unwrap();

        let persisted = read_panel_runtime_state(&config.system_root).await.unwrap();
        let pending = persisted.pending.unwrap();
        assert_eq!(persisted.active.version, "0.8.1");
        assert_eq!(pending.candidate.version, "0.8.2");
        assert_eq!(pending.candidate.core.version, "0.8.2");
        assert_eq!(pending.candidate.web_version, "2.0.2");
        verify_panel_release(&pending.candidate).unwrap();
    }

    #[tokio::test]
    async fn damaged_active_release_queues_previous_with_its_database_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let config = test_config(temp.path(), temp.path().join("data/panel.db"));
        let previous_root = config.system_root.join("releases/panel/0.8.1");
        let previous_binary = previous_root.join("core/niupanel");
        fs::create_dir_all(previous_binary.parent().unwrap()).unwrap();
        fs::write(&previous_binary, b"previous core").unwrap();
        let previous = panel_release(&previous_root, "0.8.1", "0.8.1", &previous_binary);
        let active = panel_release(
            &config.system_root.join("releases/panel/0.8.2"),
            "0.8.2",
            "0.8.2",
            &config
                .system_root
                .join("releases/panel/0.8.2/core/niupanel"),
        );
        let snapshot = config.system_root.join("snapshots/activation-to-damaged");
        fs::create_dir_all(&snapshot).unwrap();
        let mut state = PanelRuntimeState {
            protocol: RELEASE_PROTOCOL_VERSION,
            active: active.clone(),
            previous: Some(previous.clone()),
            pending: None,
            last_failure: None,
        };
        commit_panel_activation(
            &config.system_root,
            &state,
            &PanelActivationTransaction {
                transaction_id: "activation-to-damaged".into(),
                from_version: previous.version.clone(),
                to_version: active.version.clone(),
                reason: PanelActivationReason::Update,
                snapshot_path: snapshot.to_string_lossy().into_owned(),
                requested_at: Utc::now().to_rfc3339(),
                completed_at: Utc::now().to_rfc3339(),
                successful: true,
                error: None,
            },
        )
        .await
        .unwrap();

        recover_missing_active(&config, &mut state).await.unwrap();

        let restored = read_panel_runtime_state(&config.system_root).await.unwrap();
        let pending = restored.pending.unwrap();
        assert_eq!(restored.active.version, "0.8.2");
        assert_eq!(pending.candidate.version, "0.8.1");
        assert!(matches!(pending.reason, PanelActivationReason::Recovery));
        assert_eq!(
            pending.restore_before_start.as_deref(),
            Some(snapshot.to_string_lossy().as_ref())
        );
    }
}
