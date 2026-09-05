use super::*;

#[derive(Clone)]
pub struct PluginProcessRuntime {
    pools: Arc<Mutex<HashMap<String, Arc<ProcessWorkerPool>>>>,
    plugin_limits: Arc<Mutex<HashMap<String, Arc<Semaphore>>>>,
    global_limit: Arc<Semaphore>,
}

impl Default for PluginProcessRuntime {
    fn default() -> Self {
        Self {
            pools: Arc::new(Mutex::new(HashMap::new())),
            plugin_limits: Arc::new(Mutex::new(HashMap::new())),
            global_limit: Arc::new(Semaphore::new(MAX_CONCURRENT_PLUGIN_INVOCATIONS)),
        }
    }
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
        let _permits = self.acquire_invocation_permits(&spec).await?;
        invoke_single_shot(spec, request).await
    }

    pub async fn invoke_single_shot_cancellable(
        &self,
        spec: ProcessPluginSpec,
        request: PluginInvokeRequest,
        cancellation: CancellationToken,
    ) -> Result<PluginInvokeResponse> {
        let _permits = self
            .acquire_invocation_permits_cancellable(&spec, &cancellation)
            .await?;
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => Err(AppError::Cancelled),
            result = invoke_single_shot(spec, request) => result,
        }
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
        let _permits = self.acquire_invocation_permits(&spec).await?;
        let key = process_pool_key(&spec);
        let pool = {
            let mut pools = self.pools.lock().await;
            pools
                .entry(key)
                .or_insert_with(|| Arc::new(ProcessWorkerPool::new(spec.clone())))
                .clone()
        };
        pool.invoke(
            request,
            &tool_handler,
            false,
            &|_| Box::pin(async { Ok(()) }),
            CancellationToken::new(),
        )
        .await
    }

    pub async fn invoke_json_lines_cancellable<F>(
        &self,
        spec: ProcessPluginSpec,
        request: PluginInvokeRequest,
        tool_handler: F,
        cancellation: CancellationToken,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
    {
        let _permits = self
            .acquire_invocation_permits_cancellable(&spec, &cancellation)
            .await?;
        let key = process_pool_key(&spec);
        let pool = {
            let mut pools = self.pools.lock().await;
            pools
                .entry(key)
                .or_insert_with(|| Arc::new(ProcessWorkerPool::new(spec.clone())))
                .clone()
        };
        pool.invoke(
            request,
            &tool_handler,
            false,
            &|_| Box::pin(async { Ok(()) }),
            cancellation,
        )
        .await
    }

    pub async fn invoke_json_lines_streaming<F, S>(
        &self,
        spec: ProcessPluginSpec,
        request: PluginInvokeRequest,
        tool_handler: F,
        stream_handler: S,
        cancellation: CancellationToken,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
        S: Fn(ProcessPluginStreamEvent) -> PluginStreamFuture + Send + Sync,
    {
        let _permits = self
            .acquire_invocation_permits_cancellable(&spec, &cancellation)
            .await?;
        let key = process_pool_key(&spec);
        let pool = {
            let mut pools = self.pools.lock().await;
            pools
                .entry(key)
                .or_insert_with(|| Arc::new(ProcessWorkerPool::new(spec.clone())))
                .clone()
        };
        pool.invoke(request, &tool_handler, true, &stream_handler, cancellation)
            .await
    }

    pub async fn stop_plugin(&self, plugin_id: &str) {
        let mut pools = self.pools.lock().await;
        pools.retain(|key, _| !key.starts_with(&format!("{plugin_id}@")));
        drop(pools);
        let mut limits = self.plugin_limits.lock().await;
        limits.retain(|key, _| !key.starts_with(&format!("{plugin_id}@")));
    }

    async fn acquire_invocation_permits(
        &self,
        spec: &ProcessPluginSpec,
    ) -> Result<(OwnedSemaphorePermit, OwnedSemaphorePermit)> {
        let global = self.global_limit.clone().try_acquire_owned().map_err(|_| {
            AppError::ConcurrencyLimitExceeded(format!(
                "At most {MAX_CONCURRENT_PLUGIN_INVOCATIONS} plugin actions may run concurrently"
            ))
        })?;
        let key = process_pool_key(spec);
        let limit = {
            let mut limits = self.plugin_limits.lock().await;
            limits
                .entry(key)
                .or_insert_with(|| Arc::new(Semaphore::new(spec.worker.max)))
                .clone()
        };
        let plugin = limit.try_acquire_owned().map_err(|_| {
            AppError::ConcurrencyLimitExceeded(format!(
                "Plugin '{}' allows at most {} concurrent actions",
                spec.plugin_id, spec.worker.max
            ))
        })?;
        Ok((global, plugin))
    }

    async fn acquire_invocation_permits_cancellable(
        &self,
        spec: &ProcessPluginSpec,
        cancellation: &CancellationToken,
    ) -> Result<(OwnedSemaphorePermit, OwnedSemaphorePermit)> {
        let key = process_pool_key(spec);
        let limit = {
            let mut limits = self.plugin_limits.lock().await;
            limits
                .entry(key)
                .or_insert_with(|| Arc::new(Semaphore::new(spec.worker.max)))
                .clone()
        };
        let plugin = tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Err(AppError::Cancelled),
            permit = limit.acquire_owned() => permit.map_err(|_| {
                AppError::Internal("Plugin invocation semaphore is closed".to_string())
            })?,
        };
        let global = tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Err(AppError::Cancelled),
            permit = self.global_limit.clone().acquire_owned() => permit.map_err(|_| {
                AppError::Internal("Global plugin invocation semaphore is closed".to_string())
            })?,
        };
        Ok((global, plugin))
    }
}

