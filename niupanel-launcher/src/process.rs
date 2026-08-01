use super::*;

pub(crate) fn spawn_core(
    config: &LauncherConfig,
    release: &PanelReleaseDescriptor,
) -> Result<Child> {
    verify_panel_release(release).map_err(|error| anyhow!(error))?;
    let mut command = Command::new(&release.core.path);
    command
        .current_dir(&config.working_dir)
        .env("NIUPANEL_LAUNCHED", "1")
        .env("NIUPANEL_ACTIVE_PANEL_VERSION", &release.version)
        .env("NIUPANEL_ACTIVE_CORE_VERSION", &release.core.version)
        .env("NIUPANEL_ACTIVE_WEB_VERSION", &release.web_version)
        .env("NIUPANEL_ACTIVE_WEB_DIR", &release.web_path)
        .env("NIUPANEL_SYSTEM_DIR", &config.system_root)
        .kill_on_drop(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;

        let launcher_pid = unsafe { nix::libc::getpid() };
        unsafe {
            command.as_std_mut().pre_exec(move || {
                if nix::libc::prctl(nix::libc::PR_SET_PDEATHSIG, nix::libc::SIGKILL, 0, 0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if nix::libc::getppid() != launcher_pid {
                    return Err(std::io::Error::other(
                        "Launcher exited while the Panel process was starting",
                    ));
                }
                Ok(())
            });
        }
    }
    command
        .spawn()
        .with_context(|| format!("Failed to start Panel {}", release.version))
}

pub(crate) async fn activate_candidate(
    config: &LauncherConfig,
    state: &mut PanelRuntimeState,
    requested: &PendingPanelActivation,
) -> Result<Option<Child>> {
    let mut pending = requested.clone();
    let resuming = pending.activation_snapshot_path.is_some();
    let snapshot = match pending.activation_snapshot_path.as_deref() {
        Some(path) => {
            let path = PathBuf::from(path);
            if !path.is_dir() {
                bail!(
                    "Activation snapshot for transaction {} is missing: {}",
                    pending.transaction_id,
                    path.display()
                );
            }
            path
        }
        None => {
            let path = create_database_snapshot(config, &pending.transaction_id)?;
            pending.activation_snapshot_path = Some(path.to_string_lossy().into_owned());
            state.pending = Some(pending.clone());
            write_panel_runtime_state(&config.system_root, state)
                .await
                .map_err(|error| anyhow!(error))?;
            path
        }
    };
    if resuming {
        restore_database_snapshot(config, &snapshot).with_context(|| {
            format!(
                "Failed to restore the pre-activation snapshot for transaction {}",
                pending.transaction_id
            )
        })?;
    }
    if let Some(restore_path) = pending.restore_before_start.as_deref() {
        if let Err(error) = restore_database_snapshot(config, Path::new(restore_path)) {
            fail_activation(
                config,
                state,
                &pending,
                &snapshot,
                format!("Failed to restore rollback snapshot: {error:#}"),
            )
            .await?;
            return Ok(None);
        }
    }

    let mut child = match spawn_core(config, &pending.candidate) {
        Ok(child) => child,
        Err(error) => {
            let message = format!("Failed to start candidate Panel: {error:#}");
            restore_database_snapshot(config, &snapshot)?;
            fail_activation(config, state, &pending, &snapshot, message).await?;
            return Ok(None);
        }
    };
    let outcome = wait_for_candidate(config, &mut child, &pending.candidate).await?;
    match outcome {
        CandidateOutcome::Healthy => {
            let previous = std::mem::replace(&mut state.active, pending.candidate.clone());
            state.previous = if matches!(
                pending.reason,
                niupanel_common::version::PanelActivationReason::Recovery
            ) {
                None
            } else {
                Some(previous.clone())
            };
            state.pending = None;
            state.last_failure = None;
            commit_panel_activation(
                &config.system_root,
                state,
                &PanelActivationTransaction {
                    transaction_id: pending.transaction_id.clone(),
                    from_version: previous.version,
                    to_version: pending.candidate.version.clone(),
                    reason: pending.reason.clone(),
                    snapshot_path: snapshot.to_string_lossy().into_owned(),
                    requested_at: pending.requested_at.clone(),
                    completed_at: Utc::now().to_rfc3339(),
                    successful: true,
                    error: None,
                },
            )
            .await
            .map_err(|error| anyhow!(error))?;
            Ok(Some(child))
        }
        CandidateOutcome::Failed(message) => {
            terminate_child(&mut child).await;
            restore_database_snapshot(config, &snapshot)?;
            fail_activation(config, state, &pending, &snapshot, message).await?;
            Ok(None)
        }
        CandidateOutcome::Shutdown => {
            terminate_child(&mut child).await;
            restore_database_snapshot(config, &snapshot)?;
            fail_activation(
                config,
                state,
                &pending,
                &snapshot,
                "Panel activation was interrupted by shutdown".into(),
            )
            .await?;
            Ok(None)
        }
    }
}

pub(crate) async fn fail_activation(
    config: &LauncherConfig,
    state: &mut PanelRuntimeState,
    pending: &PendingPanelActivation,
    snapshot: &Path,
    message: String,
) -> Result<()> {
    state.pending = None;
    state.last_failure = Some(PanelActivationFailure {
        transaction_id: pending.transaction_id.clone(),
        version: pending.candidate.version.clone(),
        failed_at: Utc::now().to_rfc3339(),
        message: message.clone(),
    });
    commit_panel_activation(
        &config.system_root,
        state,
        &PanelActivationTransaction {
            transaction_id: pending.transaction_id.clone(),
            from_version: state.active.version.clone(),
            to_version: pending.candidate.version.clone(),
            reason: pending.reason.clone(),
            snapshot_path: snapshot.to_string_lossy().into_owned(),
            requested_at: pending.requested_at.clone(),
            completed_at: Utc::now().to_rfc3339(),
            successful: false,
            error: Some(message),
        },
    )
    .await
    .map_err(|error| anyhow!(error))
}

pub(crate) async fn wait_for_candidate(
    config: &LauncherConfig,
    child: &mut Child,
    candidate: &PanelReleaseDescriptor,
) -> Result<CandidateOutcome> {
    let started = Instant::now();
    let mut healthy_at = None;
    loop {
        if config.shutdown.load(Ordering::Relaxed) {
            return Ok(CandidateOutcome::Shutdown);
        }
        if let Some(status) = child.try_wait()? {
            return Ok(CandidateOutcome::Failed(format!(
                "Candidate Panel exited before activation completed: {status}"
            )));
        }
        if health_check(config, &candidate.version).await {
            let healthy = healthy_at.get_or_insert_with(Instant::now);
            if healthy.elapsed() >= ACTIVATION_PROBATION {
                return Ok(CandidateOutcome::Healthy);
            }
        } else if healthy_at.is_some() {
            return Ok(CandidateOutcome::Failed(
                "Candidate Panel became unhealthy during activation probation".into(),
            ));
        }
        if started.elapsed() >= HEALTH_TIMEOUT + ACTIVATION_PROBATION {
            return Ok(CandidateOutcome::Failed(
                "Candidate Panel health check timed out".into(),
            ));
        }
        tokio::time::sleep(Duration::from_millis(750)).await;
    }
}

pub(crate) async fn health_check(config: &LauncherConfig, expected_version: &str) -> bool {
    let operation = async {
        let mut stream = tokio::net::TcpStream::connect(&config.health_addr).await?;
        let request = format!(
            "GET /healthz HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            config.health_addr
        );
        stream.write_all(request.as_bytes()).await?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        let response = String::from_utf8_lossy(&response);
        Ok::<bool, std::io::Error>(
            response.starts_with("HTTP/1.1 200")
                && response.contains(&format!("\"panel_version\":\"{expected_version}\"")),
        )
    };
    tokio::time::timeout(Duration::from_secs(2), operation)
        .await
        .ok()
        .and_then(std::result::Result::ok)
        .unwrap_or(false)
}

pub(crate) async fn wait_for_active_child(
    config: &LauncherConfig,
    child: &mut Child,
) -> Result<()> {
    loop {
        if config.shutdown.load(Ordering::Relaxed) {
            terminate_child(child).await;
            return Ok(());
        }
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

pub(crate) async fn terminate_child(child: &mut Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid as i32),
            nix::sys::signal::Signal::SIGTERM,
        );
        if tokio::time::timeout(Duration::from_secs(10), child.wait())
            .await
            .is_ok()
        {
            return;
        }
    }
    let _ = child.kill().await;
}

pub(crate) fn install_shutdown_watcher(flag: Arc<AtomicBool>) {
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM handler");
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = terminate.recv() => {}
            }
        }
        #[cfg(not(unix))]
        let _ = tokio::signal::ctrl_c().await;
        flag.store(true, Ordering::Relaxed);
    });
}
