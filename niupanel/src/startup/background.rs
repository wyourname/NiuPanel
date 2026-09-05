use anyhow::Result;
use niupanel_common::constants::settings::{
    NPM_REGISTRY_MIRROR, PNPM_NODE_DIST_MIRROR, UV_PYPI_MIRROR, UV_PYTHON_MIRROR,
};
use niupanel_common::{error, info, warn};
use niupanel_core::audit::service::AuditService;
use niupanel_core::event_bus::{EventBus, SystemEvent, SystemNotification};
use niupanel_core::handlers::{db_sync::DbSyncHandler, notification::NotificationHandler};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement,
};
use std::path::Path;
use std::sync::Arc;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

pub type SharedSystemMetrics = Arc<tokio::sync::RwLock<niupanel_common::metrics::SystemMetrics>>;

pub async fn spawn_background_tasks(state: crate::common::state::AppState) -> Result<()> {
    let db = state.db.clone();
    let event_bus = state.event_bus.clone();
    spawn_notification_handler(db.clone(), event_bus.clone(), state.http_client.clone());
    spawn_db_sync_handler(db.clone(), event_bus.clone());

    crate::modules::settings::service::start_log_cleanup_scheduler(db.clone()).await;
    crate::modules::git_sync::service::GitService::start_auto_sync_scheduler(db.clone()).await;
    start_login_attempts_cleanup_scheduler(db.clone()).await;
    start_api_keys_cleanup_scheduler(db.clone()).await;
    spawn_upgrade_self_check(db.clone(), event_bus.clone());
    start_sandbox_environments_recovery(db.clone()).await;
    start_telegram_bot(state).await;

    Ok(())
}

pub fn spawn_system_shell_recovery(db: DatabaseConnection) {
    tokio::spawn(async move {
        if let Ok(Some(env_model)) = niupanel_entity::environments::Entity::find()
            .filter(niupanel_entity::environments::Column::Name.eq("System"))
            .one(&db)
            .await
        {
            let Some(reqs) = env_model.requirements else {
                return;
            };

            let packages: Vec<String> = reqs
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if packages.is_empty() {
                return;
            }

            info!(
                "Starting background recovery of linux packages: {:?}",
                packages
            );

            let mut cmd_update = tokio::process::Command::new("apt-get");
            cmd_update.arg("update");
            let _ = cmd_update.output().await;

            let mut cmd = tokio::process::Command::new("apt-get");
            cmd.arg("install").arg("-y").args(&packages);
            cmd.env("DEBIAN_FRONTEND", "noninteractive");

            match cmd.output().await {
                Ok(output) if output.status.success() => {
                    info!("Successfully recovered Shell packages.");
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("Failed to recover Shell packages: {}", stderr);
                }
                Err(e) => {
                    error!("Error executing apt-get for package recovery: {}", e);
                }
            }
        }
    });
}

pub fn spawn_upgrade_self_check(db: DatabaseConnection, event_bus: EventBus) {
    tokio::spawn(async move {
        match collect_upgrade_self_check_report(&db).await {
            Ok(Some(report)) => {
                warn!("Upgrade self-check found issues: {}", report);
                event_bus.publish(SystemEvent::System(SystemNotification::Alert {
                    message: report,
                }));
            }
            Ok(None) => {
                info!("Upgrade self-check passed");
            }
            Err(e) => {
                warn!("Upgrade self-check failed to run: {}", e);
                event_bus.publish(SystemEvent::System(SystemNotification::Alert {
                    message: format!("升级后自检执行失败: {}", e),
                }));
            }
        }
    });
}

