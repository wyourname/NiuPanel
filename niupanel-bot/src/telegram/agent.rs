use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::context::NotificationContextStore;
use super::ledger::{LedgerBegin, TelegramMessageLedger};
use super::runtime::TelegramAgentRuntime;
use super::{TelegramChatBinding, find_binding};
use futures::StreamExt;
use niupanel_common::logger::warn;
use serde::Serialize;
use serde_json::Value;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::{BotCommand, ChatAction, Document, Me, MessageId, ReplyParameters, ThreadId};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

const TELEGRAM_MESSAGE_LIMIT: usize = 3_900;
const MAX_ATTACHMENT_BYTES: usize = 512 * 1024;
const MAX_ATTACHMENT_CHARS: usize = 120_000;

#[derive(Debug, Clone, Serialize)]
pub struct TelegramAgentAttachment {
    pub name: String,
    pub media_type: String,
    pub size: u64,
    pub content: String,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
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
    pub progress: watch::Sender<TelegramAgentProgress>,
}

#[derive(Debug, Clone)]
pub struct TelegramAgentResponse {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelegramAgentAction {
    Chat,
    ResetSession,
    EnableYolo,
    DisableYolo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    fn message(&self) -> String {
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub fn bot_commands() -> Vec<BotCommand> {
    vec![
        BotCommand::new("start", "打开 Ops Agent"),
        BotCommand::new("new", "清空当前 Agent 会话"),
        BotCommand::new("cancel", "停止当前 Agent 运行"),
        BotCommand::new("mode", "切换当前会话审批模式"),
    ]
}

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    chat_bindings: Vec<TelegramChatBinding>,
    bot_identity: Me,
    agent_runtime: TelegramAgentRuntime,
    message_ledger: TelegramMessageLedger,
    reply_contexts: NotificationContextStore,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let chat_id_string = chat_id.to_string();
    let thread_id = msg.thread_id.map(|thread_id| thread_id.0.0);
    let Some(sender) = msg.from.as_ref() else {
        return Ok(());
    };
    let telegram_user_id = sender.id.0.to_string();
    // Telegram stores document questions in caption rather than text.
    let text = msg
        .text()
        .or_else(|| msg.caption())
        .map(str::trim)
        .filter(|text| !text.is_empty());
    let command = text
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .split('@')
        .next()
        .unwrap_or_default();
    let identity = format!(
        "Chat ID: `{}`\nTopic ID: `{}`\nTelegram User ID: `{}`",
        chat_id,
        thread_id
            .map(|thread_id| thread_id.to_string())
            .unwrap_or_else(|| "无".to_string()),
        telegram_user_id
    );
    let Some(binding) = find_binding(&chat_bindings, &chat_id_string, thread_id) else {
        if command == "/start" || msg.chat.is_private() {
            let mut request =
                bot.send_message(chat_id, format!("当前会话尚未授权。\n\n{identity}"));
            if let Some(thread_id) = thread_id {
                request = request.message_thread_id(ThreadId(MessageId(thread_id)));
            }
            request.await?;
        }
        return Ok(());
    };
    if !sender_authorized(binding, &chat_id_string, &telegram_user_id) {
        if command == "/start" || msg.chat.is_private() {
            let mut request =
                bot.send_message(chat_id, format!("当前发送者尚未授权。\n\n{identity}"));
            if let Some(thread_id) = thread_id {
                request = request.message_thread_id(ThreadId(MessageId(thread_id)));
            }
            request.await?;
        }
        return Ok(());
    }

    let is_reply_to_bot = msg
        .reply_to_message()
        .and_then(|message| message.from.as_ref())
        .is_some_and(|user| user.id == bot_identity.id);
    if !message_targets_agent(
        msg.chat.is_private(),
        &binding.interaction_mode,
        command,
        text.unwrap_or_default(),
        is_reply_to_bot,
        bot_identity.username(),
    ) {
        return Ok(());
    }

    let ledger_key = format!("{}:{}:{}", bot_identity.id.0, chat_id, msg.id.0);
    match message_ledger.begin(&ledger_key).await {
        Ok(LedgerBegin::Started) => {}
        Ok(LedgerBegin::Duplicate) => return Ok(()),
        Ok(LedgerBegin::Interrupted) => {
            send_thread_reply(
                &bot,
                chat_id,
                thread_id,
                msg.id,
                "这条消息在上次运行中被中断，已阻止重复执行。请重新发送一次以继续。",
            )
            .await?;
            return Ok(());
        }
        Err(error) => {
            send_thread_reply(
                &bot,
                chat_id,
                thread_id,
                msg.id,
                format!("Telegram 消息幂等状态不可用，已停止处理：{error}"),
            )
            .await?;
            return Ok(());
        }
    }

    if command == "/start" {
        let mut request = bot.send_message(
            chat_id,
            format!(
                "NiuPanel Ops Agent 已连接。直接描述目标，或回复任务通知与系统告警继续诊断。使用 /new 清空会话和待确认操作。\n\n{identity}"
            ),
        );
        if let Some(thread_id) = thread_id {
            request = request.message_thread_id(ThreadId(MessageId(thread_id)));
        }
        request.await?;
        complete_ledger(&message_ledger, &ledger_key).await;
        return Ok(());
    }

    let session_key = telegram_agent_session_key(
        binding.user_id,
        &chat_id_string,
        thread_id,
        &telegram_user_id,
    );
    if cancellation_request(command, text.unwrap_or_default()) {
        let cancelled = agent_runtime.cancel_session(&session_key);
        let response = if cancelled == 0 {
            "当前会话没有正在运行或排队的 Agent 请求。".to_string()
        } else {
            format!(
                "已停止当前会话中的 {cancelled} 个 Agent 请求。已确认执行的面板操作不会被回滚。"
            )
        };
        send_thread_reply(&bot, chat_id, thread_id, msg.id, response).await?;
        complete_ledger(&message_ledger, &ledger_key).await;
        return Ok(());
    }

    let reset = command == "/new";
    let mode_action = if command == "/mode" {
        match text
            .unwrap_or_default()
            .split_whitespace()
            .nth(1)
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("yolo") => Some(TelegramAgentAction::EnableYolo),
            Some("normal" | "safe") => Some(TelegramAgentAction::DisableYolo),
            _ => {
                send_thread_reply(
                    &bot,
                    chat_id,
                    thread_id,
                    msg.id,
                    "用法：/mode yolo 或 /mode normal",
                )
                .await?;
                complete_ledger(&message_ledger, &ledger_key).await;
                return Ok(());
            }
        }
    } else {
        None
    };
    if reset {
        agent_runtime.cancel_session(&session_key);
    }
    let initial_progress = if msg.document().is_some() {
        TelegramAgentProgress::ReadingAttachment
    } else {
        TelegramAgentProgress::Queued
    };
    let (progress, progress_updates) = watch::channel(initial_progress.clone());
    let progress_message =
        send_thread_reply(&bot, chat_id, thread_id, msg.id, initial_progress.message()).await?;
    let progress_task = tokio::spawn(update_progress_message(
        bot.clone(),
        chat_id,
        progress_message.id,
        progress_updates,
    ));
    let attachments = if reset {
        Vec::new()
    } else if let Some(document) = msg.document() {
        match download_text_attachment(&bot, document).await {
            Ok(attachment) => vec![attachment],
            Err(error) => {
                progress_task.abort();
                edit_progress_or_send(
                    &bot,
                    chat_id,
                    thread_id,
                    progress_message.id,
                    msg.id,
                    format!("无法分析附件：{error}"),
                )
                .await?;
                complete_ledger(&message_ledger, &ledger_key).await;
                return Ok(());
            }
        }
    } else {
        Vec::new()
    };
    let text = if reset {
        String::new()
    } else if msg.document().is_none() && msg.text().is_none() {
        progress_task.abort();
        edit_progress_or_send(
            &bot,
            chat_id,
            thread_id,
            progress_message.id,
            msg.id,
            "当前仅支持文本消息和 UTF-8 文本文档，暂不支持图片、音频或视频分析。",
        )
        .await?;
        complete_ledger(&message_ledger, &ledger_key).await;
        return Ok(());
    } else if let Some(text) = text {
        text.to_string()
    } else if attachments.is_empty() {
        progress_task.abort();
        edit_progress_or_send(
            &bot,
            chat_id,
            thread_id,
            progress_message.id,
            msg.id,
            "当前支持文本消息，以及日志、配置和常见代码等 UTF-8 文本附件。",
        )
        .await?;
        complete_ledger(&message_ledger, &ledger_key).await;
        return Ok(());
    } else {
        "请分析这个附件。".to_string()
    };
    let actor_context = serde_json::json!({
        "user_id": binding.user_id,
        "binding_label": binding.label,
        "telegram_user_id": telegram_user_id,
        "chat_id": chat_id_string,
        "thread_id": thread_id,
    });
    let panel_context = if let Some(replied_to) = msg.reply_to_message() {
        match reply_contexts
            .get(&chat_id.to_string(), replied_to.id.0)
            .await
        {
            Some(context) => serde_json::json!({
                "channel": "telegram",
                "actor": actor_context,
                "notification_reply": context,
            }),
            None => serde_json::json!({
                "channel": "telegram",
                "actor": actor_context,
            }),
        }
    } else {
        serde_json::json!({
            "channel": "telegram",
            "actor": actor_context,
        })
    };
    let request = TelegramAgentRequest {
        client_request_id: ledger_key.clone(),
        chat_id: chat_id.to_string(),
        thread_id,
        telegram_user_id,
        user_id: binding.user_id,
        text,
        attachments,
        action: mode_action.unwrap_or(if reset {
            TelegramAgentAction::ResetSession
        } else {
            TelegramAgentAction::Chat
        }),
        panel_context,
        progress,
    };

