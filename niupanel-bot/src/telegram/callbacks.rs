use niupanel_common::escape_tg_markdown;
use niupanel_common::logger::{debug, warn};
use niupanel_common::models::update::UpdateState;
use niupanel_core::event_bus::{AuthEvent, EventBus, SystemEvent, TelegramEvent};
use niupanel_core::settings::update::UpdateService;
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::task_status::TaskStatus;
use niupanel_entity::{tasks, variables, variables_tasks};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::{FileId, InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode};

use super::commands::{self, handle_task_manage_menu, handle_var_manage_menu};
use super::conversation as conv;
use super::conversation::*;
use super::helpers::*;
use super::{download_and_save_file, download_file_content};

async fn reply_success(bot: &Bot, chat_id: ChatId, mid: Option<MessageId>, text: &str) {
    if let Some(id) = mid {
        let _ = bot.edit_message_text(chat_id, id, text).await;
    } else {
        let _ = bot.send_message(chat_id, text).await;
    }
}

pub async fn handle_callback(
    bot: &Bot,
    q: &CallbackQuery,
    db: &DatabaseConnection,
    eb: &EventBus,
    tm: &TaskManagerService,
    convs: &ConversationStore,
    jm: &super::JobMessageStore,
    us: &UpdateService,
    ls: &super::LiveStreamStore,
) {
    let chat_id = match q.message.as_ref() {
        Some(m) => m.chat().id,
        None => return,
    };
    let data = match q.data.as_ref() {
        Some(d) => d.as_str(),
        None => return,
    };
    let msg_id = q.message.as_ref().map(|m| m.id());

    debug!("Callback: {} from {}", data, chat_id);

    match data {
        d if d.starts_with("auth:approve:") => {
            let ticket = d.strip_prefix("auth:approve:").unwrap_or(d).to_string();
            eb.publish(SystemEvent::Auth(AuthEvent::LoginApprovalResponse {
                ticket,
                approved: true,
            }));
            if let Some(mid) = msg_id {
                let _ = bot.edit_message_text(chat_id, mid, "✅ 已批准登录").await;
            }
        }
        d if d.starts_with("auth:deny:") => {
            let ticket = d.strip_prefix("auth:deny:").unwrap_or(d).to_string();
            eb.publish(SystemEvent::Auth(AuthEvent::LoginApprovalResponse {
                ticket,
                approved: false,
            }));
            if let Some(mid) = msg_id {
                let _ = bot.edit_message_text(chat_id, mid, "❌ 已拒绝登录").await;
            }
        }
        d if d.starts_with("task:quick_add:") => {
            let file_id = d.strip_prefix("task:quick_add:").unwrap_or(d).to_string();
            if let Some(mid) = msg_id {
                let _ = bot
                    .edit_message_text(chat_id, mid, "⏳ 正在准备部署...")
                    .await;
                handle_quick_task_add(bot, chat_id, mid, file_id, db, eb, tm).await;
            }
        }
        d if d.starts_with("file:save:") => {
            let file_id = d.strip_prefix("file:save:").unwrap_or(d).to_string();
            if let Some(mid) = msg_id {
                if let Ok(file) = bot.get_file(FileId(file_id)).await {
                    let name = std::path::Path::new(&file.path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unnamed")
                        .to_string();
                    if download_and_save_file(bot, file.id.to_string(), name.clone())
                        .await
                        .is_ok()
                    {
                        let _ = bot
                            .edit_message_text(
                                chat_id,
                                mid,
                                format!(
                                    "✅ 文件 `{}` 已保存至服务器 `telegram` 目录",
                                    escape_tg_markdown(&name)
                                ),
                            )
                            .parse_mode(ParseMode::MarkdownV2)
                            .await;
                    }
                }
            }
        }
        d if d.starts_with("pkg:preview:") => {
            let file_id = d.strip_prefix("pkg:preview:").unwrap_or(d).to_string();
            if let Some(mid) = msg_id {
                let _ = bot
                    .edit_message_text(chat_id, mid, "⏳ 正在解析分享包...")
                    .await;
                handle_package_preview(bot, chat_id, mid, file_id, db).await;
            }
        }
        d if d.starts_with("pkg:import:") => {
            let staging_id = d.strip_prefix("pkg:import:").unwrap_or(d).to_string();
            if let Some(mid) = msg_id {
                let _ = bot
                    .edit_message_text(chat_id, mid, "⏳ 正在导入任务...")
                    .await;
                handle_package_import(bot, chat_id, mid, staging_id, db, eb).await;
            }
        }
        "upgrade:start" => {
            if let Some(mid) = msg_id {
                let _ = bot
                    .edit_message_text(chat_id, mid, "⏳ 正在准备更新...")
                    .await;
                if let Err(e) = us.execute_update().await {
                    let _ = bot
                        .edit_message_text(chat_id, mid, format!("❌ 更新启动失败: {}", e))
                        .await;
                } else {
                    monitor_update(bot.clone(), chat_id, mid, us.clone()).await;
                }
            }
        }
        d if d.starts_with("wf_approve_") => {
            let nonce = d.strip_prefix("wf_approve_").unwrap_or(d).to_string();
            let pending_script_path = format!("/tmp/tg_pending_approval_{}.sh", nonce);

            if let Ok(script) = tokio::fs::read_to_string(&pending_script_path).await {
                if let Some(mid) = msg_id {
                    let _ = bot
                        .edit_message_text(
                            chat_id,
                            mid,
                            "✅ 已授权执行该自动化操作，正在执行\\.\\.\\.",
                        )
                        .parse_mode(ParseMode::MarkdownV2)
                        .await;
                }

                let dummy_cmd = niupanel_entity::tg_commands::Model {
                    id: 0,
                    name: format!("approve_{}", nonce),
                    script,
                    created_at: chrono::Utc::now().naive_utc(),
                    updated_at: chrono::Utc::now().naive_utc(),
                };

                // execute the script
                super::helpers::execute_custom_command(bot, chat_id, db, eb, dummy_cmd).await;

                // clean up
                let _ = tokio::fs::remove_file(&pending_script_path).await;
            } else {
                if let Some(mid) = msg_id {
                    let _ = bot
                        .edit_message_text(chat_id, mid, "❌ 授权失败：该操作已过期或不存在")
                        .await;
                }
            }
        }
        d if d.starts_with("wf_reject_") => {
            let nonce = d.strip_prefix("wf_reject_").unwrap_or(d).to_string();
            let pending_script_path = format!("/tmp/tg_pending_approval_{}.sh", nonce);

            // clean up
            let _ = tokio::fs::remove_file(&pending_script_path).await;

            if let Some(mid) = msg_id {
                let _ = bot
                    .edit_message_text(chat_id, mid, "🚫 已取消该自动化操作")
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;
            }
        }
        "upgrade:cancel" => {
            let _ = us.cancel_update().await;
            if let Some(mid) = msg_id {
                let _ = bot
                    .edit_message_text(chat_id, mid, "⌛ 正在取消更新...")
                    .await;
            }
        }
        d if d.starts_with("tasks_page_") || d.starts_with("tasks_filter_") => {
            let (page, status_filter) = if let Some(rest) = d.strip_prefix("tasks_filter_") {
                let mut parts = rest.splitn(2, '_');
                let status = parts.next().unwrap_or("").to_string();
                let p = parts
                    .next()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                (p, Some(status))
            } else {
                let p = d
                    .strip_prefix("tasks_page_")
                    .unwrap_or(d)
                    .parse::<u64>()
                    .unwrap_or(0);
                (p, None)
            };
            commands::handle_tasks(bot, chat_id, db, page, msg_id, status_filter.as_deref()).await;
        }
        d if d.starts_with("vars_page_") => {
            if let Ok(p) = d.strip_prefix("vars_page_").unwrap_or(d).parse::<u64>() {
                commands::handle_vars(bot, chat_id, db, p, msg_id).await;
            }
        }
        d if d.starts_with("task_manage_") => {
            if let Ok(id) = d.strip_prefix("task_manage_").unwrap_or(d).parse::<i32>() {
                handle_task_manage_menu(bot, chat_id, db, id, msg_id.unwrap()).await;
            }
        }
        d if d.starts_with("var_manage_") => {
            if let Ok(id) = d.strip_prefix("var_manage_").unwrap_or(d).parse::<i32>() {
                handle_var_manage_menu(bot, chat_id, db, id, msg_id.unwrap()).await;
            }
        }
        d if d.starts_with("pip") || d.starts_with("npm") || d.starts_with("apt") => {
            if let Some(mid) = msg_id {
                if super::deps::handle_dep_callback(bot, chat_id, mid, d, tm, eb, jm).await {
                    return;
                }
            }
        }
        d if d.starts_with("task_edit_") => {
            let id_str = d.strip_prefix("task_edit_").unwrap_or(d);
            commands::handle_edit(bot, chat_id, db, convs, id_str).await;
        }
        d if d.starts_with("var_edit_") => {
            let id_str = d.strip_prefix("var_edit_").unwrap_or(d);
            commands::handle_edit_var(bot, chat_id, db, convs, id_str).await;
        }
        d if d.starts_with("view_logs_") => {
            let id_str = d.strip_prefix("view_logs_").unwrap_or(d);
            commands::handle_logs(bot, chat_id, db, id_str, msg_id).await;
        }
        d if d.starts_with("live_logs_") => {
            let id_str = d.strip_prefix("live_logs_").unwrap_or(d);
            if let Ok(id) = id_str.parse::<i32>() {
                handle_live_logs(bot.clone(), chat_id, msg_id, id, db, tm, ls.clone()).await;
            }
        }
        d if d.starts_with("stop_live_") => {
            // User explicitly cancels the live stream
            let id_str = d.strip_prefix("stop_live_").unwrap_or(d);
            if let Ok(id) = id_str.parse::<i32>() {
                if let Some((_, handle)) = ls.write().await.remove(&chat_id.to_string()) {
                    handle.abort();
                }
                if let Some(mid) = msg_id {
                    let _ = bot
                        .edit_message_text(
                            chat_id,
                            mid,
                            format!("⏹️ 已停止追踪任务 `{}` 的日志。", id),
                        )
                        .await;
                    commands::handle_task_manage_menu(bot, chat_id, db, id, mid).await;
                }
            }
        }
        d if d.starts_with("confirm_run_") => {
            let id_str = d.strip_prefix("confirm_run_").unwrap_or(d);
            commands::handle_run(bot, chat_id, db, eb, tm, id_str, msg_id).await;
        }
        d if d.starts_with("confirm_stop_") => {
            let id_str = d.strip_prefix("confirm_stop_").unwrap_or(d);
            commands::handle_stop(bot, chat_id, db, eb, tm, id_str, msg_id).await;
        }
        d if d.starts_with("confirm_del_") => {
            if let Ok(id) = d.strip_prefix("confirm_del_").unwrap_or(d).parse::<i32>() {
                let _ = tasks::Entity::delete_by_id(id).exec(db).await;
                if let Err(err) = tm.remove_task_schedule(id).await {
                    reply_success(
                        bot,
                        chat_id,
                        msg_id,
                        &format!("🗑️ 任务已删除，但移除调度失败: {}", err),
                    )
                    .await;
                    return;
                }
                reply_success(bot, chat_id, msg_id, "🗑️ 任务已删除").await;
            }
        }
        d if d.starts_with("confirm_delvar_") => {
            if let Ok(id) = d
                .strip_prefix("confirm_delvar_")
                .unwrap_or(d)
                .parse::<i32>()
            {
                let _ = variables_tasks::Entity::delete_many()
                    .filter(variables_tasks::Column::VariableId.eq(id))
                    .exec(db)
                    .await;
                let _ = variables::Entity::delete_by_id(id).exec(db).await;
                reply_success(bot, chat_id, msg_id, "🗑️ 变量已删除").await;
            }
        }
        d if d.starts_with("confirm_enable_") || d.starts_with("confirm_disable_") => {
            let enable = d.contains("enable");
            let id_str = if enable {
                d.strip_prefix("confirm_enable_").unwrap_or(d)
            } else {
                d.strip_prefix("confirm_disable_").unwrap_or(d)
            };
            if let Ok(id) = id_str.parse::<i32>() {
                use sea_orm::IntoActiveModel;
                if let Ok(Some(t)) = tasks::Entity::find_by_id(id).one(db).await {
                    let mut am = t.into_active_model();
                    am.enabled = Set(enable);
                    let _ = am.update(db).await;
                    handle_task_manage_menu(bot, chat_id, db, id, msg_id.unwrap()).await;
                }
            }
        }
        d if d.starts_with("editfield_") => {
            handle_edit_field_callback(bot, chat_id, d, msg_id, convs).await
        }
        d if d.starts_with("editvarfield_") => {
            handle_edit_var_field_callback(bot, chat_id, d, msg_id, convs).await
        }
        d if d.starts_with("setvarscope_") => {
            handle_set_var_scope(bot, chat_id, d, msg_id, db, convs).await
        }
        d if d.starts_with("setvarscript_") => {
            handle_set_var_script_toggle(bot, chat_id, d, msg_id, convs, db).await
        }
        d if d.starts_with("addenv_") => {
            handle_add_env_callback(bot, chat_id, d, msg_id, convs).await
        }
        "add_cron_skip" => handle_add_cron_skip(bot, chat_id, msg_id, convs).await,
        "cancel_action" => handle_cancel(bot, chat_id, msg_id, convs).await,
        "addvar_done_selection" => handle_var_done(bot, chat_id, msg_id, db, convs).await,
        "add_confirm" => commands::handle_add_confirm(bot, chat_id, db, eb, convs).await,
        _ => warn!("未知回调: {}", data),
    }

    let _ = bot.answer_callback_query(q.id.clone()).await;
}

async fn handle_edit_field_callback(
    bot: &Bot,
    chat_id: ChatId,
    data: &str,
    mid: Option<MessageId>,
    convs: &ConversationStore,
) {
    let parts: Vec<&str> = data
        .strip_prefix("editfield_")
        .unwrap_or(data)
        .splitn(2, '_')
        .collect();
    if parts.len() == 2 {
        if let Ok(id) = parts[0].parse::<i32>() {
            let field = parts[1].to_string();
            let prompt = match field.as_str() {
                "name" => "请输入新名称:",
                "cron" => "请输入新 Cron:",
                "desc" => "请输入新描述:",
                _ => return,
            };
            if let Some(id_msg) = mid {
                let _ = bot.edit_message_text(chat_id, id_msg, prompt).await;
            }
            convs.write().await.insert(
                chat_id.to_string(),
                ConversationState::EditTask {
                    task_id: id,
                    step: EditTaskStep::WaitingValue { field },
                    last_msg_id: mid,
                    created_at: conv::now(),
                },
            );
        }
    }
}

async fn handle_edit_var_field_callback(
    bot: &Bot,
    chat_id: ChatId,
    data: &str,
    mid: Option<MessageId>,
    convs: &ConversationStore,
) {
    let parts: Vec<&str> = data
        .strip_prefix("editvarfield_")
        .unwrap_or(data)
        .splitn(2, '_')
        .collect();
    if parts.len() == 2 {
        if let Ok(id) = parts[0].parse::<i32>() {
            let field = parts[1].to_string();
            if field == "scope" {
                let kb = InlineKeyboardMarkup::new(vec![
                    vec![
                        InlineKeyboardButton::callback(
                            "🌍 全局",
                            format!("setvarscope_{}_Global", id),
                        ),
                        InlineKeyboardButton::callback(
                            "📜 局部",
                            format!("setvarscope_{}_Script", id),
                        ),
                    ],
                    vec![InlineKeyboardButton::callback("❌ 取消", "cancel_action")],
                ]);
                if let Some(m) = mid {
                    let _ = bot
                        .edit_message_text(chat_id, m, "🌍 请选择新的作用域：")
                        .reply_markup(kb)
                        .await;
                }
                return;
            }
            let prompt = match field.as_str() {
                "key" => "请输入新 Key:",
                "value" => "请输入新 Value:",
                _ => return,
            };
            if let Some(id_msg) = mid {
                let _ = bot.edit_message_text(chat_id, id_msg, prompt).await;
            }
            convs.write().await.insert(
                chat_id.to_string(),
                ConversationState::EditVar {
                    var_id: id,
                    step: EditVarStep::WaitingValue { field },
                    last_msg_id: mid,
                    selected_tasks: vec![],
                    created_at: conv::now(),
                },
            );
        }
    }
}

async fn handle_set_var_scope(
    bot: &Bot,
    chat_id: ChatId,
    data: &str,
    mid: Option<MessageId>,
    db: &DatabaseConnection,
    convs: &ConversationStore,
) {
    let parts: Vec<&str> = data
        .strip_prefix("setvarscope_")
        .unwrap_or(data)
        .splitn(2, '_')
        .collect();
    if parts.len() == 2 {
        if let Ok(id) = parts[0].parse::<i32>() {
            let scope = parts[1].to_string();
            if scope == "Script" {
                // Fetch current associated tasks
                let current: Vec<i32> = variables_tasks::Entity::find()
                    .filter(variables_tasks::Column::VariableId.eq(id))
                    .all(db)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|vt| vt.task_id)
                    .collect();
                if let Some(m) = mid {
                    commands::render_variable_task_selector(
                        bot,
                        chat_id,
                        db,
                        m,
                        &current,
                        &format!("setvarscript_{}", id),
                    )
                    .await;
                    convs.write().await.insert(
                        chat_id.to_string(),
                        ConversationState::EditVar {
                            var_id: id,
                            step: EditVarStep::SelectingField,
                            last_msg_id: Some(m),
                            selected_tasks: current,
                            created_at: conv::now(),
                        },
                    );
                }
                return;
            }
            if let Ok(Some(var)) = variables::Entity::find_by_id(id).one(db).await {
                let mut am = var.into_active_model();
                am.scope = Set(scope);
                if let Ok(_) = am.update(db).await {
                    let _ = variables_tasks::Entity::delete_many()
                        .filter(variables_tasks::Column::VariableId.eq(id))
                        .exec(db)
                        .await;
                    if let Some(m) = mid {
                        handle_var_manage_menu(bot, chat_id, db, id, m).await;
                    }
                }
            }
        }
    }
}

