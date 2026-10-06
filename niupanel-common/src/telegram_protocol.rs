//! Wire contract between the panel host and the Agent plugin's Telegram channel.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub const LEGACY_TELEGRAM_PLUGIN_ID: &str = "niupanel-telegram";
pub const TELEGRAM_CAPABILITY: &str = "channel.telegram.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramAgentAttachment {
    pub name: String,
    pub media_type: String,
    pub size: u64,
    pub content: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramAgentRequest {
    pub client_request_id: String,
    pub chat_id: String,
    pub thread_id: Option<i32>,
    pub telegram_user_id: String,
    pub user_id: i32,
    pub text: String,
    pub attachments: Vec<TelegramAgentAttachment>,
    pub action: TelegramAgentAction,
    pub panel_context: Value,
    #[serde(skip, default = "default_progress")]
    pub progress: watch::Sender<TelegramAgentProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramAgentResponse {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelegramAgentAction {
    Chat,
    ResetSession,
    EnableYolo,
    DisableYolo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelegramAgentProgress {
    Queued,
    ReadingAttachment,
    WaitingForSession,
    WaitingForCapacity,
    Running,
    CallingTool(String),
    Finalizing,
}

impl TelegramAgentProgress {
    pub fn message(&self) -> String {
        match self {
            Self::Queued => "Ops Agent 已接收，正在排队…".to_string(),
            Self::ReadingAttachment => "正在安全读取文本附件…".to_string(),
            Self::WaitingForSession => "正在等待当前会话中的上一条消息完成…".to_string(),
            Self::WaitingForCapacity => "正在等待可用的 Agent 运行槽位…".to_string(),
            Self::Running => "Ops Agent 正在分析并核实面板状态…".to_string(),
            Self::CallingTool(tool) => format!("正在调用面板工具：{tool}"),
            Self::Finalizing => "诊断完成，正在整理结果…".to_string(),
        }
    }
}

impl TelegramAgentRequest {
    pub fn session_key(&self) -> String {
        telegram_agent_session_key(
            self.user_id,
            &self.chat_id,
            self.thread_id,
            &self.telegram_user_id,
        )
    }

    pub fn report_progress(&self, progress: TelegramAgentProgress) {
        self.progress.send_replace(progress);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelegramAgentError {
    Cancelled,
    Failed(String),
}

impl std::fmt::Display for TelegramAgentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("Agent run cancelled"),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

pub type TelegramAgentFuture =
    Pin<Box<dyn Future<Output = Result<TelegramAgentResponse, TelegramAgentError>> + Send>>;
pub type TelegramAgentHandler =
    Arc<dyn Fn(TelegramAgentRequest, CancellationToken) -> TelegramAgentFuture + Send + Sync>;

fn telegram_agent_session_key(
    user_id: i32,
    chat_id: &str,
    thread_id: Option<i32>,
    telegram_user_id: &str,
) -> String {
    format!(
        "{}:{}:{}:{}",
        user_id,
        chat_id,
        thread_id.unwrap_or(0),
        telegram_user_id
    )
}

fn default_progress() -> watch::Sender<TelegramAgentProgress> {
    watch::channel(TelegramAgentProgress::Queued).0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TelegramNotification {
    Task {
        task_id: i32,
        job_id: Option<i32>,
        is_system: bool,
        status: String,
        task_name: Option<String>,
        user_id: Option<i32>,
        output: Option<String>,
    },
    Alert {
        message: String,
    },
    Notification {
        title: String,
        content: String,
        level: String,
    },
    Message {
        chat_id: String,
        thread_id: Option<i32>,
        text: Option<String>,
        file: Option<TelegramOutboundFile>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramOutboundFile {
    pub content_base64: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramAgentUpdate {
    pub id: String,
    pub progress: TelegramAgentProgress,
    pub result: Option<Result<TelegramAgentResponse, TelegramAgentError>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TelegramTickInput {
    pub config: Option<TelegramBotConfig>,
    #[serde(default)]
    pub legacy_ledger: Option<String>,
    #[serde(default)]
    pub notifications: Vec<TelegramNotification>,
    #[serde(default)]
    pub updates: Vec<TelegramAgentUpdate>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TelegramTickOutput {
    pub requests: Vec<TelegramAgentRequest>,
    pub cancelled: Vec<String>,
    pub running: bool,
}

impl TelegramBotConfig {
    /// The host derives the panel identity from its own binding, never from plugin input.
    pub fn bound_user_id(&self, request: &TelegramAgentRequest) -> Option<i32> {
        let binding = self
            .chat_bindings
            .iter()
            .find(|b| b.chat_id == request.chat_id && b.thread_id == request.thread_id)
            .or_else(|| {
                self.chat_bindings
                    .iter()
                    .find(|b| b.chat_id == request.chat_id && b.thread_id.is_none())
            })?;
        let allowed = if binding.telegram_user_ids.is_empty() {
            !binding.chat_id.starts_with('-') && binding.chat_id == request.telegram_user_id
        } else {
            binding
                .telegram_user_ids
                .contains(&request.telegram_user_id)
        };
        allowed.then_some(binding.user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(chat: &str, topic: Option<i32>, sender: &str) -> TelegramAgentRequest {
        TelegramAgentRequest {
            client_request_id: "test-1".to_string(),
            chat_id: chat.to_string(),
            thread_id: topic,
            telegram_user_id: sender.to_string(),
            user_id: 999,
            text: "hello".to_string(),
            attachments: Vec::new(),
            action: TelegramAgentAction::Chat,
            panel_context: Value::Null,
            progress: default_progress(),
        }
    }

    #[test]
    fn derives_identity_from_host_binding_and_checks_senders() {
        let config = TelegramBotConfig {
            chat_bindings: vec![TelegramChatBinding {
                chat_id: "42".to_string(),
                user_id: 7,
                ..Default::default()
            }],
            ..Default::default()
        };
        assert_eq!(config.bound_user_id(&request("42", None, "42")), Some(7));
        assert_eq!(config.bound_user_id(&request("42", None, "99")), None);
        assert_eq!(config.bound_user_id(&request("99", None, "99")), None);
    }

    #[test]
    fn topic_binding_does_not_fall_back_after_sender_rejection() {
        let config = TelegramBotConfig {
            chat_bindings: vec![
                TelegramChatBinding {
                    chat_id: "-42".into(),
                    user_id: 7,
                    telegram_user_ids: vec!["42".into()],
                    ..Default::default()
                },
                TelegramChatBinding {
                    chat_id: "-42".into(),
                    thread_id: Some(12),
                    user_id: 8,
                    telegram_user_ids: vec!["99".into()],
                    ..Default::default()
                },
                TelegramChatBinding {
                    chat_id: "-99".into(),
                    user_id: 9,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            config.bound_user_id(&request("-42", Some(12), "99")),
            Some(8)
        );
        assert_eq!(config.bound_user_id(&request("-42", Some(12), "42")), None);
        assert_eq!(
            config.bound_user_id(&request("-42", Some(13), "42")),
            Some(7)
        );
        assert_eq!(config.bound_user_id(&request("-99", None, "99")), None);
    }

    #[test]
    fn request_round_trips_without_serializing_progress_channel() {
        let source = request("42", Some(3), "42");
        let json = serde_json::to_value(&source).unwrap();
        assert!(json.get("progress").is_none());
        let decoded: TelegramAgentRequest = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.session_key(), source.session_key());
        assert_eq!(decoded.action, TelegramAgentAction::Chat);
    }
}
