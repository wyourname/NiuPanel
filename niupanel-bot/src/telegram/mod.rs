pub mod callbacks;
pub mod commands;
pub mod conversation;
pub mod deps;
pub mod helpers;
pub mod system;
pub mod task;
pub mod var;

use niupanel_common::logger::{debug, error, info, warn};
use niupanel_common::metrics::SystemMetrics;
use niupanel_common::{config::Config, escape_tg_markdown};
use niupanel_core::event_bus::{
    AuthEvent, EventBus, SystemEvent, SystemNotification, TaskEvent, TelegramEvent,
};
use niupanel_core::settings::update::UpdateService;
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::task_status::TaskStatus;
use niupanel_entity::{tasks, tg_workflows};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::{
    ChatId, FileId, InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode,
};
use teloxide::utils::command::BotCommands;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use conversation::{AddTaskStep, ConversationState, ConversationStore, new_conversation_store};
use std::collections::HashMap;

/// 追踪运维任务 ID 与触发它的 Telegram 消息 ID 的映射
pub type JobMessageStore = Arc<RwLock<HashMap<i32, (ChatId, MessageId)>>>;

/// 追踪用户活跃的实时日志流 (ChatId -> (TaskId, tokio::task::JoinHandle))
pub type LiveStreamStore = Arc<RwLock<HashMap<String, (i32, tokio::task::JoinHandle<()>)>>>;

async fn execute_workflow_for_admins(
    bot: &Bot,
    admins: &[String],
    db: &DatabaseConnection,
    event_bus: &EventBus,
    workflow: tg_workflows::Model,
    task_id_context: Option<i32>,
) {
    let chat_ids = admins
        .iter()
        .filter_map(|id| match id.parse::<i64>() {
            Ok(id) if id != 0 => Some(ChatId(id)),
            _ => {
                warn!("忽略无效的 Telegram 管理员 Chat ID: {}", id);
                None
            }
        })
        .collect::<Vec<_>>();

    if workflow.action_type == "shell" {
        if let Some(chat_id) = chat_ids.first().copied() {
            helpers::execute_workflow(bot, chat_id, db, event_bus, workflow, task_id_context).await;
        }
        return;
    }

    for chat_id in chat_ids {
        helpers::execute_workflow(
            bot,
            chat_id,
            db,
            event_bus,
            workflow.clone(),
            task_id_context,
        )
        .await;
    }
}

async fn execute_matching_workflows(
    bot: &Bot,
    admins: &[String],
    db: &DatabaseConnection,
    event_bus: &EventBus,
    event_type: &str,
    task_id_context: Option<i32>,
) {
    let workflows = match tg_workflows::Entity::find()
        .filter(tg_workflows::Column::EventType.eq(event_type))
        .all(db)
        .await
    {
        Ok(workflows) => workflows,
        Err(err) => {
            error!("查询 {} 工作流失败: {}", event_type, err);
            return;
        }
    };

    for workflow in workflows {
        execute_workflow_for_admins(bot, admins, db, event_bus, workflow, task_id_context).await;
    }
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "📱 NiuPanel 移动运维终端")]
enum Command {
    #[command(description = "显示此帮助信息")]
    Help,
    #[command(description = "启动机器人")]
    Start,
    #[command(description = "任务管理: list, search, run, stop, retry, add, edit, del, logs")]
    Task(String),
    #[command(description = "变量管理: list, add, edit, del")]
    Var(String),
    #[command(description = "环境管理: list, pip npm apt")]
    Env(String),
    #[command(description = "系统管理: status, upgrade")]
    System(String),
    #[command(description = "执行 Shell 命令: /sh <命令>")]
    Sh(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TelegramBotConfig {
    pub enabled: bool,
    pub token: String,
    pub admin_chat_id: String,
    pub proxy_url: Option<String>,
    pub api_base_url: Option<String>,
    pub events: Vec<String>,
    pub cf_proxy_enabled: bool,
    pub cf_host: String,
    pub cf_ip: String,
    pub cf_token: String,
    pub login_2fa: bool,
}

pub struct TelegramBot {
    bot: Bot,
    admin_chat_ids: Vec<String>,
    events: Vec<String>,
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
                user: "".to_string(),
                passwd: "".to_string(),
                log: std::path::PathBuf::new(),
                loglevel: "info".to_string(),
            };
            tokio::spawn(async move {
                let _ =
                    niupanel_proxy::start_server(Arc::new(proxy_config), rx, ready_tx, "MainBot")
                        .await;
            });
            if let Ok(actual_addr) = ready_rx.await {
                proxy_url = Some(format!("socks5h://{}", actual_addr));
            }
        }