pub(super) struct ProcessWorkerPool {
    spec: ProcessPluginSpec,
    workers: Arc<Mutex<ProcessWorkerPoolState>>,
}

#[derive(Default)]
struct ProcessWorkerPoolState {
    idle: Vec<ProcessWorker>,
    total: usize,
}

impl ProcessWorkerPool {
    pub(super) fn new(spec: ProcessPluginSpec) -> Self {
        Self {
            spec,
            workers: Arc::new(Mutex::new(ProcessWorkerPoolState::default())),
        }
    }

    pub(super) async fn invoke<F, S>(
        &self,
        request: PluginInvokeRequest,
        tool_handler: &F,
        stream: bool,
        stream_handler: &S,
        cancellation: CancellationToken,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
        S: Fn(ProcessPluginStreamEvent) -> PluginStreamFuture + Send + Sync,
    {
        let mut worker = self.worker().await?;
        let timeout_sec = timeout_sec_for(&self.spec, &request);
        let result = tokio::select! {
            biased;
            _ = cancellation.cancelled() => Err(AppError::Cancelled),
            result = timeout(
                Duration::from_secs(timeout_sec),
                invoke_worker(
                    worker.worker_mut(),
                    self.spec.clone(),
                    request,
                    tool_handler,
                    stream,
                    stream_handler,
                ),
            ) => result.map_err(|_| AppError::ProcessFailed {
                command: self.spec.entry.clone(),
                exit_code: None,
                stderr: format!("Process plugin timed out after {timeout_sec}s"),
            })?,
        };

        match result {
            Ok(response) => {
                worker.recycle().await;
                Ok(response)
            }
            Err(error) => {
                worker.discard().await;
                Err(error)
            }
        }
    }

    async fn worker(&self) -> Result<ProcessWorkerLease> {
        let mut workers = self.workers.lock().await;
        if let Some(worker) = workers.idle.pop() {
            return Ok(ProcessWorkerLease::new(worker, self.workers.clone()));
        }
        if workers.total >= self.spec.worker.max {
            return Err(AppError::ConcurrencyLimitExceeded(format!(
                "Plugin '{}' worker pool is busy",
                self.spec.plugin_id
            )));
        }
        workers.total += 1;
        drop(workers);

        match ProcessWorker::spawn(&self.spec).await {
            Ok(worker) => Ok(ProcessWorkerLease::new(worker, self.workers.clone())),
            Err(error) => {
                self.discard_worker().await;
                Err(error)
            }
        }
    }

    async fn discard_worker(&self) {
        let mut workers = self.workers.lock().await;
        workers.total = workers.total.saturating_sub(1);
    }
}

struct ProcessWorkerLease {
    worker: Option<ProcessWorker>,
    workers: Arc<Mutex<ProcessWorkerPoolState>>,
}

impl ProcessWorkerLease {
    fn new(worker: ProcessWorker, workers: Arc<Mutex<ProcessWorkerPoolState>>) -> Self {
        Self {
            worker: Some(worker),
            workers,
        }
    }