async fn handle_set_var_script_toggle(
    bot: &Bot,
    chat_id: ChatId,
    data: &str,
    mid: Option<MessageId>,
    convs: &ConversationStore,
    db: &DatabaseConnection,
) {
    let parts: Vec<&str> = data
        .strip_prefix("setvarscript_")
        .unwrap_or(data)
        .splitn(2, '_')
        .collect();
    if parts.len() == 2 {
        let var_id = parts[0].parse::<i32>().unwrap_or(0);
        let task_id = parts[1].parse::<i32>().unwrap_or(0);
        let mut store = convs.write().await;
        if let Some(ConversationState::EditVar { selected_tasks, .. }) =
            store.get_mut(&chat_id.to_string())
        {
            if let Some(pos) = selected_tasks.iter().position(|&x| x == task_id) {
                selected_tasks.remove(pos);
            } else {
                selected_tasks.push(task_id);
            }
            let current = selected_tasks.clone();
            drop(store);
            if let Some(m) = mid {
                commands::render_variable_task_selector(
                    bot,
                    chat_id,
                    db,
                    m,
                    &current,
                    &format!("setvarscript_{}", var_id),
                )
                .await;
            }
        }
    }
}

async fn handle_add_env_callback(
    bot: &Bot,
    chat_id: ChatId,
    data: &str,
    mid: Option<MessageId>,
    convs: &ConversationStore,
) {
    let env = data.strip_prefix("addenv_").unwrap_or(data).to_string();
    let mut store = convs.write().await;
    if let Some(ConversationState::AddTask {
        data,
        step,
        last_msg_id,
        ..
    }) = store.get_mut(&chat_id.to_string())
    {
        data.env_type = Some(env);
        *step = AddTaskStep::WaitingCron;
        let kb = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
            "⏩ 跳过 (手动执行)",
            "add_cron_skip",
        )]]);
        if let Some(m) = mid.or(*last_msg_id) {
            let _ = bot
                .edit_message_text(
                    chat_id,
                    m,
                    "⏰ 请输入 Cron 表达式 (如: 0 0 * * *)\n发送 'q' 取消",
                )
                .reply_markup(kb)
                .await;
        }
    }
}

