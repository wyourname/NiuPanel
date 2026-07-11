use niupanel_common::config::Config;
use niupanel_common::escape_tg_markdown;
use niupanel_core::event_bus::EventBus;
use niupanel_core::task_manager::service::TaskManagerService;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

use super::helpers::*;

// ──────────────────────────────── /env list ────────────────────────────────

pub async fn handle_envs(bot: &Bot, chat_id: ChatId) {
    let mut text = String::from("🌐 *运行环境列表*\n\n");

    // Python 环境
    let runtimes_dir = std::path::PathBuf::from(&Config::global().runtimes_dir);
    let py_dir = runtimes_dir.join("python");
    if py_dir.exists() {
        if let Ok(mut entries) = tokio::fs::read_dir(&py_dir).await {
            let mut found_py = false;
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry
                    .file_type()
                    .await
                    .map(|ft| ft.is_dir())
                    .unwrap_or(false)
                {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("venv_") {
                        let version = name.strip_prefix("venv_").unwrap_or(&name);
                        if !found_py {
                            text.push_str("🐍 *Python*\n");
                            found_py = true;
                        }
                        text.push_str(&format!(
                            "  • `{}` \\({}\\)\n",
                            escape_tg_markdown(&name),
                            escape_tg_markdown(version)
                        ));
                    }
                }
            }
            if !found_py {
                text.push_str("🐍 *Python*: 未安装虚拟环境\n");
            }
        }
    } else {
        text.push_str("🐍 *Python*: 未安装虚拟环境\n");
    }

    text.push_str("\n");

    // Node 环境 - 从 fnm list 获取已安装版本
    let fnm_bin = niupanel_core::script::interpreter::node::NodeEnvironment::get_fnm_bin();
    let fnm_output = tokio::process::Command::new(&fnm_bin)
        .arg("list")
        .output()
        .await;
    match fnm_output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut found_node = false;
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() || line.contains("system") {
                    continue;
                }
                let is_active = line.contains("default");
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(ver) = parts.iter().find(|&&p| p.starts_with('v')) {
                    let ver_clean = ver.strip_prefix('v').unwrap_or(ver);
                    if !found_node {
                        text.push_str("📦 *Node\\.js* \\(fnm 全局版本\\)\n");
                        found_node = true;
                    }
                    let active_mark = if is_active { " ✅" } else { "" };
                    text.push_str(&format!(
                        "  • `{}`{}\n",
                        escape_tg_markdown(ver_clean),
                        active_mark
                    ));
                }
            }
            if !found_node {
                text.push_str("📦 *Node\\.js*: 未安装任何版本\n");
            }
        }
        Err(_) => {
            text.push_str("📦 *Node\\.js*: fnm 未安装或不在 PATH\n");
        }
    }

    text.push_str("🐚 *Shell*: 系统默认 \\(Debian 12\\)\n");
    text.push_str("\n📌 使用 `/env <pip|npm|apt>` 管理环境依赖");

    let _ = bot
        .send_message(chat_id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .await;
}

// ──────────────────────────────── /env pip ────────────────────────────────

pub async fn handle_pip(
    bot: &Bot,
    chat_id: ChatId,
    tm: &TaskManagerService,
    eb: &EventBus,
    args: &str,
) {
    let parts: Vec<&str> = args.trim().split_whitespace().collect();
    let sub_cmd = parts.first().cloned().unwrap_or("");
    let pkg_arg = parts.get(1..).unwrap_or_default().join(" ");

    let venvs = list_python_venvs();
    if venvs.is_empty() {
        let _ = bot
            .send_message(chat_id, "❌ 未找到 Python 虚拟环境。")
            .await;
        return;
    }

    if !["install", "uninstall", "remove", "list"].contains(&sub_cmd) {
        let _ = bot.send_message(chat_id, "📦 *pip 用法*\n\n`/env pip install <包名>`\n`/env pip uninstall <包名>`\n`/env pip list`").parse_mode(ParseMode::MarkdownV2).await;
        return;
    }

    if venvs.len() > 1 && !args.contains("--venv=") {
        let mut kb = vec![];
        for venv in venvs {
            let pkg_cb = pkg_arg.replace(' ', ",");
            kb.push(vec![InlineKeyboardButton::callback(
                format!("🐍 {}", venv),
                format!("pip:select:{}:{}:{}", sub_cmd, venv, pkg_cb),
            )]);
        }
        let _ = bot
            .send_message(chat_id, "🔍 请选择目标环境：")
            .reply_markup(InlineKeyboardMarkup::new(kb))
            .await;
        return;
    }

    let target = venvs[0].clone();
    process_pip_action(bot, chat_id, sub_cmd, &pkg_arg, &target, tm, eb).await;
}

