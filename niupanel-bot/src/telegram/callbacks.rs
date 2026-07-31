use niupanel_common::logger::{debug, warn};
use niupanel_common::models::update::UpdateState;
use niupanel_common::variable::{normalize_variable_key, validate_variable_value};
use niupanel_common::{config::Config, escape_tg_markdown};
use niupanel_core::event_bus::{AuthEvent, EventBus, SystemEvent, TelegramEvent};
use niupanel_core::settings::update::UpdateService;
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::task_status::TaskStatus;
use niupanel_entity::{tasks, variables, variables_tasks};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set, TransactionTrait,
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

            if let Some(script) = take_pending_workflow_approval(&nonce, chat_id).await {
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
            let rejected = discard_pending_workflow_approval(&nonce, chat_id).await;

            if let Some(mid) = msg_id {
                let _ = bot
                    .edit_message_text(
                        chat_id,
                        mid,
                        if rejected {
                            "🚫 已取消该自动化操作"
                        } else {
                            "❌ 取消失败：该操作已过期或不存在"
                        },
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;
            }
        }
        "upgrade:cancel" => {
            if let Some(mid) = msg_id {
                let message = match us.cancel_update().await {
                    Ok(()) => "⌛ 正在取消更新...".to_string(),
                    Err(err) => format!("❌ 取消更新失败: {}", err),
                };
                let _ = bot.edit_message_text(chat_id, mid, message).await;
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
                if let Some(mid) = msg_id {
                    handle_task_manage_menu(bot, chat_id, db, id, mid).await;
                }
            }
        }
        d if d.starts_with("var_manage_") => {
            if let Ok(id) = d.strip_prefix("var_manage_").unwrap_or(d).parse::<i32>() {
                if let Some(mid) = msg_id {
                    handle_var_manage_menu(bot, chat_id, db, id, mid).await;
                }
            }
        }
        d if d.starts_with("pip") || d.starts_with("pnpm") || d.starts_with("apt") => {
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
                let task = match tasks::Entity::find_by_id(id).one(db).await {
                    Ok(Some(task)) => task,
                    Ok(None) => {
                        reply_success(bot, chat_id, msg_id, "❌ 任务不存在或已被删除").await;
                        return;
                    }
                    Err(err) => {
                        reply_success(bot, chat_id, msg_id, &format!("❌ 查询任务失败: {err}"))
                            .await;
                        return;
                    }
                };
                if let Err(err) = tm.remove_task_schedule(id).await {
                    reply_success(
                        bot,
                        chat_id,
                        msg_id,
                        &format!("❌ 移除任务调度失败，任务未删除: {}", err),
                    )
                    .await;
                    return;
                }
                if let Err(err) =
                    niupanel_core::task::delete_task_records(db, vec![id], true, false).await
                {
                    let restore_message = match tasks::Entity::find_by_id(id).one(db).await {
                        Ok(Some(_)) => match tm.update_task_schedule(&task).await {
                            Ok(()) => "；原调度已恢复".to_string(),
                            Err(restore_err) => {
                                format!("；原调度恢复失败: {}", restore_err)
                            }
                        },
                        Ok(None) => "；任务记录已删除，未恢复调度".to_string(),
                        Err(check_err) => {
                            format!("；无法确认删除状态，未恢复调度: {}", check_err)
                        }
                    };
                    reply_success(
                        bot,
                        chat_id,
                        msg_id,
                        &format!("❌ 删除任务失败: {}{}", err, restore_message),
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
                let result: Result<u64, sea_orm::DbErr> = async {
                    let txn = db.begin().await?;
                    variables_tasks::Entity::delete_many()
                        .filter(variables_tasks::Column::VariableId.eq(id))
                        .exec(&txn)
                        .await?;
                    let deleted = variables::Entity::delete_by_id(id).exec(&txn).await?;
                    txn.commit().await?;
                    Ok(deleted.rows_affected)
                }
                .await;

                match result {
                    Ok(0) => reply_success(bot, chat_id, msg_id, "❌ 变量不存在或已被删除").await,
                    Ok(_) => reply_success(bot, chat_id, msg_id, "🗑️ 变量已删除").await,
                    Err(err) => {
                        reply_success(bot, chat_id, msg_id, &format!("❌ 删除变量失败: {err}"))
                            .await
                    }
                }
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
                let Some(mid) = msg_id else {
                    return;
                };
                match tasks::Entity::find_by_id(id).one(db).await {
                    Ok(Some(task)) => {
                        let previous = task.clone();
                        let mut am = task.into_active_model();
                        am.enabled = Set(enable);
                        match am.update(db).await {
                            Ok(updated) => {
                                if let Err(schedule_err) = tm.update_task_schedule(&updated).await {
                                    let mut rollback = previous.clone().into_active_model();
                                    rollback.enabled = Set(previous.enabled);
                                    let rollback_result = rollback.update(db).await;
                                    if enable {
                                        let _ = tm.remove_task_schedule(id).await;
                                    } else {
                                        let _ = tm.update_task_schedule(&previous).await;
                                    }
                                    let rollback_suffix = rollback_result
                                        .err()
                                        .map(|err| format!("；数据库回滚失败: {}", err))
                                        .unwrap_or_default();
                                    let _ = bot
                                        .edit_message_text(
                                            chat_id,
                                            mid,
                                            format!(
                                                "❌ 更新任务调度失败: {}{}",
                                                schedule_err, rollback_suffix
                                            ),
                                        )
                                        .await;
                                    return;
                                }
                                handle_task_manage_menu(bot, chat_id, db, id, mid).await;
                            }
                            Err(err) => {
                                let _ = bot
                                    .edit_message_text(
                                        chat_id,
                                        mid,
                                        format!("❌ 更新任务状态失败: {}", err),
                                    )
                                    .await;
                            }
                        }
                    }
                    Ok(None) => {
                        let _ = bot
                            .edit_message_text(chat_id, mid, "⚠️ 任务不存在或已被删除")
                            .await;
                    }
                    Err(err) => {
                        let _ = bot
                            .edit_message_text(chat_id, mid, format!("❌ 查询任务失败: {}", err))
                            .await;
                    }
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
        "add_confirm" => commands::handle_add_confirm(bot, chat_id, db, eb, tm, convs).await,
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
            if scope != "Global" {
                return;
            }
            if let Ok(Some(var)) = variables::Entity::find_by_id(id).one(db).await {
                let Ok(txn) = db.begin().await else {
                    return;
                };
                let mut am = var.into_active_model();
                am.scope = Set(scope);
                am.scope_id = Set(None);
                am.updated_at = Set(chrono::Utc::now());
                if am.update(&txn).await.is_ok()
                    && variables_tasks::Entity::delete_many()
                        .filter(variables_tasks::Column::VariableId.eq(id))
                        .exec(&txn)
                        .await
                        .is_ok()
                    && txn.commit().await.is_ok()
                {
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
            let scope = data.scope.unwrap_or_else(|| "Global".to_string());
            if scope != "Global" && scope != "Script" {
                reply_success(bot, chat_id, mid.or(last_msg_id), "❌ 变量作用域无效").await;
                return;
            }
            if scope == "Script" && data.selected_tasks.is_empty() {
                reply_success(
                    bot,
                    chat_id,
                    mid.or(last_msg_id),
                    "❌ 脚本变量必须至少关联一个任务",
                )
                .await;
                return;
            }
            let key = match normalize_variable_key(data.key.as_deref().unwrap_or_default()) {
                Ok(key) => key,
                Err(error) => {
                    reply_success(bot, chat_id, mid.or(last_msg_id), &format!("❌ {error}")).await;
                    return;
                }
            };
            let value = data.value.unwrap_or_default();
            if let Err(error) = validate_variable_value(&value) {
                reply_success(bot, chat_id, mid.or(last_msg_id), &format!("❌ {error}")).await;
                return;
            }
            let selected_tasks = if scope == "Script" {
                data.selected_tasks
            } else {
                vec![]
            };
            let primary_task_id = selected_tasks.first().copied();
            let am = variables::ActiveModel {
                key: Set(key),
                value: Set(value),
                scope: Set(scope),
                scope_id: Set(primary_task_id),
                enabled: Set(true),
                ..Default::default()
            };
            let Ok(txn) = db.begin().await else {
                return;
            };
            if let Ok(v) = am.insert(&txn).await {
                let mut succeeded = true;
                for tid in selected_tasks {
                    if (variables_tasks::ActiveModel {
                        variable_id: Set(v.id),
                        task_id: Set(tid),
                        sort_order: Set(v.id),
                    })
                    .insert(&txn)
                    .await
                    .is_err()
                    {
                        succeeded = false;
                        break;
                    }
                }
                if !succeeded || txn.commit().await.is_err() {
                    return;
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
            if selected_tasks.is_empty() {
                reply_success(
                    bot,
                    chat_id,
                    mid.or(last_msg_id),
                    "❌ 脚本变量必须至少关联一个任务",
                )
                .await;
                return;
            }
            if let Ok(Some(var)) = variables::Entity::find_by_id(var_id).one(db).await {
                let Ok(txn) = db.begin().await else {
                    return;
                };
                let mut am = var.into_active_model();
                am.scope = Set("Script".to_string());
                am.scope_id = Set(selected_tasks.first().copied());
                am.updated_at = Set(chrono::Utc::now());
                if am.update(&txn).await.is_ok() {
                    if variables_tasks::Entity::delete_many()
                        .filter(variables_tasks::Column::VariableId.eq(var_id))
                        .exec(&txn)
                        .await
                        .is_err()
                    {
                        return;
                    }
                    let mut succeeded = true;
                    for tid in selected_tasks {
                        if (variables_tasks::ActiveModel {
                            variable_id: Set(var_id),
                            task_id: Set(tid),
                            sort_order: Set(var_id),
                        })
                        .insert(&txn)
                        .await
                        .is_err()
                        {
                            succeeded = false;
                            break;
                        }
                    }
                    if !succeeded || txn.commit().await.is_err() {
                        return;
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
        match tasks::Entity::find_by_id(task_id).one(db).await {
            Ok(Some(task)) => {
                let previous = task.clone();
                let mut am = task.into_active_model();
                let val = text.trim();
                match field.as_str() {
                    "name" => am.name = Set(val.to_string()),
                    "cron" => {
                        am.cron_schedule = Set((!val.is_empty()).then(|| val.to_string()));
                    }
                    "desc" => am.description = Set(Some(val.to_string())),
                    _ => return true,
                };
                match am.update(db).await {
                    Ok(updated) => {
                        if field == "cron"
                            && let Err(schedule_error) = tm.update_task_schedule(&updated).await
                        {
                            let database_rollback = previous
                                .clone()
                                .into_active_model()
                                .reset_all()
                                .update(db)
                                .await;
                            let schedule_rollback = tm.update_task_schedule(&previous).await;
                            reply_success(
                                bot,
                                chat_id,
                                last_msg_id,
                                &format!(
                                    "❌ 更新 Cron 调度失败: {}（数据库回滚: {}，调度回滚: {}）",
                                    schedule_error,
                                    if database_rollback.is_ok() {
                                        "成功"
                                    } else {
                                        "失败"
                                    },
                                    if schedule_rollback.is_ok() {
                                        "成功"
                                    } else {
                                        "失败"
                                    }
                                ),
                            )
                            .await;
                            return true;
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
                    Err(error) => {
                        reply_success(
                            bot,
                            chat_id,
                            last_msg_id,
                            &format!("❌ 更新任务失败: {}", error),
                        )
                        .await;
                    }
                }
            }
            Ok(None) => {
                reply_success(bot, chat_id, last_msg_id, "⚠️ 任务不存在或已被删除").await;
            }
            Err(error) => {
                reply_success(
                    bot,
                    chat_id,
                    last_msg_id,
                    &format!("❌ 查询任务失败: {}", error),
                )
                .await;
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
                "key" => match normalize_variable_key(val) {
                    Ok(key) => am.key = Set(key),
                    Err(error) => {
                        let _ = bot.send_message(chat_id, format!("❌ {error}")).await;
                        return true;
                    }
                },
                "value" => {
                    if let Err(error) = validate_variable_value(val) {
                        let _ = bot.send_message(chat_id, format!("❌ {error}")).await;
                        return true;
                    }
                    am.value = Set(val.to_string());
                }
                _ => return true,
            };
            am.updated_at = Set(chrono::Utc::now());
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
                    format!(
                        "✏️ 修改变量 {}: {}={}",
                        var_id,
                        field,
                        if field == "value" { "***" } else { val }
                    ),
                );
            }
        }
        return true;
    }
    false
}

mod operations;

use operations::{
    handle_live_logs, handle_package_import, handle_package_preview, handle_quick_task_add,
    monitor_update,
};