async fn handle_add_cron_skip(
    bot: &Bot,
    chat_id: ChatId,
    mid: Option<MessageId>,
    convs: &ConversationStore,
) {
    let mut store = convs.write().await;
    if let Some(ConversationState::AddTask {
        step, last_msg_id, ..
    }) = store.get_mut(&chat_id.to_string())
    {
        *step = AddTaskStep::Confirming;
        let kb = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("✅ 确认创建", "add_confirm"),
            InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
        ]]);
        if let Some(m) = mid.or(*last_msg_id) {
            let _ = bot
                .edit_message_text(chat_id, m, "📋 确认创建该任务？")
                .reply_markup(kb)
                .await;
        }
    }
}

async fn handle_cancel(
    bot: &Bot,
    chat_id: ChatId,
    mid: Option<MessageId>,
    convs: &ConversationStore,
) {
    convs.write().await.remove(&chat_id.to_string());
    if let Some(id) = mid {
        let _ = bot.edit_message_text(chat_id, id, "❌ 操作已取消").await;
    }
}

async fn handle_var_done(
    bot: &Bot,
    chat_id: ChatId,
    mid: Option<MessageId>,
    db: &DatabaseConnection,
    convs: &ConversationStore,
) {
    let state = convs.write().await.remove(&chat_id.to_string());
    match state {
        Some(ConversationState::AddVar {
            data, last_msg_id, ..
        }) => {
            let am = variables::ActiveModel {
                key: Set(data.key.unwrap_or_default()),
                value: Set(data.value.unwrap_or_default()),
                scope: Set(data.scope.unwrap_or_else(|| "Global".to_string())),
                enabled: Set(true),
                ..Default::default()
            };
            if let Ok(v) = am.insert(db).await {
                for tid in data.selected_tasks {
                    let _ = variables_tasks::ActiveModel {
                        variable_id: Set(v.id),
                        task_id: Set(tid),
                        sort_order: Set(v.id),
                    }
                    .insert(db)
                    .await;
                }
                reply_success(
                    bot,
                    chat_id,
                    mid.or(last_msg_id),
                    &format!("✅ 变量 `{}` 已创建", v.key),
                )
                .await;
            }
        }
        Some(ConversationState::EditVar {
            var_id,
            selected_tasks,
            last_msg_id,
            ..
        }) => {
            if let Ok(Some(var)) = variables::Entity::find_by_id(var_id).one(db).await {
                let mut am = var.into_active_model();
                am.scope = Set("Script".to_string());
                if let Ok(_) = am.update(db).await {
                    let _ = variables_tasks::Entity::delete_many()
                        .filter(variables_tasks::Column::VariableId.eq(var_id))
                        .exec(db)
                        .await;
                    for tid in selected_tasks {
                        let _ = variables_tasks::ActiveModel {
                            variable_id: Set(var_id),
                            task_id: Set(tid),
                            sort_order: Set(var_id),
                        }
                        .insert(db)
                        .await;
                    }
                    if let Some(m) = mid.or(last_msg_id) {
                        handle_var_manage_menu(bot, chat_id, db, var_id, m).await;
                    }
                }
            }
        }
        _ => {}
    }
}