    let typing_bot = bot.clone();
    let typing_task = tokio::spawn(async move {
        loop {
            let mut request = typing_bot.send_chat_action(chat_id, ChatAction::Typing);
            if let Some(thread_id) = thread_id {
                request = request.message_thread_id(ThreadId(MessageId(thread_id)));
            }
            if request.await.is_err() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        }
    });
    let result = agent_runtime.invoke(request).await;
    typing_task.abort();
    progress_task.abort();

    match result {
        Ok(response) => {
            let chunks = split_message(&response.text, TELEGRAM_MESSAGE_LIMIT);
            edit_progress_or_send(
                &bot,
                chat_id,
                thread_id,
                progress_message.id,
                msg.id,
                chunks[0].clone(),
            )
            .await?;
            for chunk in chunks.into_iter().skip(1) {
                send_thread_message(&bot, chat_id, thread_id, chunk).await?;
            }
        }
        Err(TelegramAgentError::Cancelled) => {
            edit_progress_or_send(
                &bot,
                chat_id,
                thread_id,
                progress_message.id,
                msg.id,
                "当前 Agent 运行已停止。已确认执行的面板操作不会被回滚。",
            )
            .await?;
        }
        Err(TelegramAgentError::Failed(error)) => {
            let error = error.chars().take(1_000).collect::<String>();
            edit_progress_or_send(
                &bot,
                chat_id,
                thread_id,
                progress_message.id,
                msg.id,
                format!("Ops Agent 暂时不可用：{error}"),
            )
            .await?;
        }
    }
    complete_ledger(&message_ledger, &ledger_key).await;
    Ok(())
}

async fn update_progress_message(
    bot: Bot,
    chat_id: ChatId,
    message_id: MessageId,
    mut progress: watch::Receiver<TelegramAgentProgress>,
) {
    let mut last_text = progress.borrow().message();
    while progress.changed().await.is_ok() {
        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
        let text = progress.borrow_and_update().message();
        if text == last_text {
            continue;
        }
        if bot
            .edit_message_text(chat_id, message_id, text.clone())
            .await
            .is_ok()
        {
            last_text = text;
        }
    }
}

async fn send_thread_reply(
    bot: &Bot,
    chat_id: ChatId,
    thread_id: Option<i32>,
    reply_to: MessageId,
    text: impl Into<String>,
) -> ResponseResult<Message> {
    let mut request = bot
        .send_message(chat_id, text)
        .reply_parameters(ReplyParameters::new(reply_to).allow_sending_without_reply());
    if let Some(thread_id) = thread_id {
        request = request.message_thread_id(ThreadId(MessageId(thread_id)));
    }
    request.await
}

async fn send_thread_message(
    bot: &Bot,
    chat_id: ChatId,
    thread_id: Option<i32>,
    text: impl Into<String>,
) -> ResponseResult<Message> {
    let mut request = bot.send_message(chat_id, text);
    if let Some(thread_id) = thread_id {
        request = request.message_thread_id(ThreadId(MessageId(thread_id)));
    }
    request.await
}

async fn edit_progress_or_send(
    bot: &Bot,
    chat_id: ChatId,
    thread_id: Option<i32>,
    progress_message_id: MessageId,
    reply_to: MessageId,
    text: impl Into<String>,
) -> ResponseResult<()> {
    let text = text.into();
    if bot
        .edit_message_text(chat_id, progress_message_id, text.clone())
        .await
        .is_err()
    {
        send_thread_reply(bot, chat_id, thread_id, reply_to, text).await?;
    }
    Ok(())
}

async fn complete_ledger(ledger: &TelegramMessageLedger, key: &str) {
    if let Err(error) = ledger.complete(key).await {
        warn!("Telegram message ledger completion failed: {}", error);
    }
}

async fn download_text_attachment(
    bot: &Bot,
    document: &Document,
) -> Result<TelegramAgentAttachment, String> {
    let name = document
        .file_name
        .as_deref()
        .unwrap_or("attachment.txt")
        .trim();
    let media_type = document
        .mime_type
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| media_type_from_name(name).to_string());
    validate_attachment_metadata(name, &media_type, document.file.size as usize)?;

