use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Json,
    extract::{Extension, State},
};
use niupanel_bot::telegram::{TelegramBotConfig, TelegramChatBinding};
use niupanel_common::auth::permissions::UserRole;
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::ApiResponse;
use niupanel_core::audit::service::AuditService;
use niupanel_core::event_bus::{SystemEvent, SystemNotification};
use niupanel_core::notification::service::NotificationService;
use niupanel_core::settings::SettingsManager;
use niupanel_entity::users;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use utoipa::ToSchema;

const TG_CONFIG_KEY: &str = "plugin.telegram.config";
const ALLOWED_EVENTS: [&str; 4] = ["success", "failed", "alert", "notification"];
const ALLOWED_INTERACTION_MODES: [&str; 2] = ["direct", "mention"];
const BINDING_SCHEMA_VERSION: u8 = 3;
const MAX_BINDINGS: usize = 32;
const MAX_LABEL_CHARS: usize = 64;

#[derive(Deserialize, Serialize, ToSchema, Clone, Default)]
#[serde(default)]
pub struct TelegramChatBindingSchema {
    pub chat_id: String,
    pub thread_id: Option<i32>,
    pub user_id: i32,
    pub label: String,
    pub events: Vec<String>,
    pub telegram_user_ids: Vec<String>,
    pub interaction_mode: String,
}

impl From<TelegramChatBindingSchema> for TelegramChatBinding {
    fn from(binding: TelegramChatBindingSchema) -> Self {
        Self {
            chat_id: binding.chat_id,
            thread_id: binding.thread_id,
            user_id: binding.user_id,
            label: binding.label,
            events: binding.events,
            telegram_user_ids: binding.telegram_user_ids,
            interaction_mode: binding.interaction_mode,
        }
    }
}