pub async fn handle_edit_step(
    bot: &Bot,
    chat_id: ChatId,
    text: &str,
    db: &DatabaseConnection,
    eb: &EventBus,
    tm: &TaskManagerService,
    convs: &ConversationStore,
) -> bool {
    let state = convs.read().await.get(&chat_id.to_string()).cloned();
    if let Some(ConversationState::EditTask {
        task_id,
        step: EditTaskStep::WaitingValue { field },
        last_msg_id,
        ..
    }) = state
    {
        convs.write().await.remove(&chat_id.to_string());
        if let Ok(Some(task)) = tasks::Entity::find_by_id(task_id).one(db).await {
            let mut am = task.into_active_model();
            let val = text.trim();
            match field.as_str() {
                "name" => am.name = Set(val.to_string()),
                "cron" => am.cron_schedule = Set(Some(val.to_string())),
                "desc" => am.description = Set(Some(val.to_string())),
                _ => return true,
            };
            if let Ok(updated) = am.update(db).await {
                if field == "cron" {
                    let _ = tm.update_task_schedule(&updated).await;
                }
                if let Some(mid) = last_msg_id {
                    handle_task_manage_menu(bot, chat_id, db, task_id, mid).await;
                }
                publish_audit(
                    eb,
                    chat_id.to_string(),
                    format!("✏️ 修改任务 {}: {}={}", task_id, field, val),
                );
            }
        }
        return true;
    }
    false
}