async fn collect_upgrade_self_check_report(db: &DatabaseConnection) -> Result<Option<String>> {
    let mut findings = Vec::new();

    if !has_index(db, "idx-users-email").await? {
        findings.push("用户邮箱唯一索引 `idx-users-email` 不存在".to_string());
    }

    let duplicate_emails = duplicate_user_emails(db).await?;
    if !duplicate_emails.is_empty() {
        let samples = duplicate_emails
            .into_iter()
            .take(10)
            .map(|(email, count)| format!("{}({})", email, count))
            .collect::<Vec<_>>()
            .join(", ");
        findings.push(format!("发现重复邮箱: {}", samples));
    }

    if findings.is_empty() {
        Ok(None)
    } else {
        Ok(Some(findings.join("; ")))
    }
}

async fn duplicate_user_emails(db: &DatabaseConnection) -> Result<Vec<(String, i64)>> {
    let sql = r#"
        SELECT email, COUNT(*) AS cnt
        FROM users
        WHERE email IS NOT NULL AND email != ''
        GROUP BY email
        HAVING COUNT(*) > 1
        ORDER BY cnt DESC, email ASC
    "#;

    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            sql.to_string(),
        ))
        .await?;

    let mut result = Vec::new();
    for row in rows {
        let email: String = row.try_get_by_index(0)?;
        let count: i64 = row.try_get_by_index(1)?;
        result.push((email, count));
    }
    Ok(result)
}

async fn has_index(db: &DatabaseConnection, index_name: &str) -> Result<bool> {
    let sql = format!(
        "SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = '{}'",
        index_name.replace('\'', "''")
    );
    let row = db
        .query_one_raw(Statement::from_string(db.get_database_backend(), sql))
        .await?;
    Ok(row.is_some())
}

pub fn spawn_system_metrics_updater(system_metrics: SharedSystemMetrics) {
    tokio::spawn(async move {
        let refresh_kind = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::nothing().with_ram());
        let mut system = System::new_with_specifics(refresh_kind);
        let os_info = System::long_os_version().unwrap_or_else(|| "Unknown".to_string());
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));

        loop {
            interval.tick().await;

            system.refresh_cpu_usage();
            system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
            let (memory_total, memory_used) =
                visible_memory(system.total_memory(), system.used_memory());

            let metrics = niupanel_common::metrics::SystemMetrics {
                cpu_usage: system.global_cpu_usage(),
                memory_total,
                memory_used,
                uptime: System::uptime(),
                os_info: os_info.clone(),
            };

            let mut m = system_metrics.write().await;
            *m = metrics;
        }
    });
}

fn visible_memory(host_total: u64, host_used: u64) -> (u64, u64) {
    const CGROUP_V2: (&str, &str) = ("/sys/fs/cgroup/memory.max", "/sys/fs/cgroup/memory.current");
    const CGROUP_V1: (&str, &str) = (
        "/sys/fs/cgroup/memory/memory.limit_in_bytes",
        "/sys/fs/cgroup/memory/memory.usage_in_bytes",
    );

    for (limit_path, current_path) in [CGROUP_V2, CGROUP_V1] {
        let Some(limit) = read_cgroup_memory_value(limit_path) else {
            continue;
        };
        let Some(current) = read_cgroup_memory_value(current_path) else {
            continue;
        };

        // Bare-metal hosts commonly expose `max` or a sentinel much larger
        // than physical RAM. Only a real, tighter container limit should
        // replace host memory in the status UI.
        if limit > 0 && limit < host_total {
            return (limit, current.min(limit));
        }
    }

    (host_total, host_used)
}

fn read_cgroup_memory_value(path: impl AsRef<Path>) -> Option<u64> {
    let raw = std::fs::read_to_string(path).ok()?;
    parse_cgroup_memory_value(&raw)
}

fn parse_cgroup_memory_value(raw: &str) -> Option<u64> {
    let value = raw.trim();
    if value.is_empty() || value == "max" {
        return None;
    }
    value.parse().ok()
}

#[cfg(test)]
mod metrics_tests {
    use super::parse_cgroup_memory_value;

