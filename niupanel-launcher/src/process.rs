use super::*;

pub(crate) fn spawn_core(
    config: &LauncherConfig,
    release: &CoreReleaseDescriptor,
) -> Result<Child> {
    if !Path::new(&release.path).is_file() {
        bail!("Core binary does not exist: {}", release.path);
    }
    let mut command = Command::new(&release.path);
    command
        .current_dir(&config.working_dir)
        .env("NIUPANEL_LAUNCHED", "1")
        .env("NIUPANEL_ACTIVE_CORE_VERSION", &release.version)
        .env("NIUPANEL_SYSTEM_DIR", &config.system_root)
        .kill_on_drop(true);
    command
        .spawn()
        .with_context(|| format!("Failed to start Core {}", release.version))
}

pub(crate) async fn activate_candidate(
    config: &LauncherConfig,
    state: &mut CoreRuntimeState,
    pending: &PendingCoreActivation,
) -> Result<Option<Child>> {
    let snapshot = create_database_snapshot(config, &pending.transaction_id)?;
    if let Some(restore_path) = pending.restore_before_start.as_deref() {
        if let Err(error) = restore_database_snapshot(config, Path::new(restore_path)) {
            fail_activation(
                config,
                state,
                pending,
                &snapshot,
                format!("Failed to restore rollback snapshot: {error:#}"),
            )?;
            return Ok(None);
        }
    }

    let mut child = match spawn_core(config, &pending.candidate) {
        Ok(child) => child,
        Err(error) => {
            let message = format!("Failed to start candidate Core: {error:#}");
            restore_database_snapshot(config, &snapshot)?;
            fail_activation(config, state, pending, &snapshot, message)?;
            return Ok(None);
        }
    };
    let outcome = wait_for_candidate(config, &mut child, &pending.candidate).await?;
    match outcome {
        CandidateOutcome::Healthy => {
            let previous = std::mem::replace(&mut state.active, pending.candidate.clone());
            state.previous = Some(previous.clone());
            state.pending = None;
            state.last_failure = None;
            write_core_runtime_state(&config.system_root, state).map_err(|error| anyhow!(error))?;
            write_core_activation_transaction(
                &config.system_root,
                &CoreActivationTransaction {
                    transaction_id: pending.transaction_id.clone(),
                    from: previous,
                    to: pending.candidate.clone(),
                    reason: pending.reason.clone(),
                    snapshot_path: snapshot.to_string_lossy().into_owned(),
                    requested_at: pending.requested_at.clone(),
                    completed_at: Utc::now().to_rfc3339(),
                    successful: true,
                    error: None,
                },
            )
            .map_err(|error| anyhow!(error))?;
            Ok(Some(child))
        }
        CandidateOutcome::Failed(message) => {
            terminate_child(&mut child).await;
            restore_database_snapshot(config, &snapshot)?;
            fail_activation(config, state, pending, &snapshot, message)?;
            Ok(None)
        }
        CandidateOutcome::Shutdown => {
            terminate_child(&mut child).await;
            restore_database_snapshot(config, &snapshot)?;
            fail_activation(
                config,
                state,
                pending,
                &snapshot,
                "Core activation was interrupted by shutdown".into(),
            )?;
            Ok(None)
        }
    }
}

pub(crate) fn fail_activation(
    config: &LauncherConfig,
    state: &mut CoreRuntimeState,
    pending: &PendingCoreActivation,
    snapshot: &Path,
    message: String,
) -> Result<()> {
    state.pending = None;
    state.last_failure = Some(CoreActivationFailure {
        transaction_id: pending.transaction_id.clone(),
        version: pending.candidate.version.clone(),
        failed_at: Utc::now().to_rfc3339(),
        message: message.clone(),
    });
    write_core_runtime_state(&config.system_root, state).map_err(|error| anyhow!(error))?;
    write_core_activation_transaction(
        &config.system_root,
        &CoreActivationTransaction {
            transaction_id: pending.transaction_id.clone(),
            from: state.active.clone(),
            to: pending.candidate.clone(),
            reason: pending.reason.clone(),
            snapshot_path: snapshot.to_string_lossy().into_owned(),
            requested_at: pending.requested_at.clone(),
            completed_at: Utc::now().to_rfc3339(),
            successful: false,
            error: Some(message),
        },
    )
    .map_err(|error| anyhow!(error))
}

pub(crate) async fn wait_for_candidate(
    config: &LauncherConfig,
    child: &mut Child,
    candidate: &CoreReleaseDescriptor,
) -> Result<CandidateOutcome> {
    let started = Instant::now();
    let mut healthy_at = None;
    loop {
        if config.shutdown.load(Ordering::Relaxed) {
            return Ok(CandidateOutcome::Shutdown);
        }
        if let Some(status) = child.try_wait()? {
            return Ok(CandidateOutcome::Failed(format!(
                "Candidate Core exited before activation completed: {status}"
            )));
        }
        if health_check(config, &candidate.version).await {
            let healthy = healthy_at.get_or_insert_with(Instant::now);
            if healthy.elapsed() >= ACTIVATION_PROBATION {
                return Ok(CandidateOutcome::Healthy);
            }
        } else if healthy_at.is_some() {
            return Ok(CandidateOutcome::Failed(
                "Candidate Core became unhealthy during activation probation".into(),
            ));
        }
        if started.elapsed() >= HEALTH_TIMEOUT + ACTIVATION_PROBATION {
            return Ok(CandidateOutcome::Failed(
                "Candidate Core health check timed out".into(),
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
                && response.contains(&format!("\"core_version\":\"{expected_version}\"")),
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