pub async fn handle_edit_var_step(
    bot: &Bot,
    chat_id: ChatId,
    text: &str,
    db: &DatabaseConnection,
    eb: &EventBus,
    convs: &ConversationStore,
) -> bool {
    let state = convs.read().await.get(&chat_id.to_string()).cloned();
    if let Some(ConversationState::EditVar {
        var_id,
        step: EditVarStep::WaitingValue { field },
        last_msg_id,
        ..
    }) = state
    {
        convs.write().await.remove(&chat_id.to_string());
        if let Ok(Some(var)) = variables::Entity::find_by_id(var_id).one(db).await {
            let mut am = var.into_active_model();
            let val = text.trim();
            match field.as_str() {
                "key" => am.key = Set(val.to_string()),
                "value" => am.value = Set(val.to_string()),
                _ => return true,
            };
            if let Ok(_) = am.update(db).await {
                if let Some(mid) = last_msg_id {
                    handle_var_manage_menu(bot, chat_id, db, var_id, mid).await;
                }
                let _ = bot
                    .send_message(chat_id, format!("✅ 变量 `{}` 修改成功", field))
                    .await;
                publish_audit(
                    eb,
                    chat_id.to_string(),
                    format!("✏️ 修改变量 {}: {}={}", var_id, field, val),
                );
            }
        }
        return true;
    }
    false
}