    #[test]
    fn parses_finite_cgroup_memory_values() {
        assert_eq!(parse_cgroup_memory_value("536870912\n"), Some(536_870_912));
        assert_eq!(parse_cgroup_memory_value("max\n"), None);
        assert_eq!(parse_cgroup_memory_value("invalid"), None);
    }
}

fn spawn_notification_handler(
    db: DatabaseConnection,
    event_bus: EventBus,
    http_client: reqwest::Client,
) {
    let notify_handler = NotificationHandler::new(event_bus, db, http_client);
    tokio::spawn(async move {
        notify_handler.run().await;
    });
}

fn spawn_db_sync_handler(db: DatabaseConnection, event_bus: EventBus) {
    let db_sync_handler = DbSyncHandler::new(event_bus, db);
    tokio::spawn(async move {
        db_sync_handler.run().await;
    });
}

async fn start_telegram_bot(state: crate::common::state::AppState) {
    use niupanel_bot::TelegramBot;
    use niupanel_core::event_bus::{SystemEvent, SystemNotification};
    use tokio_util::sync::CancellationToken;

    let state_clone = state.clone();

    tokio::spawn(async move {
        let mut current_cancel_token: Option<CancellationToken> = None;
        let mut _current_bot: Option<Arc<TelegramBot>> = None;

        if let Some((bot, token)) = load_and_start_telegram_bot(&state_clone).await {
            _current_bot = Some(bot);
            current_cancel_token = Some(token);
        }

        let mut rx = state_clone.event_bus.subscribe();
        loop {
            match rx.recv().await {
                Ok(SystemEvent::System(SystemNotification::SettingChanged { key, .. })) => {
                    if key == "plugin.telegram.config" {
                        info!("Telegram config changed, reloading bot...");

                        if let Some(token) = current_cancel_token.take() {
                            token.cancel();
                        }
                        _current_bot = None;

                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                        if let Some((bot, token)) = load_and_start_telegram_bot(&state_clone).await
                        {
                            _current_bot = Some(bot);
                            current_cancel_token = Some(token);
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                _ => {}
            }
        }
    });
}

async fn load_and_start_telegram_bot(
    state: &crate::common::state::AppState,
) -> Option<(
    Arc<niupanel_bot::TelegramBot>,
    tokio_util::sync::CancellationToken,
)> {
    use niupanel_bot::TelegramBot;
    use tokio_util::sync::CancellationToken;

    match crate::modules::telegram::handlers::load_stored_telegram_config(&state.db).await {
        Ok(config) => {
            if config.enabled && !config.token.is_empty() {
                if let Err(error) =
                    crate::modules::telegram::handlers::validate_config(&state.db, &config).await
                {
                    warn!("Telegram configuration is invalid: {}", error);
                    return None;
                }
                info!("Telegram Bot initializing...");
                let agent_handler =
                    telegram_agent_handler(state.clone(), config.agent_plugin_id.clone());
                let bot = Arc::new(TelegramBot::new(config).await);
                let token = CancellationToken::new();

                let bot_clone = bot.clone();
                let eb_clone = state.event_bus.clone();
                let db_clone = state.db.clone();
                let token_clone = token.clone();

                tokio::spawn(async move {
                    if let Err(e) = bot_clone
                        .run(eb_clone, db_clone, agent_handler, token_clone)
                        .await
                    {
                        tracing::error!("Telegram Bot error: {}", e);
                    }
                });

                return Some((bot, token));
            }
        }
        Err(error) => warn!("Telegram configuration could not be loaded: {}", error),
    }

    None
}

fn telegram_agent_handler(
    state: crate::common::state::AppState,
    configured_plugin_id: String,
) -> niupanel_bot::telegram::agent::TelegramAgentHandler {
    use niupanel_bot::telegram::agent::{TelegramAgentRequest, TelegramAgentResponse};
    use niupanel_plugin::{PluginActionCaller, PluginActionInvokeRequest};

    let plugin_id = if configured_plugin_id.trim().is_empty() {
        "niupanel-private-agents".to_string()
    } else {
        configured_plugin_id
    };
    Arc::new(move |request: TelegramAgentRequest, cancellation| {
        let plugin_id = plugin_id.clone();
        let state = state.clone();
        Box::pin(async move {
            use niupanel_bot::telegram::agent::{
                TelegramAgentAction, TelegramAgentError, TelegramAgentProgress,
            };
            use niupanel_common::error::AppError;

            let session_id = format!("telegram:{}", request.session_key());
            let user = crate::modules::auth::middleware::load_authenticated_user(
                &state,
                request.user_id,
                format!(
                    "telegram:{}:{}:{}",
                    request.chat_id,
                    request.thread_id.unwrap_or(0),
                    request.telegram_user_id
                ),
            )
            .await
            .map_err(|error| TelegramAgentError::Failed(error.to_string()))?;
            let grant_binding = crate::modules::agent_policy::ApprovalGrantBinding {
                plugin_id: plugin_id.clone(),
                principal: format!("user:{}", request.user_id),
                channel: "telegram".to_string(),
                session_id: session_id.clone(),
            };
            match request.action {
                TelegramAgentAction::EnableYolo => {
                    if user.role != niupanel_common::auth::permissions::UserRole::Admin {
                        return Err(TelegramAgentError::Failed(
                            "只有面板管理员可以启用 YOLO 模式".to_string(),
                        ));
                    }
                    let issued = crate::modules::agent_policy::issue_grant(
                        grant_binding.clone(),
                        crate::modules::agent_policy::EffectiveApprovalMode::Yolo,
                        None,
                    )
                    .await;
                    audit_telegram_grant_issued(&state, &user, &grant_binding, &issued).await;
                    return Ok(TelegramAgentResponse {
                        text: "当前 Telegram 会话已启用临时 YOLO。显式禁止的工具仍不会执行。使用 /mode normal 退出。".to_string(),
                    });
                }
                TelegramAgentAction::DisableYolo => {
                    if user.role != niupanel_common::auth::permissions::UserRole::Admin {
                        return Err(TelegramAgentError::Failed(
                            "只有面板管理员可以修改审批模式".to_string(),
                        ));
                    }
                    let revoked = crate::modules::agent_policy::revoke_grants(&grant_binding).await;
                    audit_telegram_grants_revoked(
                        &state,
                        &user,
                        &grant_binding,
                        revoked,
                        "mode_normal",
                    )
                    .await;
                    return Ok(TelegramAgentResponse {
                        text: "当前 Telegram 会话已恢复全局审批策略。".to_string(),
                    });
                }
                TelegramAgentAction::ResetSession => {
                    let revoked = crate::modules::agent_policy::revoke_grants(&grant_binding).await;
                    if revoked > 0 {
                        audit_telegram_grants_revoked(
                            &state,
                            &user,
                            &grant_binding,
                            revoked,
                            "session_reset",
                        )
                        .await;
                    }
                }
                TelegramAgentAction::Chat => {}
            }
            let (action, input) = if request.action == TelegramAgentAction::ResetSession {
                (
                    "session_delete",
                    serde_json::json!({ "session_id": session_id }),
                )
            } else {
                (
                    "chat",
                    serde_json::json!({
                        "message": request.text.clone(),
                        "attachments": request.attachments.clone(),
                        "session_id": session_id.clone(),
                        "panel_context": request.panel_context.clone(),
                        "route": "telegram",
                        "locale": "zh-CN",
                    }),
                )
            };
            let gateway = crate::modules::agent_tools::AgentToolGateway::for_plugin(
                state,
                user,
                &plugin_id,
                crate::modules::agent_tools::AgentInvocationIdentity {
                    principal: format!("user:{}", request.user_id),
                    channel: "telegram".to_string(),
                    session_id: Some(session_id.clone()),
                    presented_grant: None,
                    allow_implicit_grant: true,
                },
            )
            .await
            .map_err(|error| TelegramAgentError::Failed(error.to_string()))?;
            if request.action == TelegramAgentAction::ResetSession {
                gateway.cancel_pending_operations().await;
            }
            let coordination_scope = gateway.coordination_scope();
            let _session_guard =
                crate::modules::agent_invocations::acquire_session(&coordination_scope).await;
            let idempotency = crate::modules::agent_invocations::begin(
                &coordination_scope,
                action,
                Some(&request.client_request_id),
                &input,
            )
            .await
            .map_err(|error| TelegramAgentError::Failed(error.to_string()))?;
            let token = match idempotency {
                crate::modules::agent_invocations::IdempotencyDecision::Execute(token) => token,
                crate::modules::agent_invocations::IdempotencyDecision::Replay(output) => {
                    let text = if request.action == TelegramAgentAction::ResetSession {
                        "当前 Telegram Agent 会话已清空。".to_string()
                    } else {
                        output
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .filter(|message| !message.trim().is_empty())
                            .ok_or_else(|| {
                                TelegramAgentError::Failed(
                                    "Agent 重放结果缺少 message 字段".to_string(),
                                )
                            })?
                            .to_string()
                    };
                    return Ok(TelegramAgentResponse { text });
                }
            };
            let base_tool_handler = gateway.handler();
            let invocation_context = gateway.invocation_context(
                PluginActionCaller::Telegram,
                Some(&request.client_request_id),
            );
            let progress = request.progress.clone();
            let tool_handler = move |call: niupanel_plugin::ProcessPluginToolCall| {
                progress.send_replace(TelegramAgentProgress::CallingTool(call.tool.clone()));
                base_tool_handler(call)
            };
            let response = crate::modules::plugins::service::unified_plugin_service()
                .invoke_action_with_tools_context_cancellable(
                    &plugin_id,
                    PluginActionCaller::Telegram,
                    PluginActionInvokeRequest {
                        action: action.to_string(),
                        input,
                        client_request_id: Some(request.client_request_id.clone()),
                    },
                    Some(invocation_context),
                    gateway.definitions(),
                    tool_handler,
                    cancellation,
                )
                .await;

            let response = match response {
                Ok(response) => {
                    crate::modules::agent_invocations::complete(token, &response.output).await;
                    response
                }
                Err(AppError::Cancelled) => {
                    crate::modules::agent_invocations::fail(token).await;
                    if request.action == TelegramAgentAction::Chat
                        && let Err(cleanup_error) =
                            crate::modules::plugins::service::unified_plugin_service()
                                .invoke_action(
                                    &plugin_id,
                                    PluginActionCaller::Telegram,
                                    PluginActionInvokeRequest {
                                        action: "session_interrupt".to_string(),
                                        input: serde_json::json!({
                                            "session_id": session_id.clone(),
                                            "reason": "telegram_run_cancelled",
                                        }),
                                        client_request_id: Some(format!(
                                            "{}:interrupt",
                                            request.client_request_id
                                        )),
                                    },
                                )
                                .await
                    {
                        warn!(
                            "Failed to mark cancelled Telegram Agent session '{}' as interrupted: {}",
                            session_id, cleanup_error
                        );
                    }
                    return Err(TelegramAgentError::Cancelled);
                }
                Err(error) => {
                    crate::modules::agent_invocations::fail(token).await;
                    return Err(TelegramAgentError::Failed(error.to_string()));
                }
            };

            request.report_progress(TelegramAgentProgress::Finalizing);
            let text = if request.action == TelegramAgentAction::ResetSession {
                "当前 Telegram Agent 会话已清空。".to_string()
            } else {
                response
                    .output
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .filter(|message| !message.trim().is_empty())
                    .ok_or_else(|| {
                        TelegramAgentError::Failed("Agent 返回内容缺少 message 字段".to_string())
                    })?
                    .to_string()
            };
            Ok(TelegramAgentResponse { text })
        })
    })
}

async fn audit_telegram_grant_issued(
    state: &crate::common::state::AppState,
    user: &niupanel_common::auth::permissions::AuthenticatedUser,
    binding: &crate::modules::agent_policy::ApprovalGrantBinding,
    issued: &crate::modules::agent_policy::IssuedApprovalGrant,
) {
    AuditService::log_user(
        &state.db,
        user,
        "agent.approval_grant.issued",
        "plugin",
        Some(binding.plugin_id.clone()),
        Some(
            serde_json::json!({
                "grant_id": issued.grant_id,
                "principal": binding.principal,
                "channel": binding.channel,
                "session_id": binding.session_id,
                "expires_in_seconds": issued.expires_in_seconds,
            })
            .to_string(),
        ),
    )
    .await;
}

async fn audit_telegram_grants_revoked(
    state: &crate::common::state::AppState,
    user: &niupanel_common::auth::permissions::AuthenticatedUser,
    binding: &crate::modules::agent_policy::ApprovalGrantBinding,
    revoked: usize,
    reason: &'static str,
) {
    AuditService::log_user(
        &state.db,
        user,
        "agent.approval_grant.revoked",
        "plugin",
        Some(binding.plugin_id.clone()),
        Some(
            serde_json::json!({
                "principal": binding.principal,
                "channel": binding.channel,
                "session_id": binding.session_id,
                "revoked": revoked,
                "reason": reason,
            })
            .to_string(),
        ),
    )
    .await;
}

async fn start_api_keys_cleanup_scheduler(db: DatabaseConnection) {
    use niupanel_entity::api_keys;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        loop {
            interval.tick().await;
            info!("运行过期 API Key 清理任务...");

            let now = chrono::Utc::now();
            let res = api_keys::Entity::delete_many()
                .filter(api_keys::Column::ExpiresAt.lt(now))
                .exec(&db)
                .await;

            match res {
                Ok(res) if res.rows_affected > 0 => {
                    info!("已清理 {} 个过期的 API Key", res.rows_affected);
                }
                Err(e) => error!("清理过期 API Key 失败: {}", e),
                _ => {}
            }
        }
    });
}

async fn start_login_attempts_cleanup_scheduler(db: DatabaseConnection) {
    use niupanel_entity::login_attempts;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        loop {
            interval.tick().await;
            info!("启动登录日志定时清除任务");

            let threshold = chrono::Utc::now() - chrono::Duration::days(7);
            let res = login_attempts::Entity::delete_many()
                .filter(login_attempts::Column::LastAttemptAt.lt(threshold))
                .exec(&db)
                .await;

            match res {
                Ok(res) if res.rows_affected > 0 => {
                    info!("Cleaned up {} old login attempt records", res.rows_affected);
                }
                Err(e) => error!("Failed to cleanup login attempts: {}", e),
                _ => {}
            }
        }
    });
}

