pub mod agent;
pub mod context;
pub mod ledger;
pub mod runtime;

use agent::TelegramAgentHandler;
use context::NotificationContextStore;
use ledger::TelegramMessageLedger;
use niupanel_common::logger::{error, info, warn};
use niupanel_common::{config::Config, escape_tg_markdown};
use niupanel_core::event_bus::{
    EventBus, SystemEvent, SystemNotification, TaskEvent, TelegramEvent,
};
use niupanel_entity::task_status::TaskStatus;
use niupanel_entity::tasks;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{InputFile, MessageId, ParseMode, ThreadId};
use tokio_util::sync::CancellationToken;

const TELEGRAM_AGENT_MAX_CONCURRENCY: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TelegramBotConfig {
    pub enabled: bool,
    pub token: String,
    #[serde(default)]
    pub binding_schema_version: u8,
    #[serde(default)]
    pub chat_bindings: Vec<TelegramChatBinding>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub admin_chat_id: String,
    pub proxy_url: Option<String>,
    pub api_base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<String>,
    pub cf_proxy_enabled: bool,
    pub cf_host: String,
    pub cf_ip: String,
    pub cf_token: String,
    #[serde(default = "default_agent_plugin_id")]
    pub agent_plugin_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TelegramChatBinding {
    pub chat_id: String,
    pub thread_id: Option<i32>,
    pub user_id: i32,
    pub label: String,
    pub events: Vec<String>,
    pub telegram_user_ids: Vec<String>,
    #[serde(default = "default_interaction_mode")]
    pub interaction_mode: String,
}

fn default_interaction_mode() -> String {
    "direct".to_string()
}

impl Default for TelegramChatBinding {
    fn default() -> Self {
        Self {
            chat_id: String::new(),
            thread_id: None,
            user_id: 0,
            label: String::new(),
            events: Vec::new(),
            telegram_user_ids: Vec::new(),
            interaction_mode: default_interaction_mode(),
        }
    }
}

fn default_agent_plugin_id() -> String {
    "niupanel-private-agents".to_string()
}

impl Default for TelegramBotConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            binding_schema_version: 0,
            chat_bindings: Vec::new(),
            admin_chat_id: String::new(),
            proxy_url: None,
            api_base_url: None,
            events: Vec::new(),
            cf_proxy_enabled: false,
            cf_host: String::new(),
            cf_ip: String::new(),
            cf_token: String::new(),
            agent_plugin_id: default_agent_plugin_id(),
        }
    }
}

