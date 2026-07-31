use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use niupanel_common::version::{
    API_CONTRACT_VERSION, CORE_RELEASE_MANIFEST_FILE, CoreActivationFailure, CoreActivationReason,
    CoreActivationTransaction, CoreReleaseDescriptor, CoreReleaseManifest, CoreRuntimeState,
    PendingCoreActivation, RELEASE_PROTOCOL_VERSION, SCHEMA_EPOCH, SCHEMA_REVISION,
    read_core_release_manifest, read_core_runtime_state, write_core_activation_transaction,
    write_core_runtime_state,
};
use semver::Version;
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

    let bundled = register_bundled_release(&config)?;
    let mut state = match read_core_runtime_state(&config.system_root) {
        Ok(state) => state,
        Err(_) => CoreRuntimeState {
            protocol: RELEASE_PROTOCOL_VERSION,
            active: bundled.clone(),
            previous: None,
            pending: None,
            last_failure: None,
        },
    };
    recover_missing_active(&mut state, &bundled);
    schedule_bundled_release(&config, &mut state, &bundled)?;
    write_core_runtime_state(&config.system_root, &state).map_err(|error| anyhow!(error))?;

    install_shutdown_watcher(config.shutdown.clone());

    loop {
        if config.shutdown.load(Ordering::Relaxed) {
            return Ok(());
        }
        state = read_core_runtime_state(&config.system_root).map_err(|error| anyhow!(error))?;
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
        let config = LauncherConfig {
            system_root: temp.path().join("system"),
            bundled_binary: temp.path().join("niupanel"),
            bundled_manifest_root: temp.path().to_path_buf(),
            working_dir: temp.path().to_path_buf(),
            health_addr: "127.0.0.1:0".into(),
            database_path: database.clone(),
            shutdown: Arc::new(AtomicBool::new(false)),
        };
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
        let candidate_binary = temp.path().join("candidate-core");
        fs::write(
            &candidate_binary,
            format!(
                "#!/bin/sh\nprintf 'candidate-write' > '{}'\nexit 1\n",
                database.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&candidate_binary, fs::Permissions::from_mode(0o755)).unwrap();

        let active = CoreReleaseDescriptor {
            version: "0.8.0".into(),
            path: temp
                .path()
                .join("active-core")
                .to_string_lossy()
                .into_owned(),
            launcher_protocol: RELEASE_PROTOCOL_VERSION,
            api_contract: 1,
            schema_epoch: 1,
            schema_revision: 29,
            target: "linux-test".into(),
            installed_at: Utc::now().to_rfc3339(),
        };
        let candidate = CoreReleaseDescriptor {
            version: "0.9.0".into(),
            path: candidate_binary.to_string_lossy().into_owned(),
            launcher_protocol: RELEASE_PROTOCOL_VERSION,
            api_contract: 1,
            schema_epoch: 1,
            schema_revision: 30,
            target: "linux-test".into(),
            installed_at: Utc::now().to_rfc3339(),
        };
        let pending = PendingCoreActivation {
            transaction_id: "failed-candidate-test".into(),
            candidate,
            reason: CoreActivationReason::Update,
            requested_at: Utc::now().to_rfc3339(),
            restore_before_start: None,
        };
        let mut state = CoreRuntimeState {
            protocol: RELEASE_PROTOCOL_VERSION,
            active,
            previous: None,
            pending: Some(pending.clone()),
            last_failure: None,
        };
        let config = LauncherConfig {
            system_root: temp.path().join("system"),
            bundled_binary: temp.path().join("niupanel"),
            bundled_manifest_root: temp.path().to_path_buf(),
            working_dir: temp.path().to_path_buf(),
            health_addr: "127.0.0.1:1".into(),
            database_path: database.clone(),
            shutdown: Arc::new(AtomicBool::new(false)),
        };
        write_core_runtime_state(&config.system_root, &state).unwrap();

        let child = activate_candidate(&config, &mut state, &pending)
            .await
            .unwrap();

        assert!(child.is_none());
        assert_eq!(fs::read(&database).unwrap(), b"before-activation");
        let persisted = read_core_runtime_state(&config.system_root).unwrap();
        assert!(persisted.pending.is_none());
        assert_eq!(
            persisted.last_failure.unwrap().version,
            pending.candidate.version
        );
    }
}