        let mut builder = reqwest::Client::builder();
        if let Some(proxy) = &proxy_url {
            if !proxy.is_empty() {
                if let Ok(p) = reqwest::Proxy::all(proxy) {
                    builder = builder.proxy(p);
                }
            }
        }

        let client = builder.build().unwrap_or_default();
        let mut bot = Bot::with_client(config.token.clone(), client);
        if let Some(base_url) = &config.api_base_url {
            if !base_url.is_empty() {
                if let Ok(url) = reqwest::Url::parse(base_url) {
                    bot = bot.set_api_url(url);
                }
            }
        }

        Self {
            bot,
            admin_chat_ids: config
                .admin_chat_id
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            events: config.events,
            proxy_shutdown_tx,
        }
    }

    pub async fn run(
        &self,
        event_bus: EventBus,
        db: DatabaseConnection,
        task_manager: TaskManagerService,
        update_service: UpdateService,
        metrics: Arc<RwLock<SystemMetrics>>,
        token: CancellationToken,
    ) -> anyhow::Result<()> {
        info!("Telegram 机器人已启动。");
        let _ = self.bot.delete_my_commands().await;
        let _ = self.bot.set_my_commands(Command::bot_commands()).await;

        let ver = env!("CARGO_PKG_VERSION").replace('.', "\\.");
        let startup_msg = format!("🚀 *NiuPanel 运维机器人 v{} 已启动*", ver);
        send_to_admins(&self.bot, &self.admin_chat_ids, &startup_msg).await;

        let conversations = new_conversation_store();
        let job_messages: JobMessageStore = Arc::new(RwLock::new(HashMap::new()));
        let live_streams: LiveStreamStore = Arc::new(RwLock::new(HashMap::new()));

        let bot_n = self.bot.clone();
        let admin_ids = self.admin_chat_ids.clone();
        let enabled_events = self.events.clone();
        let mut rx = event_bus.subscribe();
        let token_n = token.clone();
        let job_msgs_n = job_messages.clone();
        let db_n = db.clone();
        let eb_n = event_bus.clone();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if token_n.is_cancelled() {
                            break;
                        }
                        handle_system_event(
                            &bot_n,
                            &admin_ids,
                            &enabled_events,
                            event,
                            job_msgs_n.clone(),
                            &db_n,
                            &eb_n,
                        )
                        .await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Telegram event listener lagged by {} messages", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        {
            let bot_alert = self.bot.clone();
            let admin_ids_alert = self.admin_chat_ids.clone();
            let metrics_alert = metrics.clone();
            let token_alert = token.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    if token_alert.is_cancelled() {
                        break;
                    }
                    commands::check_resource_alerts(&bot_alert, &admin_ids_alert, &metrics_alert)
                        .await;
                }
            });
        }

        let handler = dptree::entry()
            .branch(Update::filter_message().endpoint(
                move |bot: Bot,
                      msg: Message,
                      admin_ids: Vec<String>,
                      metrics: Arc<RwLock<SystemMetrics>>,
                      db: DatabaseConnection,
                      eb: EventBus,
                      convs: ConversationStore,
                      tm: TaskManagerService,
                      _jm: JobMessageStore,
                      us: UpdateService,
                      _ls: LiveStreamStore| async move {
                    let cid = msg.chat.id;
                    if !admin_ids.contains(&cid.to_string()) {
                        warn!("拒绝访问: {}", cid);
                        let _ = bot
                            .send_message(cid, format!("🚫 *访问拒绝*\n您的 ID: `{}`", cid))
                            .parse_mode(ParseMode::MarkdownV2)
                            .await;
                        return ResponseResult::<()>::Ok(());
                    }

                    if let Some(doc) = msg.document() {
                        let cid_s = cid.to_string();
                        let in_add = {
                            let s = convs.read().await;
                            matches!(
                                s.get(&cid_s),
                                Some(ConversationState::AddTask {
                                    step: AddTaskStep::WaitingScript,
                                    ..
                                })
                            )
                        };
                        let name = doc
                            .file_name
                            .clone()
                            .unwrap_or_else(|| "unnamed".to_string());

                        if in_add {
                            match download_file_content(&bot, doc.file.id.to_string()).await {
                                Ok(content) => {
                                    commands::handle_script_upload(&bot, cid, content, name, convs)
                                        .await;
                                }
                                Err(e) => {
                                    warn!("下载文件失败: {}", e);
                                    let _ = bot
                                        .send_message(
                                            cid,
                                            format!(
                                                "❌ 文件下载失败: {}\n\n请重新上传文件，或发送 q 取消",
                                                escape_tg_markdown(&e.to_string())
                                            ),
                                        )
                                        .parse_mode(ParseMode::MarkdownV2)
                                        .await;
                                }
                            }
                        } else {
                            // Smart识别
                            let ext = std::path::Path::new(&name)
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if ["py", "js", "sh", "ts"].contains(&ext.as_str()) {
                                let msg_text = format!(
                                    "📄 *检测到脚本文件:* `{}`\n\n是否立即将其部署为任务？",
                                    escape_tg_markdown(&name)
                                );
                                let kb = InlineKeyboardMarkup::new(vec![vec![
                                    InlineKeyboardButton::callback(
                                        "🚀 立即部署",
                                        format!("task:quick_add:{}", doc.file.id),
                                    ),
                                    InlineKeyboardButton::callback(
                                        "📂 仅保存到服务器",
                                        format!("file:save:{}", doc.file.id),
                                    ),
                                ]]);
                                let _ = bot
                                    .send_message(cid, msg_text)
                                    .parse_mode(ParseMode::MarkdownV2)
                                    .reply_markup(kb)
                                    .await;
                            } else if ext == "npack" {
                                let msg_text = format!(
                                    "📦 *检测到 NiuPanel 分享包*\n\n文件: `{}`\n\n是否打开预览？",
                                    escape_tg_markdown(&name)
                                );
                                let kb = InlineKeyboardMarkup::new(vec![vec![
                                    InlineKeyboardButton::callback(
                                        "🔍 查看内容",
                                        format!("pkg:preview:{}", doc.file.id),
                                    ),
                                ]]);
                                let _ = bot
                                    .send_message(cid, msg_text)
                                    .parse_mode(ParseMode::MarkdownV2)
                                    .reply_markup(kb)
                                    .await;
                            } else {
                                if download_and_save_file(
                                    &bot,
                                    doc.file.id.to_string(),
                                    name.clone(),
                                )
                                .await
                                .is_ok()
                                {
                                    eb.publish(SystemEvent::Telegram(
                                        TelegramEvent::MessageReceived {
                                            chat_id: cid_s,
                                            username: None,
                                            text: Some(format!("[文件] {}", name)),
                                            file_name: Some(name),
                                            file_id: Some(doc.file.id.to_string()),
                                            timestamp: msg.date.timestamp() as u64,
                                        },
                                    ));
                                }
                            }
                        }
                        return ResponseResult::<()>::Ok(());
                    }

                    if let Some(text) = msg.text() {
                        {
                            let mut store = convs.write().await;
                            let cid_s = cid.to_string();
                            if let Some(state) = store.get(&cid_s) {
                                if state.is_timed_out() {
                                    store.remove(&cid_s);
                                    let _ = bot
                                        .send_message(cid, "⏰ 操作超时，会话已自动取消")
                                        .await;
                                }
                            }
                        }

                        if callbacks::handle_edit_step(&bot, cid, text, &db, &eb, &tm, &convs).await
                        {
                            return ResponseResult::<()>::Ok(());
                        }
                        if callbacks::handle_edit_var_step(&bot, cid, text, &db, &eb, &convs).await
                        {
                            return ResponseResult::<()>::Ok(());
                        }
                        if commands::handle_add_step(&bot, cid, text, &convs, &eb).await {
                            return ResponseResult::<()>::Ok(());
                        }

                        let cmd_res = Command::parse(text, "niupanel_bot")
                            .or_else(|_| Command::parse(text, ""));
                        match cmd_res {
                            Ok(cmd) => match cmd {
                                Command::Help | Command::Start => {
                                    bot.send_message(cid, Command::descriptions().to_string())
                                        .await?;
                                }
                                Command::Task(arg) => {
                                    commands::dispatch_task_cmd(
                                        &bot, cid, &db, &eb, &tm, &convs, &arg,
                                    )
                                    .await
                                }
                                Command::Var(arg) => {
                                    commands::dispatch_var_cmd(&bot, cid, &db, &convs, &arg).await
                                }
                                Command::Env(arg) => {
                                    commands::dispatch_env_cmd(&bot, cid, &tm, &eb, &arg).await
                                }
                                Command::System(arg) => {
                                    commands::dispatch_system_cmd(
                                        &bot, cid, &db, &metrics, &eb, &arg, &us,
                                    )
                                    .await
                                }
                                Command::Sh(arg) => {
                                    commands::handle_sh(&bot, cid, &eb, &arg).await
                                }
                            },
                            Err(e) => {
                                if text.starts_with('/') {
                                    let cmd_name =
                                        text[1..].split_whitespace().next().unwrap_or("");
                                    if let Ok(Some(cmd)) =
                                        niupanel_entity::tg_commands::Entity::find()
                                            .filter(
                                                niupanel_entity::tg_commands::Column::Name
                                                    .eq(cmd_name),
                                            )
                                            .one(&db)
                                            .await
                                    {
                                        helpers::execute_custom_command(&bot, cid, &db, &eb, cmd)
                                            .await;
                                    } else {
                                        debug!("指令解析失败 ({}): {}", text, e);
                                    }
                                }
                            }
                        }
                    }
                    ResponseResult::<()>::Ok(())
                },
            ))
            .branch(Update::filter_callback_query().endpoint(
                move |bot: Bot,
                      q: CallbackQuery,
                      admin_ids: Vec<String>,
                      db: DatabaseConnection,
                      eb: EventBus,
                      convs: ConversationStore,
                      tm: TaskManagerService,
                      jm: JobMessageStore,
                      us: UpdateService,
                      ls: LiveStreamStore| async move {
                    let cid = q
                        .message
                        .as_ref()
                        .map(|m| m.chat().id.to_string())
                        .unwrap_or_default();
                    if admin_ids.contains(&cid) {
                        callbacks::handle_callback(&bot, &q, &db, &eb, &tm, &convs, &jm, &us, &ls)
                            .await;
                    }
                    ResponseResult::<()>::Ok(())
                },
            ));

        let mut dispatcher = Dispatcher::builder(self.bot.clone(), handler)
            .dependencies(dptree::deps![
                self.admin_chat_ids.clone(),
                metrics,
                db,
                event_bus,
                conversations,
                task_manager,
                job_messages.clone(),
                update_service,
                live_streams
            ])
            .enable_ctrlc_handler()
            .build();

        tokio::select! {
            _ = dispatcher.dispatch() => info!("机器人消息分发已结束。"),
            _ = token.cancelled() => {
                info!("机器人分发任务正在停止...");
                let shutdown_msg = format!("🛑 *{}*", niupanel_common::escape_tg_markdown("NiuPanel 运维机器人正在停止..."));
                send_to_admins(&self.bot, &self.admin_chat_ids, &shutdown_msg).await;
            }
        }
        Ok(())
    }
}

