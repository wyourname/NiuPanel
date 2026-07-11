use niupanel_common::escape_tg_markdown;
use niupanel_core::event_bus::{EventBus, SystemEvent, TelegramEvent};
use niupanel_entity::{tasks, tg_commands, tg_workflows};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

/// Parse a task identifier (either numeric ID or task name)
pub async fn parse_task_id(arg: &str, db: &DatabaseConnection) -> Option<i32> {
    let trimmed = arg.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(id) = trimmed.parse::<i32>() {
        return Some(id);
    }

    if let Ok(Some(task)) = tasks::Entity::find()
        .filter(tasks::Column::Name.eq(trimmed))
        .one(db)
        .await
    {
        return Some(task.id);
    }

    None
}

/// Publish an audit trail event to EventBus for frontend log stream
pub fn publish_audit(event_bus: &EventBus, chat_id: String, text: String) {
    event_bus.publish(SystemEvent::Telegram(TelegramEvent::MessageReceived {
        chat_id,
        username: Some("Robot".to_string()),
        text: Some(text),
        file_name: None,
        file_id: None,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }));
}

pub async fn execute_custom_command(
    bot: &Bot,
    chat_id: ChatId,
    _db: &DatabaseConnection,
    event_bus: &EventBus,
    cmd: tg_commands::Model,
) {
    let _ = bot
        .send_message(
            chat_id,
            format!(
                "正在执行指令: `{}`{}",
                escape_tg_markdown(&cmd.name),
                escape_tg_markdown("...")
            ),
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let script_path = format!("/tmp/tg_cmd_{}_{}.sh", cmd.id, timestamp);
    let _ = tokio::fs::write(&script_path, &cmd.script).await;

    let output = tokio::process::Command::new("bash")
        .arg(&script_path)
        .output()
        .await;

    let _ = tokio::fs::remove_file(&script_path).await;

    match output {
        Ok(out) => {
            let mut result_text = String::from_utf8_lossy(&out.stdout).to_string();
            if !out.stderr.is_empty() {
                result_text.push_str("\n--- 错误输出 ---\n");
                result_text.push_str(&String::from_utf8_lossy(&out.stderr));
            }
            if result_text.trim().is_empty() {
                result_text = "执行完成，无输出".to_string();
            }
            let _ = bot
                .send_message(chat_id, format!("```\n{}\n```", result_text))
                .parse_mode(ParseMode::MarkdownV2)
                .await;
            publish_audit(
                event_bus,
                chat_id.to_string(),
                format!("执行自定义指令: {}", cmd.name),
            );
        }
        Err(e) => {
            let _ = bot.send_message(chat_id, format!("执行失败: {}", e)).await;
        }
    }
}

/// Execute a Workflow action (including Approval requests)
pub async fn execute_workflow(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    event_bus: &EventBus,
    workflow: tg_workflows::Model,
    task_id_context: Option<i32>,
) {
    if workflow.action_type == "approval" {
        // Parse config_json to get the message and the script
        let config: serde_json::Value =
            serde_json::from_str(&workflow.config_json).unwrap_or(serde_json::json!({}));
        let message = config
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("🚨 请求审批某项系统操作");
        let script = config.get("script").and_then(|v| v.as_str()).unwrap_or("");

        let nonce = generate_nanoid(8);

        // Save the pending script to a temporary file or database so the callback can read it
        // For simplicity and resilience, we'll store it in a local temp file keyed by the nonce
        let pending_script_path = format!("/tmp/tg_pending_approval_{}.sh", nonce);
        let _ = tokio::fs::write(&pending_script_path, script).await;

        let kb = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("✅ 授权执行", format!("wf_approve_{}", nonce)),
            InlineKeyboardButton::callback("❌ 拒绝执行", format!("wf_reject_{}", nonce)),
        ]]);
        let res = bot
            .send_message(chat_id, escape_tg_markdown(message))
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(kb)
            .await;

        if let Err(e) = res {
            niupanel_common::logger::error!("Failed to send approval message: {}", e);
        }

        return;
    }

    // Existing actions like notify, shell, webhook...
    // (We will expand these as well since the base engine was missing)
    if workflow.action_type == "notify" {
        let config: serde_json::Value =
            serde_json::from_str(&workflow.config_json).unwrap_or(serde_json::json!({}));
        let mut msg = config
            .get("template")
            .and_then(|v| v.as_str())
            .unwrap_or("触发了自动化工作流")
            .to_string();
        if let Some(tid) = task_id_context {
            msg = msg.replace("{{task_id}}", &tid.to_string());
        }
        let res = bot
            .send_message(chat_id, escape_tg_markdown(&msg))
            .parse_mode(ParseMode::MarkdownV2)
            .await;
        if let Err(e) = res {
            niupanel_common::logger::error!("Failed to send notify message: {}", e);
        }
    } else if workflow.action_type == "shell" {
        let mut script = workflow.config_json.clone();

        // If config is JSON (like in cron triggers) extract the "script" field, otherwise use raw text
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&script) {
            if let Some(s) = config.get("script").and_then(|v| v.as_str()) {
                script = s.to_string();
            }
        }

        // Execute it via execute_custom_command mechanism
        let dummy_cmd = tg_commands::Model {
            id: Default::default(),
            name: format!("workflow_{}", workflow.id),
            script,
            created_at: Default::default(),
            updated_at: Default::default(),
        };
        execute_custom_command(bot, chat_id, db, event_bus, dummy_cmd).await;
    }
}