async fn handle_quick_task_add(
    bot: &Bot,
    chat_id: ChatId,
    mid: MessageId,
    file_id: String,
    db: &DatabaseConnection,
    eb: &EventBus,
    _tm: &TaskManagerService,
) {
    if let Ok(file) = bot.get_file(FileId(file_id)).await {
        let name = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();
        if let Ok(content) = download_file_content(bot, file.id.to_string()).await {
            // Save file
            let ext = std::path::Path::new(&name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let env_type = match ext {
                "py" => "python3",
                "js" => "node",
                _ => "shell",
            };

            let path = format!("telegram/{}", name);
            let full_dir = std::path::Path::new("data/scripts/telegram");
            if !full_dir.exists() {
                let _ = tokio::fs::create_dir_all(full_dir).await;
            }

            if let Err(e) = tokio::fs::write(format!("data/scripts/{}", path), content).await {
                let _ = bot
                    .edit_message_text(chat_id, mid, format!("❌ 文件写入失败: {}", e))
                    .await;
                return;
            }

            // Create Task
            let am = tasks::ActiveModel {
                name: Set(name.clone()),
                path: Set(path),
                env_type: Set(env_type.to_string()),
                status: Set(TaskStatus::Idle),
                enabled: Set(true),
                user_id: Set(1),
                ..Default::default()
            };

            match am.insert(db).await {
                Ok(t) => {
                    let _ = bot.edit_message_text(chat_id, mid, format!("✅ 任务 `{}` 已创建！\n\nID: `{}`\n环境: `{}`\n\n输入 `/task run {}` 立即执行",
                        escape_tg_markdown(&t.name), t.id, env_type, t.id)).parse_mode(ParseMode::MarkdownV2).await;
                    publish_audit(
                        eb,
                        chat_id.to_string(),
                        format!("🚀 通过 Telegram 快速部署任务: {}", t.name),
                    );
                }
                Err(e) => {
                    let _ = bot
                        .edit_message_text(chat_id, mid, format!("❌ 数据库写入失败: {}", e))
                        .await;
                }
            }
        }
    }
}

async fn handle_package_preview(
    bot: &Bot,
    chat_id: ChatId,
    mid: MessageId,
    file_id: String,
    _db: &DatabaseConnection,
) {
    if let Ok(file) = bot.get_file(FileId(file_id)).await {
        let mut buf = Vec::new();
        if bot.download_file(&file.path, &mut buf).await.is_err() {
            let _ = bot.edit_message_text(chat_id, mid, "❌ 文件下载失败").await;
            return;
        }

        use flate2::read::GzDecoder;
        let decoder = GzDecoder::new(&buf[..]);
        let _package: Result<niupanel_common::models::update::ReleaseInfo, _> =
            rmp_serde::decode::from_read(decoder);

        // Since NiuPackage is in niupanel crate (not shared), I should have moved it to common.
        // For now, I'll use a local minimal definition or skip strict typing if possible.
        // Actually, I'll just save it to staging and let Backend handle it.

        let staging_id = generate_nanoid(12);
        let path = format!("data/shares/import/{}.npack", staging_id);
        let dir = std::path::Path::new("data/shares/import");
        if !dir.exists() {
            let _ = tokio::fs::create_dir_all(dir).await;
        }

        if let Err(e) = tokio::fs::write(&path, buf).await {
            let _ = bot
                .edit_message_text(chat_id, mid, format!("❌ 暂存失败: {}", e))
                .await;
            return;
        }

        // We also need to write a .status file so Backend/Bot can read it
        let status = serde_json::json!({
            "state": "ready",
            "message": "Downloaded via Telegram",
            "progress": 100
        });
        let _ = tokio::fs::write(format!("{}.status", path), status.to_string()).await;

        let msg = format!(
            "📦 *分享包预览*\n\n已成功暂存！\n暂存 ID: `{}`\n\n您可以在移动端一键导入包内的所有任务。",
            staging_id
        );
        let kb = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("📥 一键导入", format!("pkg:import:{}", staging_id)),
            InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
        ]]);

        let _ = bot
            .edit_message_text(chat_id, mid, msg)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(kb)
            .await;
    }
}