    let file = bot
        .get_file(document.file.id.clone())
        .await
        .map_err(|error| format!("读取 Telegram 文件信息失败：{error}"))?;
    validate_attachment_metadata(name, &media_type, file.size as usize)?;

    let mut bytes = Vec::with_capacity((file.size as usize).min(MAX_ATTACHMENT_BYTES));
    let mut stream = bot.download_file_stream(&file.path);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("从 Telegram 下载文件失败：{error}"))?;
        if bytes.len().saturating_add(chunk.len()) > MAX_ATTACHMENT_BYTES {
            return Err(format!("文件超过 {} KiB 上限", MAX_ATTACHMENT_BYTES / 1024));
        }
        bytes.extend_from_slice(&chunk);
    }

    let size = bytes.len() as u64;
    let (content, truncated) = decode_attachment_content(&bytes)?;
    Ok(TelegramAgentAttachment {
        name: name.to_string(),
        media_type,
        size,
        content,
        truncated,
    })
}

fn validate_attachment_metadata(name: &str, media_type: &str, size: usize) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 255
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err("文件名无效".to_string());
    }
    if sensitive_attachment_name(name) {
        return Err("出于安全考虑，不接受凭据、密钥或环境变量文件".to_string());
    }
    if size > MAX_ATTACHMENT_BYTES {
        return Err(format!("文件超过 {} KiB 上限", MAX_ATTACHMENT_BYTES / 1024));
    }
    if !supported_text_attachment(name, media_type) {
        return Err("仅支持日志、JSON/YAML/TOML、Markdown 和常见代码文本文件".to_string());
    }
    Ok(())
}

