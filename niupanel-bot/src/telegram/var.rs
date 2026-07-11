use niupanel_common::escape_tg_markdown;
use niupanel_entity::{tasks, variables};
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder, QuerySelect};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode};

use super::commands::{add_pagination, reply_or_edit_markup};
use super::conversation as conv;
use super::conversation::*;

pub async fn dispatch_var_cmd(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    convs: &ConversationStore,
    arg: &str,
) {
    let parts: Vec<&str> = arg.split_whitespace().collect();
    let sub = parts.first().cloned().unwrap_or("list");
    let sub_arg = parts.get(1..).unwrap_or_default().join(" ");

    match sub {
        "list" => {
            let page = sub_arg.parse().unwrap_or(0);
            handle_vars(bot, chat_id, db, page, None).await;
        }
        "add" => handle_add_var_start(bot, chat_id, convs).await,
        "edit" => handle_edit_var(bot, chat_id, db, convs, &sub_arg).await,
        "del" => handle_del_var(bot, chat_id, db, &sub_arg).await,
        _ => {
            let _ = bot
                .send_message(chat_id, "ℹ️ 用法: `/var <list|add|edit|del>`")
                .await;
        }
    }
}

pub async fn handle_vars(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    page: u64,
    mid: Option<MessageId>,
) {
    let size: u64 = 15;
    let list: Vec<variables::Model> = variables::Entity::find()
        .order_by_asc(variables::Column::Key)
        .order_by_asc(variables::Column::Id)
        .offset(Some(page * size))
        .limit(Some(size))
        .all(db)
        .await
        .unwrap_or_default();
    let total = variables::Entity::find().count(db).await.unwrap_or(0);
    if list.is_empty() && page == 0 {
        let _ = bot.send_message(chat_id, "😅 暂无变量\\.").await;
        return;
    }
    let mut kb = vec![];
    for chunk in list.chunks(3) {
        let mut row = vec![];
        for v in chunk {
            row.push(InlineKeyboardButton::callback(
                v.key.clone(),
                format!("var_manage_{}", v.id),
            ));
        }
        kb.push(row);
    }
    add_pagination(&mut kb, page, total, size, "vars_page");
    reply_or_edit_markup(
        bot,
        chat_id,
        mid,
        &format!("😅 *环境变量* \\(第{}页\\)", page + 1),
        InlineKeyboardMarkup::new(kb),
    )
    .await;
}

pub async fn handle_var_manage_menu(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    id: i32,
    mid: MessageId,
) {
    let v = match variables::Entity::find_by_id(id).one(db).await {
        Ok(Some(v)) => v,
        _ => return,
    };
    let text = format!(
        "⚙️ *管理变量*\n\n*Key:* `{}`\n*Value:* `{}`\n*作用域:* `{}`",
        escape_tg_markdown(&v.key),
        if v.value.len() > 20 {
            "***".to_string()
        } else {
            escape_tg_markdown(&v.value)
        },
        escape_tg_markdown(&v.scope)
    );
    let kb = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("📝 Key", format!("editvarfield_{}_key", v.id)),
            InlineKeyboardButton::callback("😅 Value", format!("editvarfield_{}_value", v.id)),
        ],
        vec![
            InlineKeyboardButton::callback("🌍 作用域", format!("editvarfield_{}_scope", v.id)),
            InlineKeyboardButton::callback("🗑️ 删除", format!("confirm_delvar_{}", v.id)),
        ],
        vec![InlineKeyboardButton::callback(
            "⬅️ 返回",
            "vars_page_0".to_string(),
        )],
    ]);
    let _ = bot
        .edit_message_text(chat_id, mid, text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(kb)
        .await;
}

pub async fn handle_add_var_start(bot: &Bot, chat_id: ChatId, convs: &ConversationStore) {
    if let Ok(m) = bot.send_message(chat_id, "😅 请输入变量名称 (Key)：").await {
        convs.write().await.insert(
            chat_id.to_string(),
            ConversationState::AddVar {
                step: AddVarStep::WaitingKey,
                data: AddVarData::default(),
                last_msg_id: Some(m.id),
                created_at: conv::now(),
            },
        );
    }
}

pub async fn handle_edit_var(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    convs: &ConversationStore,
    arg: &str,
) {
    let vid = arg.trim().parse::<i32>().unwrap_or(0);
    if let Ok(Some(v)) = variables::Entity::find_by_id(vid).one(db).await {
        let text = format!("✏️ 编辑变量: `{}`", escape_tg_markdown(&v.key));
        let kb = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("📝 Key", format!("editvarfield_{}_key", v.id)),
                InlineKeyboardButton::callback("😅 Value", format!("editvarfield_{}_value", v.id)),
            ],
            vec![
                InlineKeyboardButton::callback("🌍 作用域", format!("editvarfield_{}_scope", v.id)),
                InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
            ],
        ]);
        if let Ok(m) = bot
            .send_message(chat_id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(kb)
            .await
        {
            convs.write().await.insert(
                chat_id.to_string(),
                ConversationState::EditVar {
                    var_id: vid,
                    step: EditVarStep::SelectingField,
                    last_msg_id: Some(m.id),
                    selected_tasks: vec![],
                    created_at: conv::now(),
                },
            );
        }
    }
}

pub async fn handle_del_var(bot: &Bot, chat_id: ChatId, _db: &DatabaseConnection, arg: &str) {
    let vid = arg.trim().parse::<i32>().unwrap_or(0);
    let kb = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("🗑️ 确认删除", format!("confirm_delvar_{}", vid)),
        InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
    ]]);
    let _ = bot
        .send_message(chat_id, format!("⚠️ 确认删除变量 ID: `{}`?", vid))
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(kb)
        .await;
}

pub async fn render_variable_task_selector(
    bot: &Bot,
    chat_id: ChatId,
    db: &DatabaseConnection,
    msg_id: MessageId,
    selected: &[i32],
    prefix: &str,
) {
    let list = tasks::Entity::find()
        .order_by_asc(tasks::Column::Name)
        .all(db)
        .await
        .unwrap_or_default();
    let mut kb = vec![];
    for chunk in list.chunks(2) {
        let mut row = vec![];
        for t in chunk {
            let icon = if selected.contains(&t.id) {
                "✅"
            } else {
                "▫️"
            };
            row.push(InlineKeyboardButton::callback(
                format!("{} {}", icon, t.name),
                format!("{}_{}", prefix, t.id),
            ));
        }
        kb.push(row);
    }

    kb.push(vec![
        InlineKeyboardButton::callback("🏁 完成", "addvar_done_selection"),
        InlineKeyboardButton::callback("❌ 取消", "cancel_action"),
    ]);
    let _ = bot
        .edit_message_text(chat_id, msg_id, "📜 请选择关联任务：")
        .reply_markup(InlineKeyboardMarkup::new(kb))
        .await;
}
