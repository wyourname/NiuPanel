use niupanel_common::escape_tg_markdown;
use niupanel_common::metrics::SystemMetrics;
use niupanel_core::event_bus::EventBus;
use niupanel_core::settings::update::UpdateService;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

use super::deps;
use super::helpers::*;

static LAST_ALERT_TIME: AtomicU64 = AtomicU64::new(0);
const ALERT_COOLDOWN_SECS: u64 = 600;

pub async fn handle_sh(bot: &Bot, chat_id: ChatId, eb: &EventBus, cmd: &str) {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        let _ = bot.send_message(chat_id, "ℹ️ 用法: `/sh <命令>`").await;
        return;
    }

    let _ = bot
        .send_message(chat_id, format!("⏳ 执行: `{}`", escape_tg_markdown(cmd)))
        .parse_mode(ParseMode::MarkdownV2)
        .await;

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output(),
    )
    .await;

    let result = match output {
        Ok(Ok(out)) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let mut combined = String::new();
            if !stdout.is_empty() {
                combined.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !combined.is_empty() {
                    combined.push('\n');
                }
                combined.push_str("[stderr]\n");
                combined.push_str(&stderr);
            }
            if combined.is_empty() {
                combined = "(无输出)".to_string();
            }
            let escaped = combined
                .replace('\\', "\\\\")
                .replace('`', "\\`")
                .replace('$', "\\$");
            let truncated = if escaped.len() > 3500 {
                let mut end = 3500;
                while !escaped.is_char_boundary(end) && end < escaped.len() {
                    end += 1;
                }
                format!("{}...\n(输出已截断)", &escaped[..end])
            } else {
                escaped
            };
            let exit_code = out.status.code().unwrap_or(-1);
            let icon = if out.status.success() {
                "✅"
            } else {
                "⚠️"
            };
            format!(
                "{} *Shell 执行结果* \\(退出码: `{}`\\)\n```\n{}\n```",
                icon, exit_code, truncated
            )
        }
        Ok(Err(e)) => format!("❌ 执行失败: `{}`", escape_tg_markdown(&e.to_string())),
        Err(_) => "⏰ 命令执行超时 \\(30秒\\)".to_string(),
    };

    let _ = bot
        .send_message(chat_id, result)
        .parse_mode(ParseMode::MarkdownV2)
        .await;

    publish_audit(eb, chat_id.to_string(), format!("💻 Shell: {}", cmd));
}

pub async fn dispatch_env_cmd(
    bot: &Bot,
    chat_id: ChatId,
    tm: &niupanel_core::task_manager::service::TaskManagerService,
    eb: &EventBus,
    arg: &str,
) {
    niupanel_common::logger::info!("收到 env 指令: {} 来自 {}", arg, chat_id);
    let parts: Vec<&str> = arg.split_whitespace().collect();
    match parts.first().cloned() {
        Some("list") => deps::handle_envs(bot, chat_id).await,
        Some("pip") => {
            deps::handle_pip(
                bot,
                chat_id,
                tm,
                eb,
                &parts.get(1..).unwrap_or_default().join(" "),
            )
            .await
        }
        Some("npm") => {
            deps::handle_npm(
                bot,
                chat_id,
                tm,
                eb,
                &parts.get(1..).unwrap_or_default().join(" "),
            )
            .await
        }
        Some("apt") => {
            deps::handle_apt(
                bot,
                chat_id,
                tm,
                eb,
                &parts.get(1..).unwrap_or_default().join(" "),
            )
            .await
        }
        _ => {
            let _ = bot
                .send_message(chat_id, "ℹ️ 用法: `/env <list|pip|npm|apt>`")
                .await;
        }
    }
}