pub struct TelegramBot {
    bot: Bot,
    chat_bindings: Vec<TelegramChatBinding>,
    proxy_shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl TelegramBot {
    pub async fn new(config: TelegramBotConfig) -> Self {
        let mut proxy_url = config.proxy_url.clone();
        let mut proxy_shutdown_tx = None;

        if config.cf_proxy_enabled && !config.cf_host.is_empty() {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
            proxy_shutdown_tx = Some(tx);
            let proxy_config = niupanel_proxy::Config {
                cfhost: config.cf_host.clone(),
                cfip: config.cf_ip.clone(),
                token: config.cf_token.clone(),
                host: "127.0.0.1".to_string(),
                port: 0,
                user: String::new(),
                passwd: String::new(),
                log: std::path::PathBuf::new(),
                loglevel: "info".to_string(),
            };
            tokio::spawn(async move {
                let _ =
                    niupanel_proxy::start_server(Arc::new(proxy_config), rx, ready_tx, "AgentBot")
                        .await;
            });
            if let Ok(actual_addr) = ready_rx.await {
                proxy_url = Some(format!("socks5h://{actual_addr}"));
            }
        }

        let mut builder = teloxide_reqwest::Client::builder();
        if let Some(proxy) = proxy_url.as_deref().filter(|proxy| !proxy.is_empty())
            && let Ok(proxy) = teloxide_reqwest::Proxy::all(proxy)
        {
            builder = builder.proxy(proxy);
        }

        let client = builder.build().unwrap_or_default();
        let mut bot = Bot::with_client(config.token, client);
        if let Some(base_url) = config
            .api_base_url
            .as_deref()
            .filter(|base_url| !base_url.is_empty())
            && let Ok(base_url) = teloxide_reqwest::Url::parse(base_url)
        {
            bot = bot.set_api_url(base_url);
        }

        Self {
            bot,
            chat_bindings: config.chat_bindings,
            proxy_shutdown_tx,
        }
    }

    pub async fn run(
        &self,
        event_bus: EventBus,
        db: DatabaseConnection,
        agent_handler: TelegramAgentHandler,
        token: CancellationToken,
    ) -> anyhow::Result<()> {
        info!("Telegram Ops Agent transport started");
        let bot_identity = self.bot.get_me().await?;
        let _ = self.bot.delete_my_commands().await;
        let _ = self.bot.set_my_commands(agent::bot_commands()).await;

        let version = env!("CARGO_PKG_VERSION").replace('.', "\\.");
        let reply_contexts = NotificationContextStore::default();
        send_to_bindings(
            &self.bot,
            &self.chat_bindings,
            &format!("🚀 *NiuPanel Ops Agent Telegram v{version} 已连接*"),
            &reply_contexts,
            None,
        )
        .await;

        let notification_task = spawn_notification_transport(
            self.bot.clone(),
            self.chat_bindings.clone(),
            event_bus,
            db,
            token.clone(),
            reply_contexts.clone(),
        );

        let agent_runtime =
            runtime::TelegramAgentRuntime::new(agent_handler, TELEGRAM_AGENT_MAX_CONCURRENCY);
        let message_ledger = TelegramMessageLedger::persistent(
            Config::global()
                .system_dir
                .join("telegram-agent-message-ledger.json"),
        )
        .await
        .map_err(anyhow::Error::msg)?;

        let handler = Update::filter_message().endpoint(agent::handle_message);
        let mut dispatcher = Dispatcher::builder(self.bot.clone(), handler)
            .dependencies(dptree::deps![
                self.chat_bindings.clone(),
                bot_identity,
                agent_runtime,
                message_ledger,
                reply_contexts
            ])
            .distribution_function(|_| None::<()>)
            .enable_ctrlc_handler()
            .build();

        tokio::select! {
            _ = dispatcher.dispatch() => info!("Telegram Ops Agent dispatcher stopped"),
            _ = token.cancelled() => info!("Telegram Ops Agent transport is stopping"),
        }
        notification_task.abort();
        Ok(())
    }
}

fn spawn_notification_transport(
    bot: Bot,
    chat_bindings: Vec<TelegramChatBinding>,
    event_bus: EventBus,
    db: DatabaseConnection,
    token: CancellationToken,
    reply_contexts: NotificationContextStore,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut receiver = event_bus.subscribe();
        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                event = receiver.recv() => match event {
                    Ok(event) => handle_system_event(
                        &bot,
                        &chat_bindings,
                        event,
                        &db,
                        &reply_contexts,
                    ).await,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        warn!("Telegram notification transport lagged by {} events", count);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    })
}

async fn handle_system_event(
    bot: &Bot,
    chat_bindings: &[TelegramChatBinding],
    event: SystemEvent,
    db: &DatabaseConnection,
    reply_contexts: &NotificationContextStore,
) {
    let (text, reply_context, event_name, target_user_id) = match event {
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
                return;
            }
            let event_name = if status == TaskStatus::Finished {
                "success"
            } else {
                "failed"
            };
            let (task_name, task_user_id) = if is_system {
                (None, None)
            } else {
                let Some(task) = tasks::Entity::find_by_id(task_id)
                    .one(db)
                    .await
                    .ok()
                    .flatten()
                else {
                    warn!(
                        "Skipping Telegram notification for missing task {}",
                        task_id
                    );
                    return;
                };
                (Some(task.name), Some(task.user_id))
            };
            let output = match output {
                Some(output) => Some(output),
                None if is_system => match job_id {
                    Some(job_id) => {
                        read_latest_log(&Config::global().jobs_dir.join(job_id.to_string()), 1_000)
                            .await
                    }
                    None => None,
                },
                None => {
                    read_latest_log(&Config::global().logs_dir.join(task_id.to_string()), 1_000)
                        .await
                }
            };
            (
                task_notification(
                    task_id,
                    job_id,
                    is_system,
                    status,
                    task_name.as_deref(),
                    output.as_deref(),
                ),
                Some(task_reply_context(
                    task_id,
                    job_id,
                    is_system,
                    status,
                    task_name.as_deref(),
                )),
                Some(event_name),
                task_user_id,
            )
        }
        SystemEvent::System(SystemNotification::Alert { message }) => (
            format!("🚨 *系统警报*\n{}", escape_tg_markdown(&message)),
            Some(serde_json::json!({
                "source": "telegram_notification_reply",
                "event": "system_alert",
                "severity": "alert",
            })),
            Some("alert"),
            None,
        ),
        SystemEvent::System(SystemNotification::Notification {
            title,
            content,
            level,
        }) => {
            let event_name = if matches!(level.as_str(), "error" | "warn") {
                "alert"
            } else {
                "notification"
            };
            let marker = match level.as_str() {
                "error" => "🔴",
                "warn" => "🟠",
                _ => "ℹ️",
            };
            (
                format!(
                    "{} *{}*\n{}",
                    marker,
                    escape_tg_markdown(&title),
                    escape_tg_markdown(&content)
                ),
                Some(serde_json::json!({
                    "source": "telegram_notification_reply",
                    "event": "system_notification",
                    "title": title,
                    "level": level,
                })),
                Some(event_name),
                None,
            )
        }
        SystemEvent::Telegram(TelegramEvent::MessageSent {
            chat_id,
            thread_id,
            text,
            file_name,
            ..
        }) => {
            let Some(binding) = find_binding(chat_bindings, &chat_id, thread_id) else {
                warn!("Rejected Telegram outbound event for unbound Chat ID");
                return;
            };
            if let Some(file_name) = file_name {
                let mut request = bot.send_document(chat_id.clone(), InputFile::file(file_name));
                if let Some(thread_id) = thread_id {
                    request = request.message_thread_id(ThreadId(MessageId(thread_id)));
                }
                if let Some(caption) = text {
                    request = request.caption(caption);
                }
                let _ = request.await;
            } else if let Some(text) = text {
                let mut request = bot.send_message(chat_id, text);
                if let Some(thread_id) = thread_id.or(binding.thread_id) {
                    request = request.message_thread_id(ThreadId(MessageId(thread_id)));
                }
                let _ = request.await;
            }
            return;
        }
        _ => return,
    };

    let recipients = recipient_bindings(chat_bindings, event_name, target_user_id);
    send_to_bindings(
        bot,
        &recipients,
        &text,
        reply_contexts,
        reply_context.as_ref(),
    )
    .await;
}

