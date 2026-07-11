use niupanel_common::escape_tg_markdown;
use niupanel_core::event_bus::{EventBus, SystemEvent, TaskEvent};
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::task_status::TaskStatus;
use niupanel_entity::tasks;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode};

use super::commands::{add_pagination, reply_or_edit, reply_or_edit_markup};
use super::conversation as conv;
use super::conversation::*;
use super::helpers::*;

pub async fn dispatch_task_cmd(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    eb: &EventBus,
    tm: &TaskManagerService,
    convs: &ConversationStore,
    arg: &str,
) {
    let parts: Vec<&str> = arg.split_whitespace().collect();
    let sub = parts.first().cloned().unwrap_or("list");
    let sub_arg = parts.get(1..).unwrap_or_default().join(" ");

    match sub {
        "list" => {
            let (page, status_filter) = parse_list_args(&sub_arg);
            handle_tasks(bot, chat_id, db, page, None, status_filter.as_deref()).await;
        }
        "search" => handle_search(bot, chat_id, db, &sub_arg).await,
        "run" => handle_run(bot, chat_id, db, eb, tm, &sub_arg, None).await,
        "stop" => {
            if sub_arg.trim().eq_ignore_ascii_case("all") {
                handle_stop_all(bot, chat_id, db, eb, tm).await;
            } else {
                handle_stop(bot, chat_id, db, eb, tm, &sub_arg, None).await;
            }
        }
        "retry" => handle_retry(bot, chat_id, db, eb, tm, &sub_arg).await,
        "chain" => handle_chain(bot, chat_id, db, eb, tm, &sub_arg).await,
        "add" => handle_add_start(bot, chat_id, convs).await,
        "edit" => handle_edit(bot, chat_id, db, convs, &sub_arg).await,
        "del" => handle_del(bot, chat_id, db, &sub_arg).await,
        "logs" => handle_logs(bot, chat_id, db, &sub_arg, None).await,
        _ => {
            let _ = bot
                .send_message(
                    chat_id,
                    "ℹ️ 用法: `/task <list|search|run|stop|retry|chain|add|edit|del|logs>`",
                )
                .await;
        }
    }
}

fn parse_list_args(arg: &str) -> (u64, Option<String>) {
    let parts: Vec<&str> = arg.split_whitespace().collect();
    let mut page = 0u64;
    let mut status_filter = None;
    for p in parts {
        if let Ok(n) = p.parse::<u64>() {
            page = n;
        } else {
            let lower = p.to_lowercase();
            match lower.as_str() {
                "running" | "idle" | "finished" | "failed" | "stopped" | "cancelled" => {
                    status_filter = Some(lower);
                }
                _ => {}
            }
        }
    }
    (page, status_filter)
}

pub async fn handle_tasks(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    page: u64,
    mid: Option<MessageId>,
    status_filter: Option<&str>,
) {
    let size: u64 = 15;
    let mut query = tasks::Entity::find();
    if let Some(status_str) = status_filter {
        if let Ok(status) = parse_status(status_str) {
            query = query.filter(tasks::Column::Status.eq(status));
        }
    }
    let list: Vec<tasks::Model> = query
        .order_by_desc(tasks::Column::UpdatedAt)
        .offset(Some(page * size))
        .limit(Some(size))
        .all(db)
        .await
        .unwrap_or_default();
    let total = tasks::Entity::find().count(db).await.unwrap_or(0);

    if list.is_empty() && page == 0 {
        let filter_hint = status_filter
            .map(|s| format!(" \\(过滤: {}\\)", s))
            .unwrap_or_default();
        reply_or_edit(
            bot,
            chat_id,
            mid,
            &format!("📋 暂无任务{}。使用 `/task add` 创建\\.", filter_hint),
        )
        .await;
        return;
    }

    let mut kb = vec![];
    for chunk in list.chunks(3) {
        let mut row = vec![];
        for t in chunk {
            let icon = match t.status {
                TaskStatus::Running => "🔵",
                TaskStatus::Finished => "🟢",
                TaskStatus::Failed => "🔴",
                _ => "⚪",
            };
            row.push(InlineKeyboardButton::callback(
                format!("{} {}", icon, t.name),
                format!("task_manage_{}", t.id),
            ));
        }
        kb.push(row);
    }
    let prefix = status_filter
        .map(|s| format!("tasks_filter_{}_", s))
        .unwrap_or_else(|| "tasks_page_".to_string());
    add_pagination(&mut kb, page, total, size, &prefix);
    let title = status_filter
        .map(|s| format!("📋 *任务列表* \\(过滤: {}\\)", s))
        .unwrap_or_else(|| format!("📋 *任务列表* \\(第{}页\\)", page + 1));
    reply_or_edit_markup(bot, chat_id, mid, &title, InlineKeyboardMarkup::new(kb)).await;
}