fn decode_attachment_content(bytes: &[u8]) -> Result<(String, bool), String> {
    if bytes.contains(&0) {
        return Err("文件包含二进制 NUL 字节".to_string());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "文件不是有效的 UTF-8 文本".to_string())?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let truncated = text.chars().count() > MAX_ATTACHMENT_CHARS;
    let content = if truncated {
        text.chars().take(MAX_ATTACHMENT_CHARS).collect()
    } else {
        text.to_string()
    };
    Ok((content, truncated))
}

fn sensitive_attachment_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let extension = name.rsplit_once('.').map(|(_, extension)| extension);
    name == ".env"
        || name.starts_with(".env.")
        || matches!(extension, Some("env" | "pem" | "key" | "p12" | "pfx"))
        || [
            "id_rsa",
            "id_ed25519",
            "credential",
            "credentials",
            "secret",
            "secrets",
        ]
        .iter()
        .any(|marker| name.contains(marker))
}

fn supported_text_attachment(name: &str, media_type: &str) -> bool {
    let lower_name = name.to_ascii_lowercase();
    let extension = lower_name.rsplit_once('.').map(|(_, extension)| extension);
    let known_name = matches!(
        lower_name.as_str(),
        "dockerfile" | "makefile" | "justfile" | "cargo.lock" | "gemfile" | "procfile"
    );
    let known_extension = extension.is_some_and(|extension| {
        matches!(
            extension,
            "txt"
                | "log"
                | "json"
                | "jsonl"
                | "yaml"
                | "yml"
                | "toml"
                | "md"
                | "markdown"
                | "csv"
                | "tsv"
                | "xml"
                | "html"
                | "htm"
                | "css"
                | "scss"
                | "less"
                | "js"
                | "mjs"
                | "cjs"
                | "jsx"
                | "ts"
                | "tsx"
                | "vue"
                | "rs"
                | "py"
                | "go"
                | "java"
                | "kt"
                | "kts"
                | "c"
                | "cc"
                | "cpp"
                | "cxx"
                | "h"
                | "hpp"
                | "cs"
                | "php"
                | "rb"
                | "sh"
                | "bash"
                | "zsh"
                | "fish"
                | "ps1"
                | "sql"
                | "ini"
                | "cfg"
                | "conf"
                | "properties"
                | "service"
        )
    });
    let media_type = media_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let known_media_type = media_type.starts_with("text/")
        || matches!(
            media_type.as_str(),
            "application/json"
                | "application/ld+json"
                | "application/x-ndjson"
                | "application/yaml"
                | "application/x-yaml"
                | "application/toml"
                | "application/xml"
                | "application/javascript"
        );
    known_name || known_extension || known_media_type
}

