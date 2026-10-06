use crate::common::state::AppState;
use crate::modules::plugins::service::unified_plugin_service;
use crate::modules::telegram::handlers::{load_stored_telegram_config, validate_config};
use niupanel_common::config::Config;
use niupanel_common::telegram_protocol::*;
use niupanel_common::warn;
use niupanel_core::event_bus::{SystemEvent, SystemNotification, TaskEvent, TelegramEvent};
use niupanel_entity::{task_status::TaskStatus, tasks};
use niupanel_plugin::{PluginInvokeRequest, PluginProcessProtocol, PluginService, PluginStatus};
use sea_orm::EntityTrait;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

pub(super) fn spawn(state: AppState) {
    tokio::spawn(async move {
        let service = unified_plugin_service();
        loop {
            match load_stored_telegram_config(&state.db).await {
                Ok(config) => {
                    if available(&service, &config.agent_plugin_id) {
                        if let Err(error) = retire_legacy_plugin(&service).await {
                            warn!("Cannot retire legacy Telegram plugin: {error}");
                        } else if config.enabled {
                            if let Err(error) = validate_config(&state.db, &config).await {
                                warn!("Telegram channel configuration is invalid: {error}");
                            } else if let Err(error) = run(&state, &service, config).await {
                                warn!("Agent Telegram channel stopped: {error}");
                            }
                        }
                    }
                }
                Err(error) => warn!("Cannot read Telegram channel configuration: {error}"),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

fn available(service: &PluginService, plugin_id: &str) -> bool {
    service.get_plugin(plugin_id).is_ok_and(|plugin| {
        plugin.enabled
            && matches!(plugin.status, PluginStatus::Enabled)
            && plugin
                .manifest
                .capabilities
                .iter()
                .any(|item| item == TELEGRAM_CAPABILITY)
            && matches!(plugin.manifest.protocol, PluginProcessProtocol::JsonLines)
            && plugin
                .manifest
                .capabilities
                .iter()
                .any(|item| item == "agents.chat")
    })
}

// Plugin data lives outside the install directory and survives retiring the old package.
async fn retire_legacy_plugin(service: &PluginService) -> anyhow::Result<()> {
    if service.get_plugin(LEGACY_TELEGRAM_PLUGIN_ID).is_ok() {
        service
            .set_enabled_async(LEGACY_TELEGRAM_PLUGIN_ID, false)
            .await?;
        service.uninstall_async(LEGACY_TELEGRAM_PLUGIN_ID).await?;
    }
    Ok(())
}

async fn legacy_ledger(plugin_id: &str) -> anyhow::Result<Option<String>> {
    let config = Config::global();
    let existing = config
        .plugins_dir
        .join(".data/plugin")
        .join(plugin_id)
        .join("telegram-agent-message-ledger.json");
    if tokio::fs::try_exists(existing).await? {
        return Ok(None);
    }
    for path in [
        config
            .plugins_dir
            .join(".data/plugin")
            .join(LEGACY_TELEGRAM_PLUGIN_ID)
            .join("telegram-agent-message-ledger.json"),
        config.system_dir.join("telegram-agent-message-ledger.json"),
    ] {
        match tokio::fs::read_to_string(&path).await {
            Ok(contents) => return Ok(Some(contents)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(None)
}

struct AgentRun {
    cancellation: CancellationToken,
    progress: watch::Receiver<TelegramAgentProgress>,
}

async fn run(
    state: &AppState,
    service: &PluginService,
    config: TelegramBotConfig,
) -> anyhow::Result<()> {
    let mut events = state.event_bus.subscribe();
    let handler = super::telegram_agent::telegram_agent_handler(
        state.clone(),
        config.agent_plugin_id.clone(),
    );
    let cancellation = CancellationToken::new();
    let mut tasks = JoinSet::new();
    let (completed_tx, mut completed_rx) = mpsc::channel::<TelegramAgentUpdate>(32);
    let mut runs: HashMap<String, AgentRun> = HashMap::new();
    let mut first = true;
    let mut sessions = HashSet::new();
    let result: anyhow::Result<()> = async {
        loop {
            if !available(service, &config.agent_plugin_id)
                || load_stored_telegram_config(&state.db).await? != config
            {
                break;
            }
            let mut notifications = Vec::new();
            // Task log events are frequent; bound work per tick so cancellation stays responsive.
            for _ in 0..256 {
                match events.try_recv() {
                    Ok(event) => match notification(state, &config, event).await {
                        Ok(Some(event)) => notifications.push(event),
                        Ok(None) => {}
                        Err(error) => warn!("Cannot forward Telegram notification: {error}"),
                    },
                    Err(tokio::sync::broadcast::error::TryRecvError::Lagged(count)) => {
                        warn!("Telegram channel skipped {count} events")
                    }
                    Err(_) => break,
                }
                if notifications.len() >= 16 {
                    break;
                }
            }
            let mut updates = runs
                .iter()
                .map(|(id, run)| TelegramAgentUpdate {
                    id: id.clone(),
                    progress: run.progress.borrow().clone(),
                    result: None,
                })
                .collect::<Vec<_>>();
            while let Ok(update) = completed_rx.try_recv() {
                runs.remove(&update.id);
                updates.push(update);
            }
            while tasks.try_join_next().is_some() {}
            let legacy_ledger = if first {
                legacy_ledger(&config.agent_plugin_id).await?
            } else {
                None
            };
            let input = TelegramTickInput {
                config: first.then(|| config.clone()),
                legacy_ledger,
                notifications,
                updates,
            };
            let response = crate::modules::telegram::transport::invoke(
                &config.agent_plugin_id,
                PluginInvokeRequest {
                    action: "transport_tick".to_string(),
                    input: serde_json::to_value(input)?,
                    timeout_sec: Some(10),
                },
            )
            .await?;
            first = false;
            let output: TelegramTickOutput = serde_json::from_value(response.output)?;
            anyhow::ensure!(output.running, "Telegram transport is not running");
            // Re-check after invocation: disable/uninstall may have raced with a returned request.
            if !available(service, &config.agent_plugin_id)
                || load_stored_telegram_config(&state.db).await? != config
            {
                break;
            }
            for id in output.cancelled {
                if let Some(run) = runs.get(&id) {
                    run.cancellation.cancel();
                }
            }
            for mut request in output.requests {
                let id = request.client_request_id.clone();
                if runs.contains_key(&id) {
                    continue;
                }
                let rejection = if id.is_empty() || id.len() > 256 || runs.len() >= 16 {
                    Some("Telegram 请求无效或运行队列已满")
                } else if let Some(user_id) = config.bound_user_id(&request) {
                    request.user_id = user_id;
                    None
                } else {
                    Some("当前发送者或会话未绑定面板用户")
                };
                if let Some(message) = rejection {
                    completed_tx
                        .send(TelegramAgentUpdate {
                            id,
                            progress: TelegramAgentProgress::Finalizing,
                            result: Some(Err(TelegramAgentError::Failed(message.to_string()))),
                        })
                        .await?;
                    continue;
                }
                sessions.insert((request.user_id, request.session_key()));
                let (progress, receiver) = watch::channel(TelegramAgentProgress::Running);
                request.progress = progress;
                let child_token = cancellation.child_token();
                runs.insert(
                    id.clone(),
                    AgentRun {
                        cancellation: child_token.clone(),
                        progress: receiver,
                    },
                );
                let handler = handler.clone();
                let completed = completed_tx.clone();
                tasks.spawn(async move {
                    let result = handler(request, child_token).await;
                    let _ = completed
                        .send(TelegramAgentUpdate {
                            id,
                            progress: TelegramAgentProgress::Finalizing,
                            result: Some(result),
                        })
                        .await;
                });
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Ok(())
    }
    .await;
    cancellation.cancel();
    service.stop_telegram_channel(&config.agent_plugin_id).await;
    // Let Agent gateway cancellation record interrupted sessions before dropping futures.
    if tokio::time::timeout(Duration::from_secs(5), async {
        while tasks.join_next().await.is_some() {}
    })
    .await
    .is_err()
    {
        tasks.abort_all();
    }
    for (user_id, session_id) in sessions {
        crate::modules::agent_policy::revoke_grants(
            &crate::modules::agent_policy::ApprovalGrantBinding {
                plugin_id: config.agent_plugin_id.clone(),
                principal: format!("user:{user_id}"),
                channel: "telegram".to_string(),
                session_id: format!("telegram:{session_id}"),
            },
        )
        .await;
    }
    result
}

async fn notification(
    state: &AppState,
    config: &TelegramBotConfig,
    event: SystemEvent,
) -> anyhow::Result<Option<TelegramNotification>> {
    let notification = match event {
        SystemEvent::Task(TaskEvent::StatusChanged {
            task_id,
            job_id,
            status,
            is_system,
            output,
            ..
        }) => {
            if !matches!(
                status,
                TaskStatus::Finished
                    | TaskStatus::Failed
                    | TaskStatus::Stopped
                    | TaskStatus::Cancelled
            ) {
                return Ok(None);
            }
            let (task_name, user_id) = if is_system {
                (None, None)
            } else {
                let Some(task) = tasks::Entity::find_by_id(task_id).one(&state.db).await? else {
                    return Ok(None);
                };
                if !task.notify {
                    return Ok(None);
                }
                (Some(task.name), Some(task.user_id))
            };
            let event_name = if status == TaskStatus::Finished {
                "success"
            } else {
                "failed"
            };
            if !config.chat_bindings.iter().any(|binding| {
                user_id.is_none_or(|id| binding.user_id == id)
                    && binding.events.iter().any(|event| event == event_name)
            }) {
                return Ok(None);
            }
            let output = match output {
                Some(output) => Some(output),
                None => {
                    let dir = if is_system {
                        job_id.map(|id| Config::global().jobs_dir.join(id.to_string()))
                    } else {
                        Some(Config::global().logs_dir.join(task_id.to_string()))
                    };
                    match dir {
                        Some(dir) => super::telegram_logs::read_latest_log(&dir, 1000).await,
                        None => None,
                    }
                }
            };
            TelegramNotification::Task {
                task_id,
                job_id,
                is_system,
                status: format!("{status:?}"),
                task_name,
                user_id,
                output,
            }
        }
        SystemEvent::System(SystemNotification::Alert { message }) => {
            TelegramNotification::Alert { message }
        }
        SystemEvent::System(SystemNotification::Notification {
            title,
            content,
            level,
        }) => TelegramNotification::Notification {
            title,
            content,
            level,
        },
        SystemEvent::Telegram(TelegramEvent::MessageSent {
            chat_id,
            thread_id,
            text,
            file_name,
            ..
        }) => {
            if !config.chat_bindings.iter().any(|b| {
                b.chat_id == chat_id && (b.thread_id.is_none() || b.thread_id == thread_id)
            }) {
                return Ok(None);
            }
            let file = if let Some(path) = file_name {
                let source = std::path::Path::new(&path);
                let bytes = tokio::fs::read(source).await?;
                Some(TelegramOutboundFile {
                    content_base64: openssl::base64::encode_block(&bytes),
                    name: source
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                })
            } else {
                None
            };
            TelegramNotification::Message {
                chat_id,
                thread_id,
                text,
                file,
            }
        }
        SystemEvent::Task(TaskEvent::Log { .. })
        | SystemEvent::System(SystemNotification::SettingChanged { .. }) => return Ok(None),
    };
    Ok(Some(notification))
}