async fn start_sandbox_environments_recovery(db: DatabaseConnection) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        info!("Running sandbox environments auto-recovery check...");

        use niupanel_core::runtime::RuntimeManager;
        use niupanel_core::settings::SettingsManager;
        use std::path::PathBuf;

        match niupanel_entity::environments::Entity::find().all(&db).await {
            Ok(envs) => {
                let mut mirrors = std::collections::HashMap::new();
                if let Ok(m) = SettingsManager::get(&db, UV_PYTHON_MIRROR).await {
                    mirrors.insert("UV_PYTHON_INSTALL_MIRROR".to_string(), m);
                }
                if let Ok(m) = SettingsManager::get(&db, UV_PYPI_MIRROR).await {
                    mirrors.insert("UV_INDEX_URL".to_string(), m);
                }
                if let Ok(m) = SettingsManager::get(&db, PNPM_NODE_DIST_MIRROR).await {
                    mirrors.insert("PNPM_NODE_DIST_MIRROR".to_string(), m);
                }
                if let Ok(m) = SettingsManager::get(&db, NPM_REGISTRY_MIRROR).await {
                    mirrors.insert("npm_config_registry".to_string(), m);
                }

                for env_model in envs {
                    if env_model.name == "System"
                        || env_model.name == "System Node"
                        || env_model.name == "System Shell"
                    {
                        continue;
                    }

                    let base_folder = if env_model.env_type == "python" {
                        "python"
                    } else if env_model.env_type == "node" {
                        "node"
                    } else {
                        continue;
                    };

                    if env_model.env_type == "python" {
                        let env_dir =
                            PathBuf::from(&niupanel_common::config::Config::global().runtimes_dir)
                                .join(base_folder)
                                .join(&env_model.name);

                        if env_dir.exists() {
                            continue;
                        }
                        info!(
                            "Python environment '{}' is missing. Initiating auto-recovery...",
                            env_model.name
                        );
                    }

                    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                    let log_name = env_model.name.clone();

                    tokio::spawn(async move {
                        while let Some(msg) = rx.recv().await {
                            niupanel_common::debug!("[Recovery {}] {}", log_name, msg);
                        }
                    });

                    let req_text = env_model.requirements.clone().unwrap_or_default();
                    let requirements: Vec<String> = req_text
                        .lines()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    if env_model.env_type == "python" {
                        let env_dir =
                            PathBuf::from(&niupanel_common::config::Config::global().runtimes_dir)
                                .join(base_folder)
                                .join(&env_model.name);
                        match RuntimeManager::open_python_environment(
                            env_dir,
                            Some(&env_model.version),
                            Some(mirrors.clone()),
                        )
                        .await
                        {
                            Ok(env) => {
                                if !req_text.is_empty() {
                                    let _ = env
                                        .install_requirements(&req_text, false, tx.clone())
                                        .await;
                                }
                                info!("Successfully recovered python sandbox: {}", env_model.name);
                            }
                            Err(e) => {
                                error!("Failed to recover python sandbox {}: {}", env_model.name, e)
                            }
                        }
                    } else if env_model.env_type == "node" {
                        let local_versions = RuntimeManager::list_local_node_versions()
                            .await
                            .unwrap_or_default();
                        let version_installed =
                            local_versions.iter().any(|(v, _)| v == &env_model.version);
                        let mut can_restore_node_packages = version_installed;

                        if !version_installed {
                            info!(
                                "Node environment '{}' is missing. Initiating auto-recovery...",
                                env_model.name
                            );
                            match RuntimeManager::create_node_environment(
                                env_model.version.clone(),
                                tx.clone(),
                                Some(mirrors.clone()),
                            )
                            .await
                            {
                                Ok(_) => {
                                    can_restore_node_packages = true;
                                    info!(
                                        "Successfully recovered node environment: {}",
                                        env_model.name
                                    );
                                }
                                Err(e) => error!(
                                    "Failed to recover node environment {}: {}",
                                    env_model.name, e
                                ),
                            }
                        } else {
                            info!(
                                "Node environment '{}' is already installed. Skipping recovery.",
                                env_model.name
                            );
                        }

                        if can_restore_node_packages && !requirements.is_empty() {
                            let env = match RuntimeManager::open_node_environment(
                                Some(&env_model.version),
                                Some(mirrors.clone()),
                            ) {
                                Ok(env) => env,
                                Err(e) => {
                                    error!(
                                        "Failed to open node environment during recovery {}: {}",
                                        env_model.name, e
                                    );
                                    continue;
                                }
                            };
                            let _ = env.install_packages(&requirements, tx.clone()).await;
                        }
                    }
                }
            }
            Err(e) => error!("Recovery check failed to fetch environments: {}", e),
        }

        info!("Sandbox environments auto-recovery check completed.");
    });
}