pub async fn check_resource_alerts(
    bot: &Bot,
    admin_ids: &[String],
    metrics: &Arc<tokio::sync::RwLock<SystemMetrics>>,
) {
    let m = metrics.read().await;
    let mem_pct = if m.memory_total > 0 {
        m.memory_used as f32 / m.memory_total as f32 * 100.0
    } else {
        0.0
    };

    let mut alerts = vec![];
    if m.cpu_usage > 90.0 {
        alerts.push(format!("🔴 CPU 使用率: `{:.1}%`", m.cpu_usage));
    } else if m.cpu_usage > 75.0 {
        alerts.push(format!("🟠 CPU 使用率: `{:.1}%`", m.cpu_usage));
    }
    if mem_pct > 90.0 {
        alerts.push(format!("🔴 内存使用率: `{:.1}%`", mem_pct));
    } else if mem_pct > 80.0 {
        alerts.push(format!("🟠 内存使用率: `{:.1}%`", mem_pct));
    }

    if alerts.is_empty() {
        LAST_ALERT_TIME.store(0, Ordering::Relaxed);
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let last = LAST_ALERT_TIME.load(Ordering::Relaxed);
    if now - last < ALERT_COOLDOWN_SECS {
        return;
    }
    LAST_ALERT_TIME.store(now, Ordering::Relaxed);

    let msg = format!("🚨 *资源告警*\n\n{}", alerts.join("\n"));
    for id in admin_ids {
        let _ = bot
            .send_message(id.clone(), &msg)
            .parse_mode(ParseMode::MarkdownV2)
            .await;
    }
}

pub async fn dispatch_system_cmd(
    bot: &Bot,
    chat_id: ChatId,
    _db: &sea_orm::DatabaseConnection,
    metrics: &Arc<tokio::sync::RwLock<SystemMetrics>>,
    eb: &EventBus,
    arg: &str,
    us: &UpdateService,
) {
    match arg.trim() {
        "status" => handle_status(bot, chat_id, metrics, eb).await,
        "upgrade" => handle_upgrade(bot, chat_id, us).await,
        _ => {
            let _ = bot
                .send_message(chat_id, "ℹ️ 用法: `/system <status|upgrade>`")
                .await;
        }
    }
}

pub async fn handle_status(
    bot: &Bot,
    chat_id: ChatId,
    m: &Arc<tokio::sync::RwLock<SystemMetrics>>,
    eb: &EventBus,
) {
    let text = {
        let metrics = m.read().await;
        format!(
            "🖥 *系统状态*\n\n*CPU:* `{:.1}%`\n*内存:* `{:.1}GB / {:.1}GB`\n*运行:* `{}`",
            metrics.cpu_usage,
            metrics.memory_used as f32 / 1073741824.0,
            metrics.memory_total as f32 / 1073741824.0,
            format_uptime_cn(metrics.uptime)
        )
    };
    let _ = bot
        .send_message(chat_id, text.clone())
        .parse_mode(ParseMode::MarkdownV2)
        .await;
    publish_audit(eb, chat_id.to_string(), text);
}

pub async fn handle_upgrade(bot: &Bot, chat_id: ChatId, us: &UpdateService) {
    let msg = match bot.send_message(chat_id, "⏳ 正在检查更新...").await {
        Ok(m) => m,
        Err(_) => return,
    };

    match us.check_update().await {
        Ok(info) => {
            let mut text = format!(
                "📦 *系统更新检查*\n\n*当前版本:* `{}`\n*最新版本:* `{}`\n",
                escape_tg_markdown(&info.current_version),
                escape_tg_markdown(&info.tag_name)
            );

            if !info.body.is_empty() {
                let body = if info.body.len() > 500 {
                    format!("{}...", &info.body[..500])
                } else {
                    info.body.clone()
                };
                text.push_str(&format!(
                    "\n*更新说明:*\n```\n{}\n```\n",
                    escape_tg_markdown(&body)
                ));
            }

            let mut kb = vec![];
            if info.update_available {
                text.push_str("\n🆕 发现新版本！是否现在升级？");
                kb.push(vec![
                    InlineKeyboardButton::callback("🚀 立即升级", "upgrade:start"),
                    InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
                ]);
            } else {
                text.push_str("\n✅ 当前已是最新版本。");
                kb.push(vec![
                    InlineKeyboardButton::callback("🔄 重新安装", "upgrade:start"),
                    InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
                ]);
            }

            let _ = bot
                .edit_message_text(chat_id, msg.id, text)
                .parse_mode(ParseMode::MarkdownV2)
                .reply_markup(InlineKeyboardMarkup::new(kb))
                .await;
        }
        Err(e) => {
            let _ = bot
                .edit_message_text(chat_id, msg.id, format!("❌ 检查更新失败: {}", e))
                .await;
        }
    }
}