async fn handle_package_import(
    bot: &Bot,
    chat_id: ChatId,
    mid: MessageId,
    staging_id: String,
    _db: &DatabaseConnection,
    eb: &EventBus,
) {
    eb.publish(SystemEvent::Telegram(TelegramEvent::PackageImportRequest {
        staging_id: staging_id.clone(),
        user_id: 1,
    }));

    let msg = format!(
        "✅ 任务导入请求已提交！\n\n暂存 ID: `{}`\n请稍后在任务列表中查看。",
        staging_id
    );
    let _ = bot
        .edit_message_text(chat_id, mid, msg)
        .parse_mode(ParseMode::MarkdownV2)
        .await;

    publish_audit(
        eb,
        chat_id.to_string(),
        format!("📥 通过 Telegram 导入分享包: {}", staging_id),
    );
}

async fn monitor_update(bot: Bot, chat_id: ChatId, mid: MessageId, us: UpdateService) {
    tokio::spawn(async move {
        let mut last_progress = 0;
        let mut last_state = UpdateState::Idle;

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;

            let status = match us.get_status() {
                Ok(status) => status,
                Err(_) => break,
            };

            if status.progress != last_progress || status.state != last_state {
                let mut text = format!(
                    "🚀 *系统更新中*\n\n*状态:* `{}`\n",
                    escape_tg_markdown(&status.message)
                );
                let mut kb = None;

                if status.state == UpdateState::Downloading {
                    let bar_len = 10;
                    let filled = (status.progress as usize * bar_len) / 100;
                    let empty = bar_len - filled;
                    let bar = format!("{}{}", "🟩".repeat(filled), "⬜".repeat(empty));
                    text.push_str(&format!("\n*进度:* {} `{}%`", bar, status.progress));

                    // 仅在下载阶段允许手动取消
                    kb = Some(InlineKeyboardMarkup::new(vec![vec![
                        InlineKeyboardButton::callback("❌ 取消更新", "upgrade:cancel"),
                    ]]));
                }

                let mut req = bot
                    .edit_message_text(chat_id, mid, text)
                    .parse_mode(ParseMode::MarkdownV2);
                if let Some(markup) = kb {
                    req = req.reply_markup(markup);
                }
                let _ = req.await;

                last_progress = status.progress;
                last_state = status.state.clone();
            }

            if status.state == UpdateState::Restarting {
                let _ = bot
                    .send_message(chat_id, "✅ *更新已完成*\n系统正在重启，请稍候\\.\\.\\.")
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;
                break;
            }

            if status.state == UpdateState::Error {
                let err_msg = status.error.unwrap_or_else(|| "未知错误".to_string());
                let _ = bot
                    .send_message(
                        chat_id,
                        format!("❌ *更新失败*\n原因: `{}`", escape_tg_markdown(&err_msg)),
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;
                break;
            }

            if status.state == UpdateState::Idle && last_state != UpdateState::Idle {
                // User cancelled or finished
                break;
            }
        }
    });
}