    fn worker_mut(&mut self) -> &mut ProcessWorker {
        self.worker.as_mut().expect("worker lease is active")
    }

    async fn recycle(mut self) {
        let worker = self.worker.take().expect("worker lease is active");
        self.workers.lock().await.idle.push(worker);
    }

    async fn discard(mut self) {
        self.worker.take();
        let mut workers = self.workers.lock().await;
        workers.total = workers.total.saturating_sub(1);
    }
}

impl Drop for ProcessWorkerLease {
    fn drop(&mut self) {
        if self.worker.take().is_none() {
            return;
        }
        if let Ok(mut workers) = self.workers.try_lock() {
            workers.total = workers.total.saturating_sub(1);
            return;
        }
        let workers = self.workers.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let mut workers = workers.lock().await;
                workers.total = workers.total.saturating_sub(1);
            });
        }
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

    pub(super) async fn invoke<F, S>(
        &mut self,
        spec: &ProcessPluginSpec,
        request: PluginInvokeRequest,
        tool_handler: &F,
        stream: bool,
        stream_handler: &S,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
        S: Fn(ProcessPluginStreamEvent) -> PluginStreamFuture + Send + Sync,
    {
        let started = Instant::now();
        let protocol_request = build_protocol_request(spec, &request, stream);
        let request_id = protocol_request.request_id.clone();
        let payload = serde_json::to_string(&protocol_request)?;
        let mut last_stream_sequence = 0_u64;

        self.stdin.write_all(payload.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        loop {
            let Some(line) = read_line_limited(&mut self.stdout).await? else {
                return Err(AppError::ProcessFailed {
                    command: spec.entry.clone(),
                    exit_code: self.child.try_wait()?.and_then(|status| status.code()),
                    stderr: "Process plugin exited without a final response".to_string(),
                });
            };

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

            let frame_kind = value.get("type").and_then(serde_json::Value::as_str);
            if frame_kind.is_some_and(|kind| kind == "tool_call") {
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

            if frame_kind.is_some_and(|kind| kind == "stream_event") {
                if !stream {
                    return Err(AppError::ValidationError(
                        "Process plugin emitted stream_event for a non-streaming request"
                            .to_string(),
                    ));
                }
                let event: ProcessPluginStreamEvent = serde_json::from_value(value)?;
                validate_stream_event(&request_id, last_stream_sequence, &event)?;
                last_stream_sequence = event.sequence;
                stream_handler(event).await?;
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

fn validate_stream_event(
    request_id: &str,
    last_sequence: u64,
    event: &ProcessPluginStreamEvent,
) -> Result<()> {
    if event.kind != "stream_event" {
        return Err(AppError::ValidationError(
            "Process plugin stream frame type must be stream_event".to_string(),
        ));
    }
    if event.request_id != request_id {
        return Err(AppError::ValidationError(
            "Process plugin stream event request_id does not match request".to_string(),
        ));
    }
    if event.sequence == 0 || event.sequence <= last_sequence {
        return Err(AppError::ValidationError(
            "Process plugin stream event sequence must be positive and strictly increasing"
                .to_string(),
        ));
    }
    if event.event.is_empty()
        || event.event.len() > 64
        || !event
            .event
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(AppError::ValidationError(
            "Process plugin stream event name is invalid".to_string(),
        ));
    }
    Ok(())
}
