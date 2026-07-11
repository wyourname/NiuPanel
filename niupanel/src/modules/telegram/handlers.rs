use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::extract::Multipart;
use axum::{
    Json,
    extract::{Extension, State},
};
use niupanel_bot::telegram::TelegramBotConfig;
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::ApiResponse;
use niupanel_core::audit::service::AuditService;
use niupanel_core::event_bus::{SystemEvent, TelegramEvent};
use niupanel_core::notification::service::NotificationService;
use niupanel_core::settings::SettingsManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

const TG_CONFIG_KEY: &str = "plugin.telegram.config";

#[derive(Deserialize, ToSchema)]
pub struct SendMessageRequest {
    pub chat_id: String,
    pub text: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SendServerFileRequest {
    pub path: String,
    pub chat_id: String,
}

#[derive(Deserialize, Serialize, ToSchema, Default, Clone)]
#[serde(default)]
pub struct TelegramConfigSchema {
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

impl From<TelegramConfigSchema> for TelegramBotConfig {
    fn from(s: TelegramConfigSchema) -> Self {
        Self {
            enabled: s.enabled,
            token: s.token,
            admin_chat_id: s.admin_chat_id,
            proxy_url: s.proxy_url,
            api_base_url: s.api_base_url,
            events: s.events,
            cf_proxy_enabled: s.cf_proxy_enabled,
            cf_host: s.cf_host,
            cf_ip: s.cf_ip,
            cf_token: s.cf_token,
            login_2fa: s.login_2fa,
        }
    }
}

impl From<TelegramBotConfig> for TelegramConfigSchema {
    fn from(c: TelegramBotConfig) -> Self {
        Self {
            enabled: c.enabled,
            token: c.token,
            admin_chat_id: c.admin_chat_id,
            proxy_url: c.proxy_url,
            api_base_url: c.api_base_url,
            events: c.events,
            cf_proxy_enabled: c.cf_proxy_enabled,
            cf_host: c.cf_host,
            cf_ip: c.cf_ip,
            cf_token: c.cf_token,
            login_2fa: c.login_2fa,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/bot/telegram/config",
    responses(
        (status = 200, description = "Get Telegram bot configuration")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn get_telegram_config(
    State(state): State<AppState>,
) -> Result<ApiResponse<TelegramConfigSchema>> {
    let config_str = SettingsManager::get(&state.db, TG_CONFIG_KEY)
        .await
        .unwrap_or_default();
    let config: TelegramBotConfig = if config_str.is_empty() {
        TelegramBotConfig::default()
    } else {
        serde_json::from_str(&config_str).unwrap_or_default()
    };
    Ok(ApiResponse::success(config.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/bot/telegram/config",
    request_body = TelegramConfigSchema,
    responses(
        (status = 200, description = "Update Telegram bot configuration")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn update_telegram_config(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(schema): Json<TelegramConfigSchema>,
) -> Result<ApiResponse<()>> {
    let config: TelegramBotConfig = schema.into();
    let config_str = serde_json::to_string(&config)
        .map_err(|e| niupanel_common::error::AppError::Serialization(e.to_string()))?;
    SettingsManager::set(&state.db, TG_CONFIG_KEY, &config_str, Some("Notification")).await?;

    state.event_bus.publish(SystemEvent::System(
        niupanel_core::event_bus::SystemNotification::SettingChanged {
            key: TG_CONFIG_KEY.to_string(),
            value: config_str,
        },
    ));

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "更新Telegram机器人设置",
        "telegram",
        None,
        Some("更新了Telegram机器人配置".to_string()),
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/bot/telegram/latency",
    responses(
        (status = 200, description = "Get Telegram API latency")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn get_latency(State(state): State<AppState>) -> Result<ApiResponse<u64>> {
    let config_str = SettingsManager::get(&state.db, TG_CONFIG_KEY)
        .await
        .unwrap_or_default();
    let config: TelegramBotConfig = serde_json::from_str(&config_str).unwrap_or_default();

    if !config.enabled || config.token.is_empty() {
        return Ok(ApiResponse::success(0));
    }

    let start = std::time::Instant::now();
    let client = reqwest::Client::new();
    let url = format!("https://api.telegram.org/bot{}/getMe", config.token);

    let _ = client.get(url).send().await;
    let latency = start.elapsed().as_millis() as u64;

    Ok(ApiResponse::success(latency))
}

#[utoipa::path(
    get,
    path = "/api/v1/bot/telegram/metrics",
    responses(
        (status = 200, description = "Get Telegram bot metrics")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn get_bot_metrics(
    State(state): State<AppState>,
) -> Result<ApiResponse<niupanel_common::metrics::SystemMetrics>> {
    let metrics = state.system_metrics.read().await.clone();
    Ok(ApiResponse::success(metrics))
}

#[utoipa::path(
    post,
    path = "/api/v1/bot/telegram/send",
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Send Telegram message")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn send_message(
    State(state): State<AppState>,
    Json(req): Json<SendMessageRequest>,
) -> Result<ApiResponse<()>> {
    state
        .event_bus
        .publish(SystemEvent::Telegram(TelegramEvent::MessageSent {
            chat_id: req.chat_id,
            text: Some(req.text),
            file_name: None,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }));
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/bot/telegram/send-file",
    request_body = SendServerFileRequest,
    responses(
        (status = 200, description = "Send server file via Telegram")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn send_server_file(
    State(state): State<AppState>,
    Json(req): Json<SendServerFileRequest>,
) -> Result<ApiResponse<()>> {
    let full_path = std::path::Path::new("data/scripts").join(req.path.trim_start_matches('/'));

    if !full_path.exists() {
        return Err(AppError::Generic("文件不存在".into()));
    }

    state
        .event_bus
        .publish(SystemEvent::Telegram(TelegramEvent::MessageSent {
            chat_id: req.chat_id,
            text: None,
            file_name: Some(full_path.to_string_lossy().to_string()),
            timestamp: chrono::Utc::now().timestamp() as u64,
        }));
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/bot/telegram/upload-send",
    responses(
        (status = 200, description = "Upload and send file via Telegram")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn send_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<ApiResponse<()>> {
    let mut chat_id = String::new();
    let mut file_path = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Generic(e.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "chat_id" {
            chat_id = field
                .text()
                .await
                .map_err(|e| AppError::Generic(e.to_string()))?;
        } else if name == "file" {
            let filename = field.file_name().unwrap_or("upload").to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::Generic(e.to_string()))?;

            let temp_dir = std::env::temp_dir();
            let path = temp_dir.join(&filename);
            tokio::fs::write(&path, data).await.map_err(AppError::Io)?;
            file_path = path.to_string_lossy().to_string();
        }
    }

    if !chat_id.is_empty() && !file_path.is_empty() {
        state
            .event_bus
            .publish(SystemEvent::Telegram(TelegramEvent::MessageSent {
                chat_id,
                text: None,
                file_name: Some(file_path),
                timestamp: chrono::Utc::now().timestamp() as u64,
            }));
    }

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/bot/telegram/test",
    request_body = TelegramConfigSchema,
    responses(
        (status = 200, description = "Test Telegram bot configuration")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn test_telegram(
    State(_state): State<AppState>,
    Json(schema): Json<TelegramConfigSchema>,
) -> Result<ApiResponse<()>> {
    let config: TelegramBotConfig = schema.into();
    let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(15));
    let mut _proxy_shutdown = None;

    if config.cf_proxy_enabled && !config.cf_host.is_empty() {
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

        let (tx, rx) = tokio::sync::oneshot::channel();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        _proxy_shutdown = Some(tx);

        tokio::spawn(async move {
            let _ =
                niupanel_proxy::start_server(Arc::new(proxy_config), rx, ready_tx, "Test").await;
        });

        if let Ok(actual_addr) = ready_rx.await {
            builder =
                builder.proxy(reqwest::Proxy::all(format!("socks5h://{}", actual_addr)).unwrap());
        } else {
            return Err(niupanel_common::error::AppError::Generic(
                "无法启动临时测试代理".to_string(),
            ));
        }
    } else if let Some(proxy) = &config.proxy_url {
        if !proxy.is_empty() {
            if let Ok(p) = reqwest::Proxy::all(proxy) {
                builder = builder.proxy(p);
            }
        }
    }

    let client = builder
        .build()
        .map_err(|e| niupanel_common::error::AppError::Generic(e.to_string()))?;

    let res = NotificationService::send_telegram(
        &client,
        &config.token,
        &config.admin_chat_id,
        &niupanel_common::escape_tg_markdown(
            "🔔 *NiuPanel Telegram Test*\nThis is a test notification from the bot config panel.",
        ),
        config.api_base_url.as_deref(),
    )
    .await;

    if let Some(tx) = _proxy_shutdown {
        let _ = tx.send(());
    }

    res?;

    Ok(ApiResponse::success(()))
}