async fn handle_system_event(
    bot: &Bot,
    admins: &[String],
    enabled: &[String],
    event: SystemEvent,
    job_msgs: JobMessageStore,
    db: &DatabaseConnection,
    eb: &EventBus,
) {
    let text = match event {
        SystemEvent::Auth(AuthEvent::LoginOtpRequest {
            username, ip, code, ..
        }) => {
            let msg = format!(
                "🔐 *NiuPanel 登录验证码*\n\n*用户:* `{}`\n*IP:* `{}`\n\n*验证码:* `{}`\n_验证码将在 5 分钟后过期。_",
                escape_tg_markdown(&username),
                escape_tg_markdown(&ip),
                escape_tg_markdown(&code)
            );
            for id in admins {
                let _ = bot
                    .send_message(id.clone(), msg.clone())
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;
            }
            return;
        }
        SystemEvent::Task(TaskEvent::StatusChanged {
            task_id,
            job_id,
            status,
            is_system,
            output,
            ..
        }) => {
            let is_finished = matches!(
                status,
                TaskStatus::Finished
                    | TaskStatus::Failed
                    | TaskStatus::Stopped
                    | TaskStatus::Cancelled
            );
            if !is_finished {
                return;
            }

            if !is_system {
                let workflow_event = match status {
                    TaskStatus::Finished => Some("success"),
                    TaskStatus::Failed => Some("failed"),
                    _ => None,
                };
                if let Some(workflow_event) = workflow_event {
                    execute_matching_workflows(bot, admins, db, eb, workflow_event, Some(task_id))
                        .await;
                }
            }

            let key = if status == TaskStatus::Finished {
                "success"
            } else {
                "failed"
            };
            if !is_system && !enabled.contains(&key.to_string()) {
                return;
            }

            let icon = if status == TaskStatus::Finished {
                "✅"
            } else {
                "⚠️"
            };
            let type_str = if is_system {
                "运维任务"
            } else {
                "脚本任务"
            };

            let display_id = if is_system {
                job_id.unwrap_or(task_id)
            } else {
                task_id
            };

            let mut task_name = String::new();
            if !is_system {
                if let Ok(Some(t)) = tasks::Entity::find_by_id(task_id).one(db).await {
                    task_name = t.name.clone();
                }
            }

            let mut final_output = output;
            if final_output.is_none() {
                if is_system {
                    if let Some(jid) = job_id {
                        let log_dir = Config::global().jobs_dir.join(jid.to_string());
                        final_output =
                            helpers::read_latest_log(&log_dir.to_string_lossy(), 1000).await;
                    }
                } else {
                    let log_dir = Config::global().logs_dir.join(task_id.to_string());
                    final_output = helpers::read_latest_log(&log_dir.to_string_lossy(), 1000).await;
                }
            }

            let mut msg_text = format!(
                "{} *{}完成*\nID: `{}`",
                icon,
                type_str,
                escape_tg_markdown(&display_id.to_string()),
            );
            if !task_name.is_empty() {
                msg_text.push_str(&format!("\n名称: `{}`", escape_tg_markdown(&task_name)));
            }
            msg_text.push_str(&format!(
                "\n状态: `{}`",
                escape_tg_markdown(&format!("{:?}", status))
            ));

            if let Some(out) = final_output {
                let clean_out = out
                    .replace('\\', "\\\\")
                    .replace('`', "\\`")
                    .replace('$', "\\$");
                let truncated_out = if clean_out.len() > 800 {
                    let mut end = 800;
                    while !clean_out.is_char_boundary(end) && end < clean_out.len() {
                        end += 1;
                    }
                    format!("{}...", &clean_out[..end])
                } else {
                    clean_out
                };
                msg_text.push_str(&format!("\n\n*输出摘要:*\n```\n{}\n```", truncated_out));
            }

            let retry_kb = if !is_system && status == TaskStatus::Failed {
                Some(InlineKeyboardMarkup::new(vec![vec![
                    InlineKeyboardButton::callback("🔄 重试", format!("confirm_run_{}", task_id)),
                    InlineKeyboardButton::callback("📄 日志", format!("view_logs_{}", task_id)),
                ]]))
            } else {
                None
            };

            if is_system {
                if let Some(jid) = job_id {
                    let mut map = job_msgs.write().await;
                    if let Some((target_cid, target_mid)) = map.remove(&jid) {
                        let mut req = bot
                            .edit_message_text(target_cid, target_mid, msg_text.clone())
                            .parse_mode(ParseMode::MarkdownV2);
                        if let Some(kb) = retry_kb.as_ref() {
                            req = req.reply_markup(kb.clone());
                        }
                        let _ = req.await;

                        let trigger_cid_str = target_cid.to_string();
                        for id in admins {
                            if id != &trigger_cid_str {
                                let mut req = bot
                                    .send_message(id.clone(), msg_text.clone())
                                    .parse_mode(ParseMode::MarkdownV2);
                                if let Some(kb) = retry_kb.as_ref() {
                                    req = req.reply_markup(kb.clone());
                                }
                                let _ = req.await;
                            }
                        }
                        return;
                    }
                }
            }

            if let Some(kb) = retry_kb {
                for id in admins {
                    let mut req = bot
                        .send_message(id.clone(), msg_text.clone())
                        .parse_mode(ParseMode::MarkdownV2);
                    req = req.reply_markup(kb.clone());
                    let _ = req.await;
                }
                return;
            }

            msg_text
        }
        SystemEvent::System(SystemNotification::Alert { message }) => {
            execute_matching_workflows(bot, admins, db, eb, "alert", None).await;
            if !enabled.contains(&"alert".to_string()) {
                return;
            }
            format!("🚨 *系统警报*\n{}", escape_tg_markdown(&message))
        }
        SystemEvent::System(SystemNotification::Notification {
            title,
            content,
            level,
        }) => {
            let key = if level == "error" || level == "warn" {
                execute_matching_workflows(bot, admins, db, eb, "alert", None).await;
                "alert"
            } else {
                "login"
            };
            if !enabled.contains(&key.to_string()) && !enabled.contains(&"notification".to_string())
            {
                return;
            }
            let emoji = match level.as_str() {
                "error" => "🔴",
                "warn" => "🟠",
                _ => "ℹ️",
            };
            format!(
                "{} *{}*\n{}",
                emoji,
                escape_tg_markdown(&title),
                escape_tg_markdown(&content)
            )
        }
        SystemEvent::Telegram(TelegramEvent::MessageSent { chat_id, text, .. }) => {
            if let Some(t) = text {
                let _ = bot.send_message(chat_id, t).await;
            }
            return;
        }
        SystemEvent::Telegram(TelegramEvent::WorkflowTriggered {
            workflow_id,
            task_id_context,
        }) => {
            niupanel_common::logger::info!("Received WorkflowTriggered for ID {}", workflow_id);
            if let Ok(Some(wf)) = niupanel_entity::tg_workflows::Entity::find_by_id(workflow_id)
                .one(db)
                .await
            {
                niupanel_common::logger::info!(
                    "Executing Workflow {} for {} admins",
                    workflow_id,
                    admins.len()
                );
                execute_workflow_for_admins(bot, admins, db, eb, wf, task_id_context).await;
            } else {
                niupanel_common::logger::warn!("Workflow {} not found in DB", workflow_id);
            }
            return;
        }
        _ => return,
    };
    for id in admins {
        let _ = bot
            .send_message(id.clone(), text.clone())
            .parse_mode(ParseMode::MarkdownV2)
            .await;
    }
}