impl From<TelegramChatBinding> for TelegramChatBindingSchema {
    fn from(binding: TelegramChatBinding) -> Self {
        Self {
            chat_id: binding.chat_id,
            thread_id: binding.thread_id,
            user_id: binding.user_id,
            label: binding.label,
            events: binding.events,
            telegram_user_ids: binding.telegram_user_ids,
            interaction_mode: binding.interaction_mode,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema, Clone)]
#[serde(default)]
pub struct TelegramConfigSchema {
    pub enabled: bool,
    pub token: String,
    pub token_present: bool,
    pub clear_token: bool,
    pub chat_bindings: Vec<TelegramChatBindingSchema>,
    pub proxy_url: Option<String>,
    pub api_base_url: Option<String>,
    pub cf_proxy_enabled: bool,
    pub cf_host: String,
    pub cf_ip: String,
    pub cf_token: String,
    pub cf_token_present: bool,
    pub clear_cf_token: bool,
    pub agent_plugin_id: String,
}

impl Default for TelegramConfigSchema {
    fn default() -> Self {
        TelegramBotConfig::default().into()
    }
}

impl From<TelegramConfigSchema> for TelegramBotConfig {
    fn from(config: TelegramConfigSchema) -> Self {
        Self {
            enabled: config.enabled,
            token: config.token,
            binding_schema_version: BINDING_SCHEMA_VERSION,
            chat_bindings: config.chat_bindings.into_iter().map(Into::into).collect(),
            admin_chat_id: String::new(),
            proxy_url: config.proxy_url,
            api_base_url: config.api_base_url,
            events: Vec::new(),
            cf_proxy_enabled: config.cf_proxy_enabled,
            cf_host: config.cf_host,
            cf_ip: config.cf_ip,
            cf_token: config.cf_token,
            agent_plugin_id: config.agent_plugin_id,
        }
    }
}

impl From<TelegramBotConfig> for TelegramConfigSchema {
    fn from(config: TelegramBotConfig) -> Self {
        let token_present = !config.token.trim().is_empty();
        let cf_token_present = !config.cf_token.trim().is_empty();
        Self {
            enabled: config.enabled,
            token: String::new(),
            token_present,
            clear_token: false,
            chat_bindings: config.chat_bindings.into_iter().map(Into::into).collect(),
            proxy_url: config.proxy_url,
            api_base_url: config.api_base_url,
            cf_proxy_enabled: config.cf_proxy_enabled,
            cf_host: config.cf_host,
            cf_ip: config.cf_ip,
            cf_token: String::new(),
            cf_token_present,
            clear_cf_token: false,
            agent_plugin_id: config.agent_plugin_id,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct TelegramUserOption {
    pub id: i32,
    pub username: String,
    pub role: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/bot",
    responses((status = 200, description = "Get redacted Telegram Agent transport configuration")),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn get_telegram_config(
    State(state): State<AppState>,
) -> Result<ApiResponse<TelegramConfigSchema>> {
    let config = load_stored_telegram_config(&state.db).await?;
    Ok(ApiResponse::success(config.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/bot/users",
    responses((status = 200, description = "List minimal user identities for Telegram Chat bindings")),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn list_telegram_users(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<ApiResponse<Vec<TelegramUserOption>>> {
    require_admin(&user)?;
    let users = users::Entity::find()
        .order_by_asc(users::Column::Username)
        .all(&state.db)
        .await?
        .into_iter()
        .map(|user| TelegramUserOption {
            id: user.id,
            username: user.username,
            role: user.role,
        })
        .collect();
    Ok(ApiResponse::success(users))
}

#[utoipa::path(
    put,
    path = "/api/v1/bot",
    request_body = TelegramConfigSchema,
    responses((status = 200, description = "Update Telegram Agent transport configuration")),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn update_telegram_config(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(schema): Json<TelegramConfigSchema>,
) -> Result<ApiResponse<()>> {
    require_admin(&user)?;
    let stored = load_stored_telegram_config(&state.db).await?;
    let mut config = merge_secrets(schema, &stored);
    normalize_config(&mut config);
    validate_config(&state.db, &config).await?;

    let config_json = serde_json::to_string(&config)
        .map_err(|error| AppError::Serialization(error.to_string()))?;
    SettingsManager::set(&state.db, TG_CONFIG_KEY, &config_json, Some("Notification")).await?;
    state
        .event_bus
        .publish(SystemEvent::System(SystemNotification::SettingChanged {
            key: TG_CONFIG_KEY.to_string(),
            value: config_json,
        }));
    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "更新 Telegram Agent 通道",
        "telegram_agent",
        None,
        Some("更新了 Telegram 传输配置".to_string()),
        Some(ip),
    )
    .await;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/bot/test",
    request_body = TelegramConfigSchema,
    responses((status = 200, description = "Test Telegram Agent transport for every configured binding")),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn test_telegram(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(schema): Json<TelegramConfigSchema>,
) -> Result<ApiResponse<()>> {
    require_admin(&user)?;
    let stored = load_stored_telegram_config(&state.db).await?;
    let mut config = merge_secrets(schema, &stored);
    normalize_config(&mut config);
    validate_config(&state.db, &config).await?;
    if config.token.is_empty() || config.chat_bindings.is_empty() {
        return Err(AppError::ValidationError(
            "测试 Telegram 通道时需要 Bot Token 和至少一个 Chat 绑定".to_string(),
        ));
    }

    let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(15));
    let mut proxy_shutdown = None;

    if config.cf_proxy_enabled && !config.cf_host.is_empty() {
        let proxy_config = niupanel_proxy::Config {
            cfhost: config.cf_host.clone(),
            cfip: config.cf_ip.clone(),
            token: config.cf_token.clone(),
            host: "127.0.0.1".to_string(),
            port: 0,
            user: String::new(),
            passwd: String::new(),
            log: std::path::PathBuf::new(),
            loglevel: "info".to_string(),
        };
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        proxy_shutdown = Some(shutdown_tx);
        tokio::spawn(async move {
            let _ = niupanel_proxy::start_server(
                Arc::new(proxy_config),
                shutdown_rx,
                ready_tx,
                "AgentBotTest",
            )
            .await;
        });
        let actual_addr = ready_rx
            .await
            .map_err(|_| AppError::Generic("无法启动 Telegram 测试代理".to_string()))?;
        builder = builder.proxy(
            reqwest::Proxy::all(format!("socks5h://{actual_addr}")).map_err(AppError::Reqwest)?,
        );
    } else if let Some(proxy) = config
        .proxy_url
        .as_deref()
        .filter(|proxy| !proxy.is_empty())
    {
        builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(AppError::Reqwest)?);
    }

    let client = builder.build().map_err(AppError::Reqwest)?;
    let text =
        niupanel_common::escape_tg_markdown("NiuPanel Ops Agent Telegram 通道连接测试成功。");
    let mut result = Ok(());
    for binding in &config.chat_bindings {
        if let Err(error) = NotificationService::send_telegram(
            &client,
            &config.token,
            &binding.chat_id,
            &text,
            config.api_base_url.as_deref(),
            binding.thread_id,
        )
        .await
        {
            result = Err(error);
            break;
        }
    }
    if let Some(shutdown) = proxy_shutdown {
        let _ = shutdown.send(());
    }
    result?;
    Ok(ApiResponse::success(()))
}

pub(crate) async fn load_stored_telegram_config(
    db: &DatabaseConnection,
) -> Result<TelegramBotConfig> {
    let raw = SettingsManager::get(db, TG_CONFIG_KEY).await?;
    if raw.trim().is_empty() {
        return Ok(TelegramBotConfig::default());
    }
    let mut config = serde_json::from_str::<TelegramBotConfig>(&raw)
        .map_err(|error| AppError::Serialization(error.to_string()))?;
    if normalize_legacy_bindings(db, &mut config).await? {
        let normalized = serde_json::to_string(&config)
            .map_err(|error| AppError::Serialization(error.to_string()))?;
        SettingsManager::set(db, TG_CONFIG_KEY, &normalized, Some("Notification")).await?;
    }
    Ok(config)
}

async fn normalize_legacy_bindings(
    db: &DatabaseConnection,
    config: &mut TelegramBotConfig,
) -> Result<bool> {
    if !config.chat_bindings.is_empty() {
        let mut changed = !config.admin_chat_id.is_empty() || !config.events.is_empty();
        if config.binding_schema_version < BINDING_SCHEMA_VERSION {
            for binding in &mut config.chat_bindings {
                if binding.interaction_mode.trim().is_empty() {
                    binding.interaction_mode = "direct".to_string();
                }
            }
            config.binding_schema_version = BINDING_SCHEMA_VERSION;
            changed = true;
        }
        config.admin_chat_id.clear();
        config.events.clear();
        return Ok(changed);
    }
    if config.admin_chat_id.trim().is_empty() {
        if config.binding_schema_version < BINDING_SCHEMA_VERSION {
            config.binding_schema_version = BINDING_SCHEMA_VERSION;
            return Ok(true);
        }
        return Ok(false);
    }

    let admin = users::Entity::find()
        .filter(users::Column::Role.eq("admin"))
        .order_by_asc(users::Column::Id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::ValidationError("旧 Telegram 配置无法迁移：未找到管理员用户".to_string())
        })?;
    migrate_legacy_bindings(config, admin.id, &admin.username);
    Ok(true)
}

fn migrate_legacy_bindings(config: &mut TelegramBotConfig, user_id: i32, username: &str) {
    let events = config.events.clone();
    let mut seen = HashSet::new();
    config.chat_bindings = config
        .admin_chat_id
        .split(',')
        .map(str::trim)
        .filter(|chat_id| !chat_id.is_empty() && seen.insert((*chat_id).to_string()))
        .map(|chat_id| TelegramChatBinding {
            chat_id: chat_id.to_string(),
            thread_id: None,
            user_id,
            label: format!("{}（旧配置）", username),
            events: events.clone(),
            telegram_user_ids: Vec::new(),
            interaction_mode: if chat_id.starts_with('-') {
                "mention".to_string()
            } else {
                "direct".to_string()
            },
        })
        .collect();
    config.binding_schema_version = BINDING_SCHEMA_VERSION;
    config.admin_chat_id.clear();
    config.events.clear();
}

fn merge_secrets(schema: TelegramConfigSchema, stored: &TelegramBotConfig) -> TelegramBotConfig {
    let clear_token = schema.clear_token;
    let clear_cf_token = schema.clear_cf_token;
    let submitted_token = schema.token.trim().to_string();
    let submitted_cf_token = schema.cf_token.trim().to_string();
    let mut config: TelegramBotConfig = schema.into();
    config.token = if clear_token {
        String::new()
    } else if submitted_token.is_empty() {
        stored.token.clone()
    } else {
        submitted_token
    };
    config.cf_token = if clear_cf_token {
        String::new()
    } else if submitted_cf_token.is_empty() {
        stored.cf_token.clone()
    } else {
        submitted_cf_token
    };
    config
}

fn normalize_config(config: &mut TelegramBotConfig) {
    config.binding_schema_version = BINDING_SCHEMA_VERSION;
    config.token = config.token.trim().to_string();
    config.proxy_url = config
        .proxy_url
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    config.api_base_url = config
        .api_base_url
        .take()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty());
    config.cf_host = config.cf_host.trim().to_string();
    config.cf_ip = config.cf_ip.trim().to_string();
    config.cf_token = config.cf_token.trim().to_string();
    config.agent_plugin_id = config.agent_plugin_id.trim().to_string();
    config.admin_chat_id.clear();
    config.events.clear();
    for binding in &mut config.chat_bindings {
        binding.chat_id = binding.chat_id.trim().to_string();
        if let Ok(chat_id) = binding.chat_id.parse::<i64>() {
            binding.chat_id = chat_id.to_string();
        }
        binding.label = binding.label.trim().to_string();
        binding.interaction_mode = binding.interaction_mode.trim().to_ascii_lowercase();
        if binding.interaction_mode.is_empty() {
            binding.interaction_mode = "direct".to_string();
        }
        binding.telegram_user_ids = binding
            .telegram_user_ids
            .iter()
            .map(|user_id| {
                let user_id = user_id.trim();
                user_id
                    .parse::<u64>()
                    .map(|user_id| user_id.to_string())
                    .unwrap_or_else(|_| user_id.to_string())
            })
            .filter(|user_id| !user_id.is_empty())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        binding.telegram_user_ids.sort();
        binding.events = binding
            .events
            .iter()
            .map(|event| event.trim().to_string())
            .filter(|event| !event.is_empty())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        binding.events.sort();
    }
}

pub(crate) async fn validate_config(
    db: &DatabaseConnection,
    config: &TelegramBotConfig,
) -> Result<()> {
    validate_config_shape(config)?;
    let user_ids = config
        .chat_bindings
        .iter()
        .map(|binding| binding.user_id)
        .collect::<HashSet<_>>();
    if user_ids.is_empty() {
        return Ok(());
    }
    let existing_ids = users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|user| user.id)
        .collect::<HashSet<_>>();
    if let Some(missing) = user_ids.difference(&existing_ids).next() {
        return Err(AppError::ValidationError(format!(
            "Telegram Chat 绑定的面板用户 {} 不存在",
            missing
        )));
    }
    Ok(())
}

fn validate_config_shape(config: &TelegramBotConfig) -> Result<()> {
    if config.enabled && config.token.is_empty() {
        return Err(AppError::ValidationError(
            "启用 Telegram 通道时 Bot Token 不能为空".to_string(),
        ));
    }
    if config.enabled && config.chat_bindings.is_empty() {
        return Err(AppError::ValidationError(
            "启用 Telegram 通道时至少需要一个 Chat 绑定".to_string(),
        ));
    }
    if config.chat_bindings.len() > MAX_BINDINGS {
        return Err(AppError::ValidationError(format!(
            "Telegram Chat 绑定不能超过 {} 个",
            MAX_BINDINGS
        )));
    }
    if config.agent_plugin_id.is_empty() {
        return Err(AppError::ValidationError(
            "Agent plugin ID 不能为空".to_string(),
        ));
    }
    let mut targets = HashSet::new();
    for binding in &config.chat_bindings {
        let chat_id = binding
            .chat_id
            .parse::<i64>()
            .ok()
            .filter(|chat_id| *chat_id != 0)
            .ok_or_else(|| {
                AppError::ValidationError(format!(
                    "Telegram Chat ID '{}' 必须是非零整数",
                    binding.chat_id
                ))
            })?;
        if binding.thread_id.is_some_and(|thread_id| thread_id <= 0) {
            return Err(AppError::ValidationError(format!(
                "Telegram Chat '{}' 的 Topic ID 必须是正整数",
                binding.chat_id
            )));
        }
        if chat_id > 0 && binding.thread_id.is_some() {
            return Err(AppError::ValidationError(
                "Telegram 私聊绑定不能配置 Topic ID".to_string(),
            ));
        }
        if !targets.insert((binding.chat_id.clone(), binding.thread_id)) {
            return Err(AppError::ValidationError(format!(
                "Telegram Chat '{}' 的 Topic {:?} 重复绑定",
                binding.chat_id, binding.thread_id
            )));
        }
        if binding.user_id <= 0 {
            return Err(AppError::ValidationError(
                "Telegram Chat 绑定必须选择有效的面板用户".to_string(),
            ));
        }
        if binding.label.chars().count() > MAX_LABEL_CHARS {
            return Err(AppError::ValidationError(format!(
                "Telegram Chat 绑定名称不能超过 {} 个字符",
                MAX_LABEL_CHARS
            )));
        }
        if !ALLOWED_INTERACTION_MODES.contains(&binding.interaction_mode.as_str()) {
            return Err(AppError::ValidationError(format!(
                "不支持 Telegram 交互模式 '{}'",
                binding.interaction_mode
            )));
        }
        if chat_id < 0 && binding.telegram_user_ids.is_empty() {
            return Err(AppError::ValidationError(format!(
                "Telegram 群组 '{}' 必须配置允许操作的 Telegram User ID",
                binding.chat_id
            )));
        }
        let mut telegram_user_ids = HashSet::new();
        for telegram_user_id in &binding.telegram_user_ids {
            let parsed = telegram_user_id.parse::<u64>().ok().filter(|id| *id > 0);
            let Some(parsed) = parsed else {
                return Err(AppError::ValidationError(format!(
                    "Telegram User ID '{}' 必须是正整数",
                    telegram_user_id
                )));
            };
            if !telegram_user_ids.insert(parsed) {
                return Err(AppError::ValidationError(format!(
                    "Telegram User ID '{}' 重复",
                    telegram_user_id
                )));
            }
            if chat_id > 0 && parsed != chat_id as u64 {
                return Err(AppError::ValidationError(
                    "私聊绑定的 Telegram User ID 必须与 Chat ID 相同，或留空自动匹配".to_string(),
                ));
            }
        }
        if let Some(event) = binding
            .events
            .iter()
            .find(|event| !ALLOWED_EVENTS.contains(&event.as_str()))
        {
            return Err(AppError::ValidationError(format!(
                "不支持 Telegram 通知事件 '{}'",
                event
            )));
        }
    }
    Ok(())
}

fn require_admin(user: &AuthenticatedUser) -> Result<()> {
    if user.role != UserRole::Admin {
        return Err(AppError::Forbidden(
            "只有管理员可以修改或测试 Telegram 通道".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> TelegramBotConfig {
        TelegramBotConfig {
            enabled: true,
            token: "secret".to_string(),
            chat_bindings: vec![TelegramChatBinding {
                chat_id: "-100123".to_string(),
                thread_id: Some(88),
                user_id: 7,
                label: "on-call".to_string(),
                events: vec!["failed".to_string()],
                telegram_user_ids: vec!["7001".to_string()],
                interaction_mode: "mention".to_string(),
            }],
            ..TelegramBotConfig::default()
        }
    }

    #[test]
    fn public_schema_redacts_secrets() {
        let mut config = valid_config();
        config.cf_token = "cf-secret".to_string();
        let schema: TelegramConfigSchema = config.into();
        assert!(schema.token.is_empty());
        assert!(schema.cf_token.is_empty());
        assert!(schema.token_present);
        assert!(schema.cf_token_present);
    }

    #[test]
    fn empty_secret_updates_preserve_stored_values_until_explicitly_cleared() {
        let mut stored = valid_config();
        stored.cf_token = "cf-secret".to_string();

        let preserved = merge_secrets(stored.clone().into(), &stored);
        assert_eq!(preserved.token, "secret");
        assert_eq!(preserved.cf_token, "cf-secret");

        let mut clear: TelegramConfigSchema = stored.clone().into();
        clear.clear_token = true;
        clear.clear_cf_token = true;
        let cleared = merge_secrets(clear, &stored);
        assert!(cleared.token.is_empty());
        assert!(cleared.cf_token.is_empty());
    }

    #[test]
    fn normalized_storage_omits_legacy_fields() {
        let stored = serde_json::to_value(valid_config()).expect("serialize config");
        assert!(stored.get("admin_chat_id").is_none());
        assert!(stored.get("events").is_none());
        assert!(stored.get("login_2fa").is_none());
        assert!(stored.get("chat_bindings").is_some());
    }

    #[test]
    fn enabled_transport_requires_credentials_and_binding() {
        let mut config = TelegramBotConfig::default();
        config.enabled = true;
        assert!(validate_config_shape(&config).is_err());
        config.token = "token".to_string();
        assert!(validate_config_shape(&config).is_err());
        config.chat_bindings = valid_config().chat_bindings;
        assert!(validate_config_shape(&config).is_ok());
    }

    #[test]
    fn duplicate_and_unknown_binding_values_are_rejected() {
        let mut config = valid_config();
        config.chat_bindings.push(config.chat_bindings[0].clone());
        assert!(validate_config_shape(&config).is_err());
        config.chat_bindings.pop();
        config.chat_bindings[0].events = vec!["credentials".to_string()];
        assert!(validate_config_shape(&config).is_err());
    }

    #[test]
    fn group_bindings_require_senders() {
        let mut config = valid_config();
        config.chat_bindings[0].telegram_user_ids.clear();
        assert!(validate_config_shape(&config).is_err());
    }

    #[test]
    fn different_topics_in_the_same_group_are_distinct_targets() {
        let mut config = valid_config();
        let mut second = config.chat_bindings[0].clone();
        second.thread_id = Some(99);
        config.chat_bindings.push(second);
        assert!(validate_config_shape(&config).is_ok());
    }

    #[test]
    fn legacy_chat_ids_are_migrated_once() {
        let mut config = TelegramBotConfig {
            admin_chat_id: " 123,456,123 ".to_string(),
            events: vec!["failed".to_string()],
            ..TelegramBotConfig::default()
        };
        migrate_legacy_bindings(&mut config, 9, "admin");
        assert_eq!(config.chat_bindings.len(), 2);
        assert!(
            config
                .chat_bindings
                .iter()
                .all(|binding| binding.user_id == 9)
        );
        assert_eq!(config.binding_schema_version, BINDING_SCHEMA_VERSION);
        assert!(config.admin_chat_id.is_empty());
        assert!(config.events.is_empty());
    }

    #[test]
    fn legacy_login_2fa_fields_are_ignored_and_not_serialized() {
        let config: TelegramBotConfig = serde_json::from_value(serde_json::json!({
            "binding_schema_version": 2,
            "login_2fa": true,
            "chat_bindings": [{
                "chat_id": "123",
                "user_id": 9,
                "login_2fa": true
            }]
        }))
        .expect("parse legacy Telegram config");
        let stored = serde_json::to_value(config).expect("serialize Telegram config");
        assert!(stored.get("login_2fa").is_none());
        assert!(stored["chat_bindings"][0].get("login_2fa").is_none());
    }
}