fn event_enabled(enabled: &[String], event: &str) -> bool {
    enabled.iter().any(|value| value == event)
}

fn recipient_bindings(
    bindings: &[TelegramChatBinding],
    event_name: Option<&str>,
    target_user_id: Option<i32>,
) -> Vec<TelegramChatBinding> {
    bindings
        .iter()
        .filter(|binding| {
            target_user_id.is_none_or(|user_id| binding.user_id == user_id)
                && event_name.is_none_or(|event| event_enabled(&binding.events, event))
        })
        .cloned()
        .collect()
}

fn find_binding<'a>(
    bindings: &'a [TelegramChatBinding],
    chat_id: &str,
    thread_id: Option<i32>,
) -> Option<&'a TelegramChatBinding> {
    bindings
        .iter()
        .find(|binding| binding.chat_id == chat_id && binding.thread_id == thread_id)
        .or_else(|| {
            thread_id.and_then(|_| {
                bindings
                    .iter()
                    .find(|binding| binding.chat_id == chat_id && binding.thread_id.is_none())
            })
        })
}

fn task_notification(
    task_id: i32,
    job_id: Option<i32>,
    is_system: bool,
    status: TaskStatus,
    task_name: Option<&str>,
    output: Option<&str>,
) -> String {
    let marker = if status == TaskStatus::Finished {
        "✅"
    } else {
        "⚠️"
    };
    let kind = if is_system {
        "运维任务"
    } else {
        "脚本任务"
    };
    let display_id = if is_system {
        job_id.unwrap_or(task_id)
    } else {
        task_id
    };
    let mut message = format!(
        "{} *{}完成*\nID: `{}`",
        marker,
        kind,
        escape_tg_markdown(&display_id.to_string())
    );
    if let Some(task_name) = task_name.filter(|name| !name.is_empty()) {
        message.push_str(&format!("\n名称: `{}`", escape_tg_markdown(task_name)));
    }
    message.push_str(&format!(
        "\n状态: `{}`",
        escape_tg_markdown(&format!("{status:?}"))
    ));
    if let Some(output) = output.filter(|output| !output.trim().is_empty()) {
        let escaped = output
            .replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace('$', "\\$");
        message.push_str(&format!(
            "\n\n*输出摘要:*\n```\n{}\n```",
            truncate_chars(&escaped, 800)
        ));
    }
    message
}

fn task_reply_context(
    task_id: i32,
    job_id: Option<i32>,
    is_system: bool,
    status: TaskStatus,
    task_name: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "source": "telegram_notification_reply",
        "event": "task_status_changed",
        "entity": if is_system { "system_job" } else { "task" },
        "task_id": (!is_system).then_some(task_id),
        "job_id": job_id,
        "task_name": task_name,
        "status": format!("{status:?}"),
    })
}

fn truncate_chars(value: &str, limit: usize) -> String {
    let mut result = value.chars().take(limit).collect::<String>();
    if value.chars().count() > limit {
        result.push_str("...");
    }
    result
}