fn parse_status(s: &str) -> Result<TaskStatus, ()> {
    match s.to_lowercase().as_str() {
        "running" => Ok(TaskStatus::Running),
        "idle" => Ok(TaskStatus::Idle),
        "finished" => Ok(TaskStatus::Finished),
        "failed" => Ok(TaskStatus::Failed),
        "stopped" => Ok(TaskStatus::Stopped),
        "cancelled" => Ok(TaskStatus::Cancelled),
        _ => Err(()),
    }
}

pub async fn handle_search(bot: &Bot, chat_id: ChatId, db: &DatabaseConnection, keyword: &str) {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        let _ = bot
            .send_message(chat_id, "ℹ️ 用法: `/task search <关键词>`")
            .await;
        return;
    }
    let pattern = format!("%{}%", keyword);
    let list: Vec<tasks::Model> = tasks::Entity::find()
        .filter(
            sea_orm::Condition::any()
                .add(tasks::Column::Name.contains(&pattern))
                .add(tasks::Column::Path.contains(&pattern)),
        )
        .order_by_desc(tasks::Column::UpdatedAt)
        .limit(Some(15))
        .all(db)
        .await
        .unwrap_or_default();

    if list.is_empty() {
        let _ = bot
            .send_message(
                chat_id,
                format!("🔍 未找到匹配 `{}` 的任务", escape_tg_markdown(keyword)),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await;
        return;
    }

    let mut kb = vec![];
    for chunk in list.chunks(3) {
        let mut row = vec![];
        for t in chunk {
            let icon = match t.status {
                TaskStatus::Running => "🔵",
                TaskStatus::Finished => "🟢",
                TaskStatus::Failed => "🔴",
                _ => "⚪",
            };
            row.push(InlineKeyboardButton::callback(
                format!("{} {}", icon, t.name),
                format!("task_manage_{}", t.id),
            ));
        }
        kb.push(row);
    }
    let _ = bot
        .send_message(
            chat_id,
            format!(
                "🔍 搜索 `{}` \\- 找到 {} 个结果",
                escape_tg_markdown(keyword),
                list.len()
            ),
        )
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(InlineKeyboardMarkup::new(kb))
        .await;
}

