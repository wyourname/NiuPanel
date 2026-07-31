use super::*;

pub(super) async fn handle_quick_task_add(
    bot: &Bot,
    chat_id: ChatId,
    mid: MessageId,
    file_id: String,
    db: &DatabaseConnection,
    eb: &EventBus,
    tm: &TaskManagerService,
) {
    let file = match bot.get_file(FileId(file_id)).await {
        Ok(file) => file,
        Err(err) => {
            let _ = bot
                .edit_message_text(chat_id, mid, format!("❌ 获取文件失败: {err}"))
                .await;
            return;
        }
    };
    let name = std::path::Path::new(&file.path)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("unnamed.sh")
        .to_string();
    let content = match download_file_content(bot, file.id.to_string()).await {
        Ok(content) if !content.is_empty() => content,
        Ok(_) => {
            let _ = bot
                .edit_message_text(chat_id, mid, "❌ 文件内容为空，任务未创建")
                .await;
            return;
        }
        Err(err) => {
            let _ = bot
                .edit_message_text(chat_id, mid, format!("❌ 下载文件失败: {err}"))
                .await;
            return;
        }
    };
    let ext = std::path::Path::new(&name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let (env_type, storage_ext) = match ext.as_str() {
        "py" => ("python3", "py"),
        "js" | "ts" => ("node", "js"),
        _ => ("shell", "sh"),
    };
    let user_id = match task_owner_id(db).await {
        Ok(id) => id,
        Err(err) => {
            let _ = bot
                .edit_message_text(chat_id, mid, format!("❌ 无法确定任务所有者: {err}"))
                .await;
            return;
        }
    };
    let (path, full_path) = match save_telegram_script(storage_ext, content).await {
        Ok(saved) => saved,
        Err(err) => {
            let _ = bot
                .edit_message_text(chat_id, mid, format!("❌ 文件写入失败: {err}"))
                .await;
            return;
        }
    };

    let am = tasks::ActiveModel {
        name: Set(name.clone()),
        path: Set(path),
        env_type: Set(env_type.to_string()),
        status: Set(TaskStatus::Idle),
        enabled: Set(true),
        user_id: Set(user_id),
        ..Default::default()
    };

    match am.insert(db).await {
        Ok(t) => {
            if let Err(schedule_error) = tm.update_task_schedule(&t).await {
                let schedule_cleanup = tm.remove_task_schedule(t.id).await;
                let database_cleanup =
                    niupanel_core::task::delete_task_records(db, vec![t.id], true, false).await;
                let _ = bot
                    .edit_message_text(
                        chat_id,
                        mid,
                        format!(
                            "❌ 创建任务调度失败: {}（调度清理: {}，任务回滚: {}）",
                            schedule_error,
                            if schedule_cleanup.is_ok() {
                                "成功"
                            } else {
                                "失败"
                            },
                            if database_cleanup.is_ok() {
                                "成功"
                            } else {
                                "失败"
                            }
                        ),
                    )
                    .await;
                return;
            }
            let _ = bot.edit_message_text(chat_id, mid, format!("✅ 任务 `{}` 已创建！\n\nID: `{}`\n环境: `{}`\n\n输入 `/task run {}` 立即执行",
                escape_tg_markdown(&t.name), t.id, env_type, t.id)).parse_mode(ParseMode::MarkdownV2).await;
            publish_audit(
                eb,
                chat_id.to_string(),
                format!("🚀 通过 Telegram 快速部署任务: {}", t.name),
            );
        }
        Err(err) => {
            if let Err(cleanup_err) = tokio::fs::remove_file(&full_path).await {
                warn!(
                    "failed to clean Telegram script {} after database error: {}",
                    full_path.display(),
                    cleanup_err
                );
            }
            let _ = bot
                .edit_message_text(chat_id, mid, format!("❌ 数据库写入失败: {err}"))
                .await;
        }
    }
}

pub(super) async fn handle_package_preview(
    bot: &Bot,
    chat_id: ChatId,
    mid: MessageId,
    file_id: String,
    _db: &DatabaseConnection,
) {
    const MAX_PACKAGE_SIZE: usize = 100 * 1024 * 1024;

    let file = match bot.get_file(FileId(file_id)).await {
        Ok(file) => file,
        Err(err) => {
            let _ = bot
                .edit_message_text(chat_id, mid, format!("❌ 获取文件失败: {err}"))
                .await;
            return;
        }
    };
    let mut buf = Vec::new();
    if let Err(err) = bot.download_file(&file.path, &mut buf).await {
        let _ = bot
            .edit_message_text(chat_id, mid, format!("❌ 文件下载失败: {err}"))
            .await;
        return;
    }
    if buf.len() > MAX_PACKAGE_SIZE {
        let _ = bot
            .edit_message_text(chat_id, mid, "❌ 分享包超过 100 MB 上限")
            .await;
        return;
    }
    if !buf.starts_with(&[0x1f, 0x8b]) {
        let _ = bot
            .edit_message_text(chat_id, mid, "❌ 文件不是有效的 Gzip 分享包")
            .await;
        return;
    }

    let staging_id = nanoid::nanoid!(12);
    let dir = &Config::global().share_import_dir;
    if let Err(err) = tokio::fs::create_dir_all(dir).await {
        let _ = bot
            .edit_message_text(chat_id, mid, format!("❌ 创建暂存目录失败: {err}"))
            .await;
        return;
    }
    let package_path = dir.join(format!("{staging_id}.npack"));
    let status_path = dir.join(format!("{staging_id}.status"));

    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    if let Err(err) = async {
        use tokio::io::AsyncWriteExt;
        let mut file = options.open(&package_path).await?;
        file.write_all(&buf).await?;
        file.sync_all().await
    }
    .await
    {
        let _ = bot
            .edit_message_text(chat_id, mid, format!("❌ 暂存失败: {err}"))
            .await;
        return;
    }

    let status = serde_json::json!({
        "state": "ready",
        "message": "Downloaded via Telegram",
        "progress": 100
    });
    if let Err(err) = tokio::fs::write(&status_path, status.to_string()).await {
        if let Err(cleanup_err) = tokio::fs::remove_file(&package_path).await {
            warn!(
                "failed to clean Telegram package {}: {}",
                package_path.display(),
                cleanup_err
            );
        }
        let _ = bot
            .edit_message_text(chat_id, mid, format!("❌ 写入暂存状态失败: {err}"))
            .await;
        return;
    }

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

pub(super) async fn handle_package_import(
    bot: &Bot,
    chat_id: ChatId,
    mid: MessageId,
    staging_id: String,
    db: &DatabaseConnection,
    eb: &EventBus,
) {
    if staging_id.len() < 8
        || staging_id.len() > 64
        || !staging_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        let _ = bot
            .edit_message_text(chat_id, mid, "❌ 无效的分享包暂存 ID")
            .await;
        return;
    }
    let user_id = match task_owner_id(db).await {
        Ok(id) => id,
        Err(err) => {
            let _ = bot
                .edit_message_text(chat_id, mid, format!("❌ 无法确定导入用户: {err}"))
                .await;
            return;
        }
    };
    if !Config::global()
        .share_import_dir
        .join(format!("{staging_id}.npack"))
        .is_file()
    {
        let _ = bot
            .edit_message_text(chat_id, mid, "❌ 分享包暂存文件不存在或已失效")
            .await;
        return;
    }

    eb.publish(SystemEvent::Telegram(TelegramEvent::PackageImportRequest {
        staging_id: staging_id.clone(),
        user_id,
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

pub(super) async fn monitor_update(bot: Bot, chat_id: ChatId, mid: MessageId, us: UpdateService) {
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

pub(super) async fn handle_live_logs(
    bot: Bot,
    chat_id: ChatId,
    mid: Option<MessageId>,
    task_id: i32,
    db: &DatabaseConnection,
    tm: &TaskManagerService,
    ls: super::super::LiveStreamStore,
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