impl Drop for TelegramBot {
    fn drop(&mut self) {
        if let Some(tx) = self.proxy_shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
async fn send_to_admins(bot: &Bot, admin_ids: &[String], text: &str) {
    for id in admin_ids {
        if let Err(e) = bot
            .send_message(id.clone(), text)
            .parse_mode(ParseMode::MarkdownV2)
            .await
        {
            error!("向管理员 {} 发送通知失败: {}", id, e);
        }
    }
}
pub(crate) async fn download_file_content(bot: &Bot, file_id: String) -> anyhow::Result<String> {
    let file = bot.get_file(FileId(file_id)).await?;
    let mut buf = Vec::new();
    bot.download_file(&file.path, &mut buf).await?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}
pub(crate) async fn download_and_save_file(
    bot: &Bot,
    file_id: String,
    name: String,
) -> anyhow::Result<()> {
    let dir = Config::global().scripts_dir.join("telegram");
    tokio::fs::create_dir_all(&dir).await?;

    let original_path = std::path::Path::new(&name);
    let base = original_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = original_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let unique_path = helpers::get_unique_path(&dir, base, ext);

    let file = bot.get_file(FileId(file_id)).await?;
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut out = options.open(&unique_path).await?;
    if let Err(err) = bot.download_file(&file.path, &mut out).await {
        drop(out);
        let _ = tokio::fs::remove_file(&unique_path).await;
        return Err(err.into());
    }
    if let Err(err) = out.sync_all().await {
        drop(out);
        let _ = tokio::fs::remove_file(&unique_path).await;
        return Err(err.into());
    }
    Ok(())
}