async fn handle_live_logs(
    bot: Bot,
    chat_id: ChatId,
    mid: Option<MessageId>,
    task_id: i32,
    db: &DatabaseConnection,
    tm: &TaskManagerService,
    ls: super::LiveStreamStore,
) {
    if let Some(m) = mid {
        let _ = bot
            .edit_message_text(chat_id, m, "⏳ 正在连接日志流...")
            .await;
    }

    // Attempt to subscribe
    let mut rx = match tm.subscribe_logs(task_id, false) {
        Ok(r) => r,
        Err(_) => {
            if let Some(m) = mid {
                let _ = bot
                    .edit_message_text(chat_id, m, "❌ 连接失败：任务可能未在运行或无实时日志。")
                    .await;
            }
            return;
        }
    };

    if let Some((_, ext_handle)) = ls.write().await.remove(&chat_id.to_string()) {
        ext_handle.abort(); // Cancel any existing active live logs for this user
    }

    // Get DB task name for display
    let task_name = match tasks::Entity::find_by_id(task_id).one(db).await {
        Ok(Some(t)) => escape_tg_markdown(&t.name),
        _ => format!("ID:{}", task_id),
    };

    let kb = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "⏹️ 停止追踪",
        format!("stop_live_{}", task_id),
    )]]);

    let bot_c = bot.clone();
    let cid_str = chat_id.to_string();
    let task_id_c = task_id;
    let ls_c = ls.clone();

    let handle = tokio::spawn(async move {
        let mut buffer = String::new();
        let mut last_update = tokio::time::Instant::now();
        let debounce_duration = tokio::time::Duration::from_millis(1500);
        let mut is_first_msg = true;

        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            buffer.push_str(&event.content);
                            if !buffer.ends_with('\n') {
                                buffer.push('\n');
                            }

                            // Check if it's time to flush manually
                            if last_update.elapsed() >= debounce_duration {
                                let mut text = buffer.clone();
                                if text.len() > 3000 {
                                    text = format!("... (被截断)\n{}", &text[text.len() - 3000..]);
                                }
                                let escaped = text.replace('\\', "\\\\").replace('`', "\\`").replace('$', "\\$");
                                let msg_text = format!("📺 *追踪: {}*\n```\n{}\n```", task_name, escaped);

                                if let Some(m) = mid {
                                    let _ = bot_c.edit_message_text(chat_id, m, msg_text)
                                        .parse_mode(ParseMode::MarkdownV2)
                                        .reply_markup(kb.clone())
                                        .await;
                                }
                                buffer.clear();
                                last_update = tokio::time::Instant::now();
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            // Stream ended
                            if let Some(m) = mid {
                                let mut text = buffer.clone();
                                if text.len() > 3000 {
                                    text = format!("... (被截断)\n{}", &text[text.len() - 3000..]);
                                }
                                let escaped = text.replace('\\', "\\\\").replace('`', "\\`").replace('$', "\\$");
                                let msg_text = format!("📺 *追踪: {}* \\[已结束\\]\n```\n{}\n```", task_name, escaped);
                                let close_kb = InlineKeyboardMarkup::new(vec![vec![
                                    InlineKeyboardButton::callback("⬅️ 返回", format!("task_manage_{}", task_id_c)),
                                ]]);
                                let _ = bot_c.edit_message_text(chat_id, m, msg_text)
                                    .parse_mode(ParseMode::MarkdownV2)
                                    .reply_markup(close_kb)
                                    .await;
                            }
                            break;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            buffer.push_str("[...丢弃了部分日志...]\n");
                        }
                    }
                }
                _ = tokio::time::sleep(debounce_duration) => {
                    if !buffer.is_empty() || is_first_msg {
                        is_first_msg = false;
                        let mut text = buffer.clone();
                        if text.len() > 3000 {
                            text = format!("... (被截断)\n{}", &text[text.len() - 3000..]);
                        }
                        let escaped = text.replace('\\', "\\\\").replace('`', "\\`").replace('$', "\\$");
                        let msg_text = format!("📺 *追踪: {}*\n```\n{}\n```", task_name, escaped);

                        if let Some(m) = mid {
                            let _ = bot_c.edit_message_text(chat_id, m, msg_text)
                                .parse_mode(ParseMode::MarkdownV2)
                                .reply_markup(kb.clone())
                                .await;
                        }
                        buffer.clear();
                        last_update = tokio::time::Instant::now();
                    }
                }
            }
        }

        // Remove from store when naturally finished
        ls_c.write().await.remove(&cid_str);
    });

    ls.write()
        .await
        .insert(chat_id.to_string(), (task_id, handle));
}