pub async fn handle_retry(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    eb: &EventBus,
    tm: &TaskManagerService,
    arg: &str,
) {
    let tid = match parse_task_id(arg, db).await {
        Some(id) => id,
        None => return,
    };
    if let Ok(Some(t)) = tasks::Entity::find_by_id(tid).one(db).await {
        if t.status != TaskStatus::Failed {
            let _ = bot
                .send_message(
                    chat_id,
                    format!(
                        "⚠️ 任务 `{}` 当前状态为 `{}`，仅失败任务可重试",
                        escape_tg_markdown(&t.name),
                        escape_tg_markdown(&format!("{:?}", t.status))
                    ),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await;
            return;
        }
        if tm.start_task(&t).await.is_ok() {
            let _ = bot
                .send_message(
                    chat_id,
                    format!("🔄 重试任务 `{}`", escape_tg_markdown(&t.name)),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await;
            publish_audit(eb, chat_id.to_string(), format!("🔄 重试任务: {}", t.name));
        }
    }
}

pub async fn handle_chain(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    eb: &EventBus,
    tm: &TaskManagerService,
    arg: &str,
) {
    let ids: Vec<i32> = arg
        .split_whitespace()
        .filter_map(|s| s.parse::<i32>().ok())
        .collect();

    if ids.len() < 2 {
        let _ = bot
            .send_message(chat_id, "ℹ️ 用法: `/task chain <id1> <id2> [id3...]`")
            .await;
        return;
    }

    let mut names = vec![];
    let mut valid_ids = vec![];
    for &id in &ids {
        if let Ok(Some(t)) = tasks::Entity::find_by_id(id).one(db).await {
            names.push(t.name.clone());
            valid_ids.push(id);
        }
    }

    if valid_ids.len() < 2 {
        let _ = bot
            .send_message(chat_id, "❌ 至少需要 2 个有效任务 ID")
            .await;
        return;
    }

    let chain_display = names
        .iter()
        .enumerate()
        .map(|(i, n)| format!("{}\\. {}", i + 1, escape_tg_markdown(n)))
        .collect::<Vec<_>>()
        .join("\n");

    let _ = bot
        .send_message(
            chat_id,
            format!(
                "🔗 *链式执行*\n{}\n\n⏳ 正在启动第一个任务\\.\\.\\.",
                chain_display
            ),
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await;

    let first_id = valid_ids[0];
    if let Ok(Some(first_task)) = tasks::Entity::find_by_id(first_id).one(db).await {
        if tm.start_task(&first_task).await.is_ok() {
            publish_audit(
                eb,
                chat_id.to_string(),
                format!("🔗 链式执行: {}", names.join(" → ")),
            );

            let remaining_ids = valid_ids[1..].to_vec();
            let remaining_names = names[1..].to_vec();
            let eb_chain = eb.clone();
            let tm_chain = tm.clone();
            let db_chain = db.clone();
            let bot_chain = bot.clone();
            let chat_id_chain = chat_id;

            tokio::spawn(async move {
                let mut rx = eb_chain.subscribe();
                let mut current_id = first_id;
                let mut idx = 0;

                loop {
                    match tokio::time::timeout(std::time::Duration::from_secs(3600), rx.recv())
                        .await
                    {
                        Ok(Ok(event)) => {
                            if let SystemEvent::Task(TaskEvent::StatusChanged {
                                task_id,
                                status,
                                is_system: false,
                                ..
                            }) = &event
                            {
                                if *task_id == current_id
                                    && matches!(
                                        status,
                                        TaskStatus::Finished
                                            | TaskStatus::Failed
                                            | TaskStatus::Stopped
                                            | TaskStatus::Cancelled
                                    )
                                {
                                    if idx >= remaining_ids.len() {
                                        let _ = bot_chain
                                            .send_message(chat_id_chain, "✅ 链式执行全部完成")
                                            .await;
                                        break;
                                    }

                                    if status != &TaskStatus::Finished {
                                        let _ = bot_chain
                                            .send_message(
                                                chat_id_chain,
                                                format!(
                                                    "⚠️ 链式执行中断: 任务 `{}` 未成功完成",
                                                    escape_tg_markdown(
                                                        remaining_names
                                                            .get(idx - 1)
                                                            .unwrap_or(&"未知".to_string())
                                                    )
                                                ),
                                            )
                                            .parse_mode(ParseMode::MarkdownV2)
                                            .await;
                                        break;
                                    }

                                    let next_id = remaining_ids[idx];
                                    if let Ok(Some(next_task)) =
                                        tasks::Entity::find_by_id(next_id).one(&db_chain).await
                                    {
                                        if tm_chain.start_task(&next_task).await.is_ok() {
                                            let _ = bot_chain
                                                .send_message(
                                                    chat_id_chain,
                                                    format!(
                                                        "🔗 启动下一个: `{}`",
                                                        escape_tg_markdown(
                                                            remaining_names
                                                                .get(idx)
                                                                .unwrap_or(&"未知".to_string())
                                                        )
                                                    ),
                                                )
                                                .parse_mode(ParseMode::MarkdownV2)
                                                .await;
                                        }
                                    }
                                    current_id = next_id;
                                    idx += 1;
                                }
                            }
                        }
                        _ => {
                            let _ = bot_chain
                                .send_message(chat_id_chain, "⏰ 链式执行等待超时")
                                .await;
                            break;
                        }
                    }
                }
            });
        }
    }
}

pub async fn handle_task_manage_menu(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    id: i32,
    mid: MessageId,
) {
    let t = match tasks::Entity::find_by_id(id).one(db).await {
        Ok(Some(t)) => t,
        _ => return,
    };
    let text = format!(
        "⚙️ *管理任务*\n\n*名称:* `{}`\n*状态:* `{:?}`\n*环境:* `{}`\n*定时:* `{}`",
        escape_tg_markdown(&t.name),
        t.status,
        escape_tg_markdown(&t.env_type),
        escape_tg_markdown(t.cron_schedule.as_deref().unwrap_or("手动"))
    );

    let mut kb = vec![vec![InlineKeyboardButton::callback(
        if t.status == TaskStatus::Running {
            "⏹️ 停止"
        } else {
            "▶️ 执行"
        },
        format!(
            "confirm_{}_{}",
            if t.status == TaskStatus::Running {
                "stop"
            } else {
                "run"
            },
            t.id
        ),
    )]];

    if t.status == TaskStatus::Running {
        kb.push(vec![InlineKeyboardButton::callback(
            "📺 追踪执行",
            format!("live_logs_{}", t.id),
        )]);
    }

    kb.push(vec![
        InlineKeyboardButton::callback("📄 日志", format!("view_logs_{}", t.id)),
        InlineKeyboardButton::callback("✏️ 编辑", format!("task_edit_{}", t.id)),
    ]);
    kb.push(vec![
        InlineKeyboardButton::callback(
            if t.enabled {
                "🚫 禁用"
            } else {
                "✅ 启用"
            },
            if t.enabled {
                format!("confirm_disable_{}", t.id)
            } else {
                format!("confirm_enable_{}", t.id)
            },
        ),
        InlineKeyboardButton::callback("🗑️ 删除", format!("confirm_del_{}", t.id)),
    ]);
    kb.push(vec![InlineKeyboardButton::callback(
        "⬅️ 返回列表",
        "tasks_page_0".to_string(),
    )]);

    let _ = bot
        .edit_message_text(chat_id, mid, text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(InlineKeyboardMarkup::new(kb))
        .await;
}

pub async fn handle_run(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    eb: &EventBus,
    tm: &TaskManagerService,
    arg: &str,
    mid: Option<MessageId>,
) {
    let tid = match parse_task_id(arg, db).await {
        Some(id) => id,
        None => return,
    };
    if let Ok(Some(t)) = tasks::Entity::find_by_id(tid).one(db).await {
        if let Some(m) = mid {
            let _ = bot
                .edit_message_text(
                    chat_id,
                    m,
                    format!("⏳ 启动: {}\\.\\.\\.", escape_tg_markdown(&t.name)),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await;
        }
        if tm.start_task(&t).await.is_ok() {
            if let Some(m) = mid {
                handle_task_manage_menu(bot, chat_id, db, t.id, m).await;
            } else {
                let _ = bot
                    .send_message(
                        chat_id,
                        format!("🚀 任务 `{}` 已开始执行", escape_tg_markdown(&t.name)),
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;
            }
            publish_audit(eb, chat_id.to_string(), format!("▶️ 执行任务: {}", t.name));
        }
    }
}

pub async fn handle_stop(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    _eb: &EventBus,
    tm: &TaskManagerService,
    arg: &str,
    mid: Option<MessageId>,
) {
    let tid = match parse_task_id(arg, db).await {
        Some(id) => id,
        None => return,
    };

    if let Some(m) = mid {
        let _ = bot
            .edit_message_text(chat_id, m, "⏳ 正在发送停止信号...")
            .await;
    }

    if tm.stop_task(tid).await.is_ok() {
        for _ in 0..15 {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            if let Ok(Some(t)) = tasks::Entity::find_by_id(tid).one(db).await {
                if t.status != TaskStatus::Running {
                    break;
                }
            }
        }
    }

    if let Some(m) = mid {
        handle_task_manage_menu(bot, chat_id, db, tid, m).await;
    }
}

pub async fn handle_stop_all(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    eb: &EventBus,
    tm: &TaskManagerService,
) {
    let running: Vec<tasks::Model> = tasks::Entity::find()
        .filter(tasks::Column::Status.eq(TaskStatus::Running))
        .all(db)
        .await
        .unwrap_or_default();

    if running.is_empty() {
        let _ = bot.send_message(chat_id, "ℹ️ 当前没有运行中的任务").await;
        return;
    }

    let count = running.len();
    let mut stopped = 0u32;
    let mut failed_names = vec![];

    for t in &running {
        if tm.stop_task(t.id).await.is_ok() {
            stopped += 1;
        } else {
            failed_names.push(t.name.clone());
        }
    }

    let mut msg = format!("⏹️ 批量停止: {}/{} 成功", stopped, count);
    if !failed_names.is_empty() {
        msg.push_str(&format!(
            "\n❌ 失败: {}",
            failed_names
                .iter()
                .map(|n| escape_tg_markdown(n))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let _ = bot
        .send_message(chat_id, msg)
        .parse_mode(ParseMode::MarkdownV2)
        .await;

    publish_audit(
        eb,
        chat_id.to_string(),
        format!("⏹️ 批量停止 {} 个任务", count),
    );
}

pub async fn handle_logs(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    arg: &str,
    mid: Option<MessageId>,
) {
    let tid = match parse_task_id(arg, db).await {
        Some(id) => id,
        None => return,
    };
    if let Ok(Some(t)) = tasks::Entity::find_by_id(tid).one(db).await {
        let log = read_latest_log(&format!("data/logs/{}", tid), 3000)
            .await
            .unwrap_or_else(|| "📄 暂无日志".to_string());

        let escaped_log = log
            .replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace('$', "\\$");

        let kb = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("🔄 刷新", format!("view_logs_{}", tid)),
            InlineKeyboardButton::callback("⬅️ 返回", format!("task_manage_{}", tid)),
        ]]);
        reply_or_edit_markup(
            bot,
            chat_id,
            mid,
            &format!(
                "📄 *{}* 日志:\n```\n{}\n```",
                escape_tg_markdown(&t.name),
                escaped_log
            ),
            kb,
        )
        .await;
    }
}

pub async fn handle_del(bot: &Bot, chat_id: ChatId, _db: &DatabaseConnection, arg: &str) {
    let tid = arg.trim().parse::<i32>().unwrap_or(0);
    let kb = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("🗑️ 确认删除", format!("confirm_del_{}", tid)),
        InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
    ]]);
    let _ = bot
        .send_message(chat_id, format!("⚠️ 确认删除任务 ID: `{}`?", tid))
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(kb)
        .await;
}

pub async fn handle_edit(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    convs: &ConversationStore,
    arg: &str,
) {
    let tid = match parse_task_id(arg, db).await {
        Some(id) => id,
        None => return,
    };
    let kb = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("📝 名称", format!("editfield_{}_name", tid)),
            InlineKeyboardButton::callback("⏰ Cron", format!("editfield_{}_cron", tid)),
        ],
        vec![
            InlineKeyboardButton::callback("📄 描述", format!("editfield_{}_desc", tid)),
            InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
        ],
    ]);
    if let Ok(m) = bot
        .send_message(chat_id, format!("✏️ 编辑任务 ID: `{}`", tid))
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(kb)
        .await
    {
        convs.write().await.insert(
            chat_id.to_string(),
            ConversationState::EditTask {
                task_id: tid,
                step: EditTaskStep::SelectingField,
                last_msg_id: Some(m.id),
                created_at: conv::now(),
            },
        );
    }
}

