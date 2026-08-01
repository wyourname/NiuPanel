pub mod auth;
pub mod config;
pub mod constants;
pub mod download;
pub mod error;
pub mod filesystem;
pub mod logger;
pub mod metrics;
pub mod models;
pub mod response;
pub mod tools;
#[cfg(feature = "axum")]
pub mod upload;
pub mod variable;
pub mod version;

pub use logger::{debug, error, info, trace, warn};

/// Telegram MarkdownV2 字符转义工具
/// 根据 Telegram Bot API 规范，对 MarkdownV2 模式下的特殊字符进行转义
pub fn escape_tg_markdown(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        match c {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|'
            | '{' | '}' | '.' | '!' => {
                escaped.push('\\');
                escaped.push(c);
            }
            _ => escaped.push(c),
        }
    }
    escaped
}

/// 通用的 Telegram 字符处理扩展
pub trait TelegramEscapable {
    fn escape_tg(&self) -> String;
}

impl TelegramEscapable for str {
    fn escape_tg(&self) -> String {
        escape_tg_markdown(self)
    }
}

impl TelegramEscapable for String {
    fn escape_tg(&self) -> String {
        escape_tg_markdown(self)
    }
}