pub(crate) async fn process_pip_action(
    bot: &Bot,
    chat_id: ChatId,
    sub_cmd: &str,
    pkg: &str,
    venv: &str,
    _tm: &TaskManagerService,
    _eb: &EventBus,
) {
    let pkg_cb = pkg.replace(' ', ",");
    match sub_cmd {
        "install" if !pkg.is_empty() => {
            let kb = InlineKeyboardMarkup::new(vec![vec![
                InlineKeyboardButton::callback(
                    "✅ 确认安装",
                    format!("pip:install:{}:{}", venv, pkg_cb),
                ),
                InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
            ]]);
            let _ = bot
                .send_message(
                    chat_id,
                    format!(
                        "📦 *确认安装 Python 包*\n\n*环境:* `{}`\n*包:* `{}`",
                        escape_tg_markdown(venv),
                        escape_tg_markdown(pkg)
                    ),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .reply_markup(kb)
                .await;
        }
        "uninstall" | "remove" if !pkg.is_empty() => {
            let kb = InlineKeyboardMarkup::new(vec![vec![
                InlineKeyboardButton::callback(
                    "✅ 确认卸载",
                    format!("pip:uninstall:{}:{}", venv, pkg_cb),
                ),
                InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
            ]]);
            let _ = bot
                .send_message(
                    chat_id,
                    format!(
                        "📦 *确认卸载 Python 包*\n\n*环境:* `{}`\n*包:* `{}`",
                        escape_tg_markdown(venv),
                        escape_tg_markdown(pkg)
                    ),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .reply_markup(kb)
                .await;
        }
        "list" => {
            let venv_path = std::path::PathBuf::from(&Config::global().runtimes_dir)
                .join("python")
                .join(venv);
            // Use UV for listing packages - much more reliable and fast
            match tokio::process::Command::new("uv")
                .args(["pip", "list"])
                .env("VIRTUAL_ENV", &venv_path)
                .output()
                .await
            {
                Ok(out) => {
                    let output = String::from_utf8_lossy(&out.stdout);
                    let truncated = if output.len() > 3000 {
                        &output[..3000]
                    } else {
                        &output
                    };
                    let _ = bot
                        .send_message(
                            chat_id,
                            format!(
                                "📦 *Python 包列表* \\(`{}`\\)\n```\n{}\n```",
                                escape_tg_markdown(venv),
                                escape_tg_markdown(truncated)
                            ),
                        )
                        .parse_mode(ParseMode::MarkdownV2)
                        .await;
                }
                Err(e) => {
                    let _ = bot
                        .send_message(chat_id, format!("❌ 执行失败: {}", e))
                        .await;
                }
            }
        }
        _ => {}
    }
}

// ──────────────────────────────── Callback Handler ────────────────────────────────

pub async fn handle_dep_callback(
    bot: &Bot,
    chat_id: ChatId,
    mid: teloxide::types::MessageId,
    data: &str,
    tm: &TaskManagerService,
    eb: &EventBus,
    job_msgs: &super::JobMessageStore,
) -> bool {
    niupanel_common::logger::info!("收到依赖管理回调: {} 来自 {}", data, chat_id);
    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() < 2 {
        return false;
    }

    let action = parts[0];
    let sub = parts[1];

    if sub == "select" && parts.len() == 5 {
        let venv = parts[3];
        let pkg = parts[4].replace(',', " ");
        if action == "npm" {
            // 全局单例模式下 npm 不再需要 venv 参数
            process_npm_action(bot, chat_id, parts[2], &pkg).await;
        } else {
            process_pip_action(bot, chat_id, parts[2], &pkg, venv, tm, eb).await;
        }
        return true;
    }

    let bot_c = bot.clone();
    let cid = chat_id;
    let runtimes_dir = Config::global().runtimes_dir.clone();

    // We'll use 'uv' as the primary engine for Python/PIP actions
    let (cmd_bin, cmd_args, job_name, env_vars, cmd_cwd) = match action {
        "pip" if parts.len() == 4 => {
            let venv = parts[2];
            let pkg_str = parts[3].replace(',', " ");
            let venv_path = std::path::PathBuf::from(&runtimes_dir)
                .join("python")
                .join(venv);

            let mut args = vec!["pip".to_string(), sub.to_string()];
            if sub == "uninstall" {
                args.push("-y".to_string());
            }

            // Split multiple packages
            for p in pkg_str.split_whitespace() {
                args.push(p.to_string());
            }

            let mut envs = std::collections::HashMap::new();
            envs.insert(
                "VIRTUAL_ENV".to_string(),
                venv_path.to_string_lossy().to_string(),
            );

            (
                "uv".to_string(),
                args,
                format!("uv pip {} {}", sub, pkg_str),
                Some(envs),
                None::<String>, // no special cwd
            )
        }
        "npm" if parts.len() >= 3 => {
            // parts: ["npm", sub, pkg_str]  (全局单例，无需 venv 选择)
            let pkg_str = parts[2..].join(":").replace(',', " ");
            let scripts_dir = Config::global().scripts_dir.clone();
            let fnm_bin = niupanel_core::script::interpreter::node::NodeEnvironment::get_fnm_bin();
            let is_uninstall = sub == "uninstall" || sub == "remove";

            let mut args = vec![
                "exec".to_string(),
                "--using=default".to_string(),
                "npm".to_string(),
                if is_uninstall { "uninstall" } else { "install" }.to_string(),
            ];
            for p in pkg_str.split_whitespace() {
                args.push(p.to_string());
            }

            (
                fnm_bin.to_string_lossy().to_string(), // convert PathBuf -> String
                args,
                format!("npm {} {} (data/scripts)", sub, pkg_str),
                None::<std::collections::HashMap<String, String>>,
                Some(scripts_dir.display().to_string()), // CWD = data/scripts for local node_modules
            )
        }
        "apt" if parts.len() == 3 => {
            let pkg_str = parts[2].replace(',', " ");
            let apt_sub = if sub == "remove" { "remove" } else { "install" };
            let mut args = vec![apt_sub.to_string(), "-y".to_string()];

            // Split multiple packages
            for p in pkg_str.split_whitespace() {
                args.push(p.to_string());
            }

            (
                "apt-get".to_string(),
                args,
                format!("apt {} {}", apt_sub, pkg_str),
                None,
                None::<String>,
            )
        }
        _ => return false,
    };

    let _ = bot_c
        .edit_message_text(
            cid,
            mid,
            format!(
                "⏳ 正在提交运维任务: `{}`...",
                escape_tg_markdown(&job_name)
            ),
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await;

    let tm_c = tm.clone();
    let eb_c = eb.clone();
    let j_name = job_name.clone();
    let job_msgs_c = job_msgs.clone();

    tokio::spawn(async move {
        niupanel_common::logger::info!("开始执行系统任务: {} (ID: {})", j_name, cid);
        let inner_j_name = j_name.clone();
        let cmd_args_copy = cmd_args.clone();
        match tm_c
            .submit_system_task(j_name.clone(), move |tx| async move {
                niupanel_common::logger::info!("系统任务已启动: {}", inner_j_name);
                // Check if command exists
                if let Err(e) = tokio::process::Command::new(&cmd_bin)
                    .arg("--version")
                    .output()
                    .await
                {
                    niupanel_common::logger::error!(
                        "系统任务失败，命令未找到: {} Error: {}",
                        cmd_bin,
                        e
                    );
                    let _ = tx.send(format!(
                        "❌ 错误: 找不到命令 '{}'。请确认该环境已正确安装系统依赖。\n详细错误: {}",
                        cmd_bin, e
                    ));
                    return;
                }

                niupanel_common::logger::info!("命令存在，准备运行: {}", cmd_bin);
                let mut cmd = tokio::process::Command::new(&cmd_bin);
                for arg in cmd_args {
                    cmd.arg(arg);
                }
                if let Some(envs) = env_vars {
                    cmd.envs(envs);
                }
                if let Some(cwd) = cmd_cwd {
                    cmd.current_dir(cwd);
                }

                niupanel_common::logger::info!(
                    "正在执行指令: {} args: {:?}",
                    cmd_bin,
                    cmd_args_copy
                );
                match cmd.output().await {
                    Ok(out) => {
                        niupanel_common::logger::info!(
                            "命令执行完成: {} 状态: {:?}",
                            cmd_bin,
                            out.status
                        );
                        let mut res = String::new();
                        if !out.stdout.is_empty() {
                            res.push_str(&String::from_utf8_lossy(&out.stdout));
                        }
                        if !out.stderr.is_empty() {
                            if !res.is_empty() {
                                res.push_str("\n");
                            }
                            res.push_str("--- Error Output ---\n");
                            res.push_str(&String::from_utf8_lossy(&out.stderr));
                        }
                        if res.is_empty() {
                            res = "命令执行完成，但无任何输出。".to_string();
                        }
                        let _ = tx.send(res);
                    }
                    Err(e) => {
                        niupanel_common::logger::error!("系统任务指令执行失败: {}", e);
                        let _ = tx.send(format!("Error: {}", e));
                    }
                }
            })
            .await
        {
            Ok(job_id) => {
                // 记录任务 ID 与消息 ID 的映射，以便后续原地更新结果
                job_msgs_c.write().await.insert(job_id, (cid, mid));

                let msg = format!(
                    "✅ *运维任务已提交*\n\n*ID:* `{}`\n*任务:* `{}`\n\n正在后台执行...",
                    escape_tg_markdown(&job_id.to_string()),
                    escape_tg_markdown(&j_name)
                );
                let _ = bot_c
                    .edit_message_text(cid, mid, msg)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;
                publish_audit(&eb_c, cid.to_string(), format!("提交运维任务: {}", j_name));
            }
            Err(e) => {
                let _ = bot_c
                    .edit_message_text(cid, mid, format!("❌ 任务提交失败: {}", e))
                    .await;
            }
        }
    });

    true
}

// ──────────────────────────────── Helpers ────────────────────────────────

pub(crate) fn list_python_venvs() -> Vec<String> {
    let mut venvs = vec![];
    let path = std::path::PathBuf::from(&Config::global().runtimes_dir).join("python");
    if !path.exists() {
        return venvs;
    }
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("venv_") && entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
            {
                venvs.push(name);
            }
        }
    }
    venvs.sort();
    venvs
}

/// 通过 fnm list 获取已安装的 Node.js 版本列表（版本号, is_default）
pub(crate) fn list_node_versions_fnm() -> Vec<(String, bool)> {
    let fnm_bin = niupanel_core::script::interpreter::node::NodeEnvironment::get_fnm_bin();
    let output = std::process::Command::new(&fnm_bin).arg("list").output();
    let Ok(out) = output else {
        return vec![];
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut versions = vec![];
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line.contains("system") {
            continue;
        }
        let is_default = line.contains("default");
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(ver) = parts.iter().find(|&&p| p.starts_with('v')) {
            let v = ver.strip_prefix('v').unwrap_or(ver).to_string();
            versions.push((v, is_default));
        }
    }
    versions
}

pub async fn handle_npm(
    bot: &Bot,
    chat_id: ChatId,
    _tm: &TaskManagerService,
    _eb: &EventBus,
    args: &str,
) {
    let parts: Vec<&str> = args.trim().split_whitespace().collect();
    let sub = parts.first().cloned().unwrap_or("");
    let pkg = parts.get(1..).unwrap_or_default().join(" ");
    if !["install", "uninstall", "remove", "list"].contains(&sub) {
        let _ = bot.send_message(chat_id, "📦 *npm 用法*\n\n`/env npm install <包名>`\n`/env npm uninstall <包名>`\n`/env npm list`")
            .parse_mode(ParseMode::MarkdownV2).await;
        return;
    }

    // 全局单例模式：直接验证 fnm 可用，无需选择环境
    let versions = list_node_versions_fnm();
    if versions.is_empty() {
        let _ = bot
            .send_message(
                chat_id,
                "❌ 未找到 Node.js 版本，请先通过面板「环境管理」安装 Node.js。",
            )
            .await;
        return;
    }

    process_npm_action(bot, chat_id, sub, &pkg).await;
}

pub async fn handle_apt(
    bot: &Bot,
    chat_id: ChatId,
    _tm: &TaskManagerService,
    _eb: &EventBus,
    args: &str,
) {
    let parts: Vec<&str> = args.trim().split_whitespace().collect();
    let sub = parts.first().cloned().unwrap_or("");
    let pkg = parts.get(1..).unwrap_or_default().join(" ");
    if !["install", "remove", "uninstall"].contains(&sub) {
        let _ = bot
            .send_message(
                chat_id,
                "🐧 *apt 用法*\n\n`/env apt install <包名>`\n`/env apt remove <包名>`",
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await;
        return;
    }
    process_apt_action(bot, chat_id, sub, &pkg).await;
}

async fn process_npm_action(bot: &Bot, chat_id: ChatId, sub: &str, pkg: &str) {
    let scripts_dir = Config::global().scripts_dir.clone();
    let fnm_bin = niupanel_core::script::interpreter::node::NodeEnvironment::get_fnm_bin();

    if sub == "list" {
        match tokio::process::Command::new(&fnm_bin)
            .args(&["exec", "--using=default", "npm", "list", "--depth=0"])
            .current_dir(&scripts_dir)
            .output()
            .await
        {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let truncated = if stdout.len() > 3000 {
                    &stdout[..3000]
                } else {
                    &stdout
                };
                let _ = bot
                    .send_message(
                        chat_id,
                        format!(
                            "📦 *Node 全局包列表* \\(data/scripts/node\\_modules\\)\n```\n{}\n```",
                            escape_tg_markdown(truncated)
                        ),
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;
            }
            Err(e) => {
                let _ = bot
                    .send_message(chat_id, format!("❌ npm list 执行失败: {}", e))
                    .await;
            }
        }
        return;
    }

    if pkg.is_empty() {
        return;
    }
    let action_label = if sub == "uninstall" || sub == "remove" {
        "卸载"
    } else {
        "安装"
    };
    let kb = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("✅ 确认", format!("npm:{}:{}", sub, pkg.replace(' ', ","))),
        InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
    ]]);
    let text = format!(
        "📦 *确认 {} Node 包*\n\n*包:* `{}`\n*位置:* data/scripts/node\\_modules",
        action_label,
        escape_tg_markdown(pkg)
    );
    match bot
        .send_message(chat_id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(kb)
        .await
    {
        Ok(_) => niupanel_common::logger::info!("已发送 npm 确认消息至 {}", chat_id),
        Err(e) => niupanel_common::logger::error!("发送 npm 确认消息失败: {}", e),
    }
}

async fn process_apt_action(bot: &Bot, chat_id: ChatId, sub: &str, pkg: &str) {
    if pkg.is_empty() {
        return;
    }
    let kb = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("✅ 确认", format!("apt:{}:{}", sub, pkg.replace(' ', ","))),
        InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
    ]]);

    let text = format!(
        "⚠️ *确认 {} 系统包*\n\n*包:* `{}`",
        sub,
        escape_tg_markdown(pkg)
    );

    match bot
        .send_message(chat_id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(kb)
        .await
    {
        Ok(_) => niupanel_common::logger::info!("已发送 apt 确认消息至 {}", chat_id),
        Err(e) => niupanel_common::logger::error!("发送 apt 确认消息失败: {}", e),
    }
}
