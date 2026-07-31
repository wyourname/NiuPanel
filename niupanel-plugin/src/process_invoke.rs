use super::*;

pub(super) async fn invoke_worker<F>(
    worker: Arc<Mutex<ProcessWorker>>,
    spec: ProcessPluginSpec,
    request: PluginInvokeRequest,
    tool_handler: &F,
) -> Result<PluginInvokeResponse>
where
    F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
{
    let mut worker = worker.lock().await;
    if worker.is_exited()? {
        *worker = ProcessWorker::spawn(&spec).await?;
    }
    worker.invoke(&spec, request, tool_handler).await
}

pub(super) async fn invoke_single_shot(
    spec: ProcessPluginSpec,
    request: PluginInvokeRequest,
) -> Result<PluginInvokeResponse> {
    let plugin_dir = absolute_existing_path(&spec.plugin_dir)?;
    let entry_path = absolute_existing_path(&safe_entry_path(&plugin_dir, &spec.entry)?)?;
    if !entry_path.exists() {
        return Err(AppError::ValidationError(format!(
            "Process plugin entry '{}' does not exist",
            spec.entry
        )));
    }

    let protocol_request = build_protocol_request(&spec, &request);
    let request_id = protocol_request.request_id.clone();
    let payload = serde_json::to_vec(&protocol_request)?;
    let timeout_sec = timeout_sec_for(&spec, &request);
    let plugin_data_dir = ensure_plugin_data_dir(&spec)?;

    let started = Instant::now();
    let mut command = sandboxed_plugin_command(&spec, &plugin_dir, &plugin_data_dir)?;
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|err| AppError::ProcessStart {
        command: entry_path.display().to_string(),
        source: err,
    })?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::Internal("Process plugin stdin is unavailable".to_string()))?;
    stdin.write_all(&payload).await?;
    stdin.write_all(b"\n").await?;
    drop(stdin);

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Internal("Process plugin stdout is unavailable".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::Internal("Process plugin stderr is unavailable".to_string()))?;
    let stdout_task = tokio::spawn(read_limited(stdout));
    let stderr_task = tokio::spawn(read_limited(stderr));

    let status = match timeout(Duration::from_secs(timeout_sec), child.wait()).await {
        Ok(status) => status?,
        Err(_) => {
            let _ = child.start_kill();
            return Err(AppError::ProcessFailed {
                command: entry_path.display().to_string(),
                exit_code: None,
                stderr: format!("Process plugin timed out after {timeout_sec}s"),
            });
        }
    };

    let stdout = stdout_task
        .await
        .map_err(|err| AppError::Internal(format!("Process stdout task failed: {err}")))??;
    let stderr = stderr_task
        .await
        .map_err(|err| AppError::Internal(format!("Process stderr task failed: {err}")))??;
    let stderr_text = String::from_utf8_lossy(&stderr).trim().to_string();

    if !status.success() {
        return Err(AppError::ProcessFailed {
            command: entry_path.display().to_string(),
            exit_code: status.code(),
            stderr: stderr_text,
        });
    }

    let stdout_text = String::from_utf8_lossy(&stdout).trim().to_string();
    let response: ProcessPluginResponse = serde_json::from_str(&stdout_text).map_err(|err| {
        AppError::Serialization(format!(
            "Process plugin returned invalid JSON: {err}; stdout={}",
            truncate(&stdout_text, 500)
        ))
    })?;
    validate_response(&request_id, response)?;
    Ok(PluginInvokeResponse {
        plugin_id: spec.plugin_id,
        extension_point: spec.extension_point,
        version: spec.version,
        runtime: PluginRuntime::Process,
        request_id,
        output: response_output(&stdout_text)?,
        stderr: (!stderr_text.is_empty()).then_some(stderr_text),
        duration_ms: started.elapsed().as_millis(),
    })
}

pub(super) fn build_protocol_request(
    spec: &ProcessPluginSpec,
    request: &PluginInvokeRequest,
) -> ProcessPluginRequest {
    ProcessPluginRequest {
        protocol_version: PROCESS_PROTOCOL_VERSION,
        request_id: format!(
            "{}-{}",
            spec.plugin_id,
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ),
        plugin_id: spec.plugin_id.clone(),
        agent_id: (spec.extension_point == "agents").then(|| spec.plugin_id.clone()),
        extension_point: spec.extension_point.clone(),
        action: request.action.clone(),
        input: request.input.clone(),
        capabilities: spec.capabilities.clone(),
        tools: spec.tools.clone(),
    }
}

pub(super) fn validate_response(request_id: &str, response: ProcessPluginResponse) -> Result<()> {
    if response
        .request_id
        .as_deref()
        .is_some_and(|id| id != request_id)
    {
        return Err(AppError::ValidationError(
            "Process plugin response request_id does not match request".to_string(),
        ));
    }
    if !response.ok {
        let error = response.error.unwrap_or(ProcessPluginError {
            code: "plugin_error".to_string(),
            message: "Process plugin returned ok=false".to_string(),
        });
        return Err(AppError::ExternalApi(format!(
            "Process plugin error [{}]: {}",
            error.code, error.message
        )));
    }
    Ok(())
}

pub(super) fn response_output(raw: &str) -> Result<serde_json::Value> {
    let response: ProcessPluginResponse = serde_json::from_str(raw)?;
    Ok(response.output.unwrap_or(serde_json::Value::Null))
}

pub(super) fn timeout_sec_for(spec: &ProcessPluginSpec, request: &PluginInvokeRequest) -> u64 {
    request
        .timeout_sec
        .or(spec.timeout_sec)
        .unwrap_or(DEFAULT_PROCESS_TIMEOUT_SEC)
        .clamp(1, MAX_PROCESS_TIMEOUT_SEC)
}

pub(super) fn ensure_plugin_data_dir(spec: &ProcessPluginSpec) -> Result<PathBuf> {
    let data_dir = niupanel_common::config::Config::global()
        .plugins_dir
        .join(".data")
        .join(&spec.extension_point)
        .join(&spec.plugin_id);
    fs::create_dir_all(&data_dir)?;
    secure_plugin_data_dir(&data_dir)?;
    absolute_existing_path(&data_dir)
}