async fn read_latest_log(directory: &std::path::Path, max_bytes: usize) -> Option<String> {
    let mut entries = tokio::fs::read_dir(directory).await.ok()?;
    let mut logs = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "log")
            && let Ok(metadata) = entry.metadata().await
        {
            logs.push((entry.path(), metadata.modified().ok()));
        }
    }
    logs.sort_by(|left, right| right.1.cmp(&left.1));
    let content = tokio::fs::read_to_string(logs.first()?.0.as_path())
        .await
        .ok()?;
    if content.len() <= max_bytes {
        return Some(content);
    }
    let mut start = content.len().saturating_sub(max_bytes);
    while start < content.len() && !content.is_char_boundary(start) {
        start += 1;
    }
    Some(content[start..].to_string())
}

async fn send_to_bindings(
    bot: &Bot,
    bindings: &[TelegramChatBinding],
    text: &str,
    reply_contexts: &NotificationContextStore,
    reply_context: Option<&serde_json::Value>,
) {
    for binding in bindings {
        let mut request = bot
            .send_message(binding.chat_id.clone(), text)
            .parse_mode(ParseMode::MarkdownV2);
        if let Some(thread_id) = binding.thread_id {
            request = request.message_thread_id(ThreadId(MessageId(thread_id)));
        }
        match request.await {
            Ok(message) => {
                if let Some(reply_context) = reply_context {
                    reply_contexts
                        .remember(
                            message.chat.id.to_string(),
                            message.id.0,
                            reply_context.clone(),
                        )
                        .await;
                }
            }
            Err(error) => {
                error!(
                    "Failed to send Telegram notification to {}: {}",
                    binding.chat_id, error
                );
            }
        }
    }
}

impl Drop for TelegramBot {
    fn drop(&mut self) {
        if let Some(shutdown) = self.proxy_shutdown_tx.take() {
            let _ = shutdown.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_json_defaults_to_the_ops_agent_plugin() {
        let config: TelegramBotConfig = serde_json::from_str("{}").expect("parse config");
        assert_eq!(config.agent_plugin_id, "niupanel-private-agents");
    }

    #[test]
    fn new_config_defaults_to_the_ops_agent_plugin() {
        assert_eq!(
            TelegramBotConfig::default().agent_plugin_id,
            "niupanel-private-agents"
        );
    }

    #[test]
    fn truncation_preserves_unicode_boundaries() {
        assert_eq!(truncate_chars("一二三四", 3), "一二三...");
    }

    #[test]
    fn task_reply_context_contains_ids_but_not_output() {
        let context = task_reply_context(7, Some(12), false, TaskStatus::Failed, Some("backup"));
        assert_eq!(context["task_id"], 7);
        assert_eq!(context["job_id"], 12);
        assert_eq!(context["status"], "Failed");
        assert!(context.get("output").is_none());
    }

    #[test]
    fn notification_recipients_respect_user_and_event_bindings() {
        let bindings = vec![
            TelegramChatBinding {
                chat_id: "101".to_string(),
                user_id: 1,
                label: "owner".to_string(),
                events: vec!["failed".to_string()],
                ..TelegramChatBinding::default()
            },
            TelegramChatBinding {
                chat_id: "102".to_string(),
                user_id: 1,
                label: "quiet".to_string(),
                events: vec!["success".to_string()],
                ..TelegramChatBinding::default()
            },
            TelegramChatBinding {
                chat_id: "201".to_string(),
                user_id: 2,
                label: "other user".to_string(),
                events: vec!["failed".to_string()],
                ..TelegramChatBinding::default()
            },
        ];

        assert_eq!(
            recipient_bindings(&bindings, Some("failed"), Some(1))
                .into_iter()
                .map(|binding| binding.chat_id)
                .collect::<Vec<_>>(),
            vec!["101"]
        );
        assert_eq!(
            recipient_bindings(&bindings, Some("failed"), None)
                .into_iter()
                .map(|binding| binding.chat_id)
                .collect::<Vec<_>>(),
            vec!["101", "201"]
        );
        assert_eq!(
            recipient_bindings(&bindings, None, Some(1))
                .into_iter()
                .map(|binding| binding.chat_id)
                .collect::<Vec<_>>(),
            vec!["101", "102"]
        );
    }

    #[test]
    fn topic_binding_overrides_generic_chat_binding() {
        let bindings = vec![
            TelegramChatBinding {
                chat_id: "-1001".to_string(),
                label: "generic".to_string(),
                ..TelegramChatBinding::default()
            },
            TelegramChatBinding {
                chat_id: "-1001".to_string(),
                thread_id: Some(77),
                label: "topic".to_string(),
                ..TelegramChatBinding::default()
            },
        ];
        assert_eq!(
            find_binding(&bindings, "-1001", Some(77)).unwrap().label,
            "topic"
        );
        assert_eq!(
            find_binding(&bindings, "-1001", Some(88)).unwrap().label,
            "generic"
        );
        assert!(find_binding(&bindings, "-2002", Some(77)).is_none());
    }
}
