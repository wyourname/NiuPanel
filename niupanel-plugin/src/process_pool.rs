use super::*;

#[derive(Clone, Default)]
pub struct PluginProcessRuntime {
    pools: Arc<Mutex<HashMap<String, Arc<ProcessWorkerPool>>>>,
}

impl PluginProcessRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn invoke_single_shot(
        &self,
        spec: ProcessPluginSpec,
        request: PluginInvokeRequest,
    ) -> Result<PluginInvokeResponse> {
        invoke_single_shot(spec, request).await
    }

    pub async fn invoke_json_lines<F>(
        &self,
        spec: ProcessPluginSpec,
        request: PluginInvokeRequest,
        tool_handler: F,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
    {
        let key = process_pool_key(&spec);
        let pool = {
            let mut pools = self.pools.lock().await;
            pools
                .entry(key)
                .or_insert_with(|| Arc::new(ProcessWorkerPool::new(spec.clone())))
                .clone()
        };
        pool.invoke(request, &tool_handler).await
    }

    pub async fn stop_plugin(&self, plugin_id: &str) {
        let mut pools = self.pools.lock().await;
        pools.retain(|key, _| !key.starts_with(&format!("{plugin_id}@")));
    }
}

pub(super) struct ProcessWorkerPool {
    spec: ProcessPluginSpec,
    workers: Mutex<Vec<Arc<Mutex<ProcessWorker>>>>,
}

impl ProcessWorkerPool {
    pub(super) fn new(spec: ProcessPluginSpec) -> Self {
        Self {
            spec,
            workers: Mutex::new(Vec::new()),
        }
    }

    pub(super) async fn invoke<F>(
        &self,
        request: PluginInvokeRequest,
        tool_handler: &F,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
    {
        let worker = self.worker().await?;
        let timeout_sec = timeout_sec_for(&self.spec, &request);
        timeout(
            Duration::from_secs(timeout_sec),
            invoke_worker(worker, self.spec.clone(), request, tool_handler),
        )
        .await
        .map_err(|_| AppError::ProcessFailed {
            command: self.spec.entry.clone(),
            exit_code: None,
            stderr: format!("Process plugin timed out after {timeout_sec}s"),
        })?
    }

    pub(super) async fn worker(&self) -> Result<Arc<Mutex<ProcessWorker>>> {
        let mut workers = self.workers.lock().await;
        if let Some(worker) = workers.first() {
            return Ok(worker.clone());
        }

        let worker = Arc::new(Mutex::new(ProcessWorker::spawn(&self.spec).await?));
        workers.push(worker.clone());
        Ok(worker)
    }
}

pub(super) struct ProcessWorker {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl ProcessWorker {
    pub(super) async fn spawn(spec: &ProcessPluginSpec) -> Result<Self> {
        let plugin_dir = absolute_existing_path(&spec.plugin_dir)?;
        let entry_path = absolute_existing_path(&safe_entry_path(&plugin_dir, &spec.entry)?)?;
        if !entry_path.exists() {
            return Err(AppError::ValidationError(format!(
                "Process plugin entry '{}' does not exist",
                spec.entry
            )));
        }
        let plugin_data_dir = ensure_plugin_data_dir(spec)?;
        let mut command = sandboxed_plugin_command(spec, &plugin_dir, &plugin_data_dir)?;
        command
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = command.spawn().map_err(|err| AppError::ProcessStart {
            command: entry_path.display().to_string(),
            source: err,
        })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::Internal("Process plugin stdin is unavailable".to_string()))?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AppError::Internal("Process plugin stdout is unavailable".to_string())
        })?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    pub(super) async fn invoke<F>(
        &mut self,
        spec: &ProcessPluginSpec,
        request: PluginInvokeRequest,
        tool_handler: &F,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
    {
        let started = Instant::now();
        let protocol_request = build_protocol_request(spec, &request);
        let request_id = protocol_request.request_id.clone();
        let payload = serde_json::to_string(&protocol_request)?;

        self.stdin.write_all(payload.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        let mut line = String::new();
        loop {
            line.clear();
            let bytes = self.stdout.read_line(&mut line).await?;
            if bytes == 0 {
                return Err(AppError::ProcessFailed {
                    command: spec.entry.clone(),
                    exit_code: self.child.try_wait()?.and_then(|status| status.code()),
                    stderr: "Process plugin exited without a final response".to_string(),
                });
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|err| {
                AppError::Serialization(format!(
                    "Process plugin returned invalid JSON line: {err}; line={}",
                    truncate(trimmed, 500)
                ))
            })?;

            if value
                .get("type")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|kind| kind == "tool_call")
            {
                let call: ProcessPluginToolCall = serde_json::from_value(value)?;
                if call.request_id != request_id {
                    return Err(AppError::ValidationError(
                        "Process plugin tool call request_id does not match request".to_string(),
                    ));
                }
                let result = match tool_handler(call.clone()).await {
                    Ok(output) => ProcessPluginToolResult {
                        kind: "tool_result".to_string(),
                        request_id: call.request_id,
                        call_id: call.call_id,
                        ok: true,
                        output: Some(output),
                        error: None,
                    },
                    Err(err) => ProcessPluginToolResult {
                        kind: "tool_result".to_string(),
                        request_id: call.request_id,
                        call_id: call.call_id,
                        ok: false,
                        output: None,
                        error: Some(ProcessPluginError {
                            code: tool_error_code(&err).to_string(),
                            message: err.to_string(),
                        }),
                    },
                };
                self.stdin
                    .write_all(serde_json::to_string(&result)?.as_bytes())
                    .await?;
                self.stdin.write_all(b"\n").await?;
                self.stdin.flush().await?;
                continue;
            }

            let response: ProcessPluginResponse = serde_json::from_value(value)?;
            validate_response(&request_id, response)?;
            let _ = timeout(Duration::from_millis(5), self.child.wait()).await;
            return Ok(PluginInvokeResponse {
                plugin_id: spec.plugin_id.clone(),
                extension_point: spec.extension_point.clone(),
                version: spec.version.clone(),
                runtime: PluginRuntime::Process,
                request_id,
                output: response_output(trimmed)?,
                stderr: None,
                duration_ms: started.elapsed().as_millis(),
            });
        }
    }

    pub(super) fn is_exited(&mut self) -> Result<bool> {
        Ok(self.child.try_wait()?.is_some())
    }
}