pub async fn handle_add_start(bot: &Bot, chat_id: ChatId, convs: &ConversationStore) {
    if let Ok(m) = bot.send_message(chat_id, "🆕 请输入新任务名称：").await {
        convs.write().await.insert(
            chat_id.to_string(),
            ConversationState::AddTask {
                step: AddTaskStep::WaitingName,
                data: AddTaskData::default(),
                last_msg_id: Some(m.id),
                created_at: conv::now(),
            },
        );
    }
}

pub async fn handle_add_step(
    bot: &Bot,
    chat_id: ChatId,
    text: &str,
    convs: &ConversationStore,
    _eb: &EventBus,
) -> bool {
    let mut store = convs.write().await;
    let cid = chat_id.to_string();
    let (step, mut data, mid, created_at) = match store.get(&cid) {
        Some(ConversationState::AddTask {
            step,
            data,
            last_msg_id,
            created_at,
        }) => (step.clone(), data.clone(), *last_msg_id, *created_at),
        _ => return false,
    };
    if text.trim().to_lowercase() == "q" {
        store.remove(&cid);
        if let Some(m) = mid {
            let _ = bot.edit_message_text(chat_id, m, "❌ 已取消").await;
        }
        return true;
    }
    match step {
        AddTaskStep::WaitingName => {
            data.name = Some(text.trim().to_string());
            let new_mid = if let Ok(sent) = bot
                .send_message(
                    chat_id,
                    format!(
                        "✅ 任务名称: `{}`\n\n🆕 请发送脚本内容或直接上传文件：\n_发送 q 取消_",
                        escape_tg_markdown(text.trim())
                    ),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await
            {
                Some(sent.id)
            } else {
                mid
            };
            store.insert(
                cid,
                ConversationState::AddTask {
                    step: AddTaskStep::WaitingScript,
                    data,
                    last_msg_id: new_mid,
                    created_at,
                },
            );
        }
        AddTaskStep::WaitingScript => {
            data.script_content = Some(text.trim().to_string());
            let kb = InlineKeyboardMarkup::new(vec![vec![
                InlineKeyboardButton::callback("🐍 Python", "addenv_python3"),
                InlineKeyboardButton::callback("📦 Node", "addenv_node"),
                InlineKeyboardButton::callback("🐚 Shell", "addenv_shell"),
            ]]);
            let new_mid = if let Ok(sent) = bot
                .send_message(chat_id, "🆕 请选择运行环境：")
                .reply_markup(kb)
                .await
            {
                Some(sent.id)
            } else {
                mid
            };
            store.insert(
                cid,
                ConversationState::AddTask {
                    step: AddTaskStep::WaitingEnvType,
                    data,
                    last_msg_id: new_mid,
                    created_at,
                },
            );
        }
        AddTaskStep::WaitingCron => {
            data.cron_schedule = Some(text.trim().to_string());
            let kb = InlineKeyboardMarkup::new(vec![vec![
                InlineKeyboardButton::callback("✅ 确认创建", "add_confirm"),
                InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
            ]]);
            let new_mid = if let Ok(sent) = bot
                .send_message(
                    chat_id,
                    format!(
                        "📋 Cron: `{}`\n确认创建该任务？",
                        escape_tg_markdown(text.trim())
                    ),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .reply_markup(kb)
                .await
            {
                Some(sent.id)
            } else {
                mid
            };
            store.insert(
                cid,
                ConversationState::AddTask {
                    step: AddTaskStep::Confirming,
                    data,
                    last_msg_id: new_mid,
                    created_at,
                },
            );
        }
        _ => return false,
    }
    true
}

pub async fn handle_script_upload(
    bot: &Bot,
    chat_id: ChatId,
    content: String,
    file_name: String,
    convs: ConversationStore,
) {
    let mut store = convs.write().await;
    let cid = chat_id.to_string();
    if let Some(ConversationState::AddTask {
        data,
        last_msg_id,
        step,
        ..
    }) = store.get_mut(&cid)
    {
        if *step != AddTaskStep::WaitingScript {
            drop(store);
            let _ = bot
                .send_message(chat_id, "⚠️ 当前步骤不支持文件上传，请按提示操作")
                .await;
            return;
        }
        data.script_content = Some(content);
        data.file_name = Some(file_name.clone());
        let ext = std::path::Path::new(&file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let env = match ext {
            "py" => "python3",
            "js" | "ts" => "node",
            _ => "shell",
        };
        data.env_type = Some(env.to_string());
        *step = AddTaskStep::WaitingCron;

        let kb = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
            "⏩ 跳过 (手动执行)",
            "add_cron_skip",
        )]]);
        if let Ok(sent) = bot
            .send_message(
                chat_id,
                format!(
                    "📄 已识别环境: `{}`\n⏰ 请输入 Cron 表达式\\(如: `0 0 \\* \\* \\*`\\) 或点击跳过：",
                    env
                ),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(kb)
            .await
        {
            *last_msg_id = Some(sent.id);
        }
    } else {
        drop(store);
        let _ = bot
            .send_message(
                chat_id,
                "⚠️ 没有进行中的任务创建会话，请使用 /task add 重新开始",
            )
            .await;
    }
}