fn media_type_from_name(name: &str) -> &'static str {
    match name
        .to_ascii_lowercase()
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_string())
        .as_deref()
    {
        Some("json" | "jsonl") => "application/json",
        Some("yaml" | "yml") => "application/yaml",
        Some("toml") => "application/toml",
        Some("xml") => "application/xml",
        _ => "text/plain",
    }
}

fn sender_authorized(binding: &TelegramChatBinding, chat_id: &str, telegram_user_id: &str) -> bool {
    if binding.telegram_user_ids.is_empty() {
        let chat_id = chat_id.parse::<i64>().ok().filter(|chat_id| *chat_id > 0);
        return chat_id == telegram_user_id.parse::<i64>().ok();
    }
    binding
        .telegram_user_ids
        .iter()
        .any(|allowed| allowed == telegram_user_id)
}

fn message_targets_agent(
    is_private: bool,
    interaction_mode: &str,
    command: &str,
    text: &str,
    is_reply_to_bot: bool,
    bot_username: &str,
) -> bool {
    if is_private
        || interaction_mode == "direct"
        || matches!(command, "/start" | "/new" | "/cancel")
    {
        return true;
    }
    is_reply_to_bot
        || text
            .to_ascii_lowercase()
            .contains(&format!("@{}", bot_username.to_ascii_lowercase()))
}

fn cancellation_request(command: &str, text: &str) -> bool {
    if command == "/cancel" {
        return true;
    }
    matches!(
        text.trim().to_ascii_lowercase().as_str(),
        "停止" | "取消" | "停止诊断" | "取消诊断" | "stop" | "cancel"
    )
}

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