/// Find a unique path in the directory. If collision exists, appends (1), (2)...
pub fn get_unique_path(dir: &std::path::Path, base_name: &str, ext: &str) -> std::path::PathBuf {
    let sanitized_name =
        base_name.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");
    let mut attempt = 0;
    loop {
        let filename = if attempt == 0 {
            format!("{}.{}", sanitized_name, ext)
        } else {
            format!("{}({}).{}", sanitized_name, attempt, ext)
        };
        let path = dir.join(filename);
        if !path.exists() {
            return path;
        }
        attempt += 1;
    }
}

pub fn generate_nanoid(len: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let s = format!("{:x}", ts);
    s.chars().rev().take(len).collect()
}

pub fn format_uptime_cn(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    if days > 0 {
        format!("{}天 {}小时 {}分", days, hours, minutes)
    } else if hours > 0 {
        format!("{}小时 {}分", hours, minutes)
    } else {
        format!("{}分", minutes)
    }
}

pub async fn read_latest_log(log_dir: &str, max_bytes: usize) -> Option<String> {
    let path = std::path::Path::new(log_dir);
    if !path.exists() {
        return None;
    }
    let mut entries = vec![];
    if let Ok(mut rd) = tokio::fs::read_dir(path).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            entries.push(entry);
        }
    } else {
        return None;
    }

    let mut metadata_entries = vec![];
    for entry in entries {
        if let Ok(meta) = entry.metadata().await {
            metadata_entries.push((entry, meta));
        }
    }

    metadata_entries.sort_by(|a, b| {
        let a_t = a.1.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let b_t = b.1.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        b_t.cmp(&a_t)
    });

    let latest = metadata_entries
        .into_iter()
        .find(|e| {
            e.0.path()
                .extension()
                .map(|ext| ext == "log")
                .unwrap_or(false)
        })?
        .0;
    match tokio::fs::read_to_string(latest.path()).await {
        Ok(content) => {
            if content.len() > max_bytes {
                // Find the nearest valid UTF-8 boundary from the end
                let mut start = content.len() - max_bytes;
                while !content.is_char_boundary(start) && start < content.len() {
                    start += 1;
                }
                Some(content[start..].to_string())
            } else {
                Some(content)
            }
        }
        Err(_) => None,
    }
}
