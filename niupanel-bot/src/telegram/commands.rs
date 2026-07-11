use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode};

pub use super::system::*;
pub use super::task::*;
pub use super::var::*;

pub(crate) async fn reply_or_edit(bot: &Bot, chat_id: ChatId, mid: Option<MessageId>, text: &str) {
    if let Some(id) = mid {
        let _ = bot
            .edit_message_text(chat_id, id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .await;
    } else {
        let _ = bot
            .send_message(chat_id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .await;
    }
}

pub(crate) async fn reply_or_edit_markup(
    bot: &Bot,
    chat_id: ChatId,
    mid: Option<MessageId>,
    text: &str,
    kb: InlineKeyboardMarkup,
) {
    if let Some(id) = mid {
        let _ = bot
            .edit_message_text(chat_id, id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(kb)
            .await;
    } else {
        let _ = bot
            .send_message(chat_id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(kb)
            .await;
    }
}

pub(crate) fn add_pagination(
    kb: &mut Vec<Vec<InlineKeyboardButton>>,
    page: u64,
    total: u64,
    size: u64,
    prefix: &str,
) {
    let pages = (total + size - 1) / size;
    let mut row = vec![];
    if page > 0 {
        row.push(InlineKeyboardButton::callback(
            "⬅️",
            format!("{}_{}", prefix, page - 1),
        ));
    }
    if page + 1 < pages {
        row.push(InlineKeyboardButton::callback(
            "➡️",
            format!("{}_{}", prefix, page + 1),
        ));
    }
    if !row.is_empty() {
        kb.push(row);
    }
}