pub async fn handle_add_confirm(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    eb: &EventBus,
    convs: &ConversationStore,
) {
    let state = convs.write().await.remove(&chat_id.to_string());
    if let Some(ConversationState::AddTask {
        data, last_msg_id, ..
    }) = state
    {
        let name = data.name.unwrap_or_default();
        let env_type = data.env_type.clone().unwrap_or_else(|| "shell".to_string());

        let ext = match env_type.as_str() {
            "python3" => "py",
            "node" => "js",
            _ => "sh",
        };

        let path = if let Some(orig_file) = data.file_name {
            format!("telegram/{}", orig_file)
        } else {
            format!("telegram/{}.{}", name, ext)
        };

        let am = tasks::ActiveModel {
            name: Set(name),
            path: Set(path),
            env_type: Set(env_type),
            cron_schedule: Set(data.cron_schedule),
            status: Set(TaskStatus::Idle),
            enabled: Set(true),
            user_id: Set(1),
            ..Default::default()
        };
        if let Ok(t) = am.insert(db).await {
            if let Some(c) = data.script_content {
                let dir = std::path::Path::new("data/scripts/telegram");
                if !dir.exists() {
                    let _ = tokio::fs::create_dir_all(dir).await;
                }
                let _ = tokio::fs::write(format!("data/scripts/{}", t.path), c).await;
            }
            let success_text = format!(
                "✅ *任务创建成功！*\n\n使用以下指令立即运行：\n`/task run {}`",
                t.id
            );
            reply_or_edit(bot, chat_id, last_msg_id, &success_text).await;
            publish_audit(eb, chat_id.to_string(), format!("✅ 创建任务: {}", t.name));
        }
    }
}