fn split_message(text: &str, limit: usize) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return vec!["Ops Agent 未返回内容。".to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;
    while remaining.chars().count() > limit {
        let boundary = remaining
            .char_indices()
            .take_while(|(index, _)| *index <= limit * 4)
            .filter(|(_, ch)| *ch == '\n' || ch.is_whitespace())
            .map(|(index, _)| index)
            .take_while(|index| remaining[..*index].chars().count() <= limit)
            .last()
            .unwrap_or_else(|| {
                remaining
                    .char_indices()
                    .nth(limit)
                    .map(|(index, _)| index)
                    .unwrap_or(remaining.len())
            });
        let (chunk, rest) = remaining.split_at(boundary);
        chunks.push(chunk.trim().to_string());
        remaining = rest.trim_start();
    }
    if !remaining.is_empty() {
        chunks.push(remaining.to_string());
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_ATTACHMENT_CHARS, cancellation_request, decode_attachment_content,
        message_targets_agent, sender_authorized, split_message, validate_attachment_metadata,
    };
    use crate::telegram::TelegramChatBinding;

    #[test]
    fn splits_unicode_without_breaking_character_boundaries() {
        let chunks = split_message("诊断结果：一二三四五六七八九十", 6);
        assert!(chunks.iter().all(|chunk| chunk.chars().count() <= 6));
        assert_eq!(chunks.concat(), "诊断结果：一二三四五六七八九十");
    }

    #[test]
    fn replaces_empty_agent_responses() {
        assert_eq!(split_message("  ", 10), vec!["Ops Agent 未返回内容。"]);
    }

    #[test]
    fn private_chat_implicitly_authorizes_only_its_owner() {
        let binding = TelegramChatBinding {
            chat_id: "42".to_string(),
            ..TelegramChatBinding::default()
        };
        assert!(sender_authorized(&binding, "42", "42"));
        assert!(!sender_authorized(&binding, "42", "7"));
        assert!(!sender_authorized(&binding, "-42", "42"));
    }

    #[test]
    fn group_chat_requires_an_explicit_sender_allowlist() {
        let mut binding = TelegramChatBinding {
            chat_id: "-10042".to_string(),
            ..TelegramChatBinding::default()
        };
        assert!(!sender_authorized(&binding, "-10042", "7"));
        binding.telegram_user_ids = vec!["7".to_string()];
        assert!(sender_authorized(&binding, "-10042", "7"));
        assert!(!sender_authorized(&binding, "-10042", "8"));
    }

    #[test]
    fn mention_mode_accepts_mentions_replies_and_control_commands() {
        assert!(!message_targets_agent(
            false,
            "mention",
            "check",
            "检查任务",
            false,
            "NiuPanelBot",
        ));
        assert!(message_targets_agent(
            false,
            "mention",
            "check",
            "@niupanelbot 检查任务",
            false,
            "NiuPanelBot",
        ));
        assert!(message_targets_agent(
            false,
            "mention",
            "check",
            "继续",
            true,
            "NiuPanelBot",
        ));
        assert!(message_targets_agent(
            false,
            "mention",
            "/new",
            "/new",
            false,
            "NiuPanelBot",
        ));
    }

    #[test]
    fn mention_mode_uses_a_document_caption_or_bot_reply() {
        assert!(message_targets_agent(
            false,
            "mention",
            "@niupanelbot",
            "@niupanelbot 分析日志",
            false,
            "NiuPanelBot",
        ));
        assert!(!message_targets_agent(
            false,
            "mention",
            "",
            "",
            false,
            "NiuPanelBot",
        ));
        assert!(message_targets_agent(
            false,
            "mention",
            "",
            "",
            true,
            "NiuPanelBot",
        ));
    }

    #[test]
    fn cancellation_only_intercepts_explicit_control_messages() {
        assert!(cancellation_request("/cancel", "/cancel@NiuPanelBot"));
        assert!(cancellation_request("", "停止诊断"));
        assert!(cancellation_request("", "cancel"));
        assert!(!cancellation_request("", "停止任务 42"));
        assert!(!cancellation_request("", "分析取消失败的任务"));
    }

    #[test]
    fn rejects_sensitive_oversized_and_binary_attachments() {
        assert!(validate_attachment_metadata(".env", "text/plain", 10).is_err());
        assert!(validate_attachment_metadata("id_rsa", "text/plain", 10).is_err());
        assert!(validate_attachment_metadata("task.log", "text/plain", 512 * 1024 + 1).is_err());
        assert!(decode_attachment_content(b"first\0second").is_err());
        assert!(decode_attachment_content(&[0xff, 0xfe]).is_err());
    }

    #[test]
    fn truncates_utf8_attachments_on_character_boundaries() {
        let text = "诊".repeat(MAX_ATTACHMENT_CHARS + 1);
        let (content, truncated) = decode_attachment_content(text.as_bytes()).expect("decode");
        assert!(truncated);
        assert_eq!(content.chars().count(), MAX_ATTACHMENT_CHARS);
        assert!(content.is_char_boundary(content.len()));
    }
}
