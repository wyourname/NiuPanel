use niupanel_entity::settings;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct SettingDto {
    pub key: String,
    pub value: String,
    pub category: String,
}

impl From<settings::Model> for SettingDto {
    fn from(model: settings::Model) -> Self {
        Self {
            key: model.key,
            value: model.value,
            category: model.category,
        }
    }
}

#[derive(Serialize, Debug, ToSchema)]
pub struct SessionDto {
    pub id: String,
    pub expiry: i64,
    pub is_current: bool,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct GeneralSettingsRequest {
    pub name: String,
    pub logo: String,
    pub timezone: String,
    pub max_concurrency: i32,
    pub log_retention_days: i32,
    pub github_proxy_url: String,
    pub uv_python_mirror: String,
    pub uv_pypi_mirror: String,
    pub default_python_version: String,
    pub default_node_version: String,
    pub fnm_node_dist_mirror: Option<String>,
    pub npm_registry_mirror: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct UpdateChannelRequest {
    pub channel: String,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct SecuritySettingsRequest {
    // System Settings
    pub max_sessions: i32,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct NotificationSettingsRequest {
    pub webhook_url: String,
    pub events: String,
    pub mail_host: String,
    pub mail_username: String,
    pub mail_password: String,
    pub mail_to: String,
}

#[derive(Deserialize, ToSchema)]
pub struct TestNotifyRequest {
    #[serde(alias = "notify.webhook_url")]
    pub webhook_url: Option<String>,
    #[serde(alias = "mail.host")]
    pub mail_host: Option<String>,
    #[serde(alias = "mail.username")]
    pub mail_username: Option<String>,
    #[serde(alias = "mail.password")]
    pub mail_password: Option<String>,
    #[serde(alias = "mail.to")]
    pub mail_to: Option<String>,
    pub notify_type: String,
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct CleanupLogsRequest {
    pub days: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, ToSchema)]
pub struct BackupOptions {
    #[serde(default)]
    pub tasks: bool, // 包含任务元数据及 scripts 目录
    #[serde(default)]
    pub variables: bool, // 系统及任务变量
    #[serde(default)]
    pub settings: bool, // 系统设置
    #[serde(default)]
    pub environments: bool, // 环境/依赖配置
    #[serde(default)]
    pub telegram: bool, // TG 机器人配置
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupManifest {
    pub version: String,
    pub created_at: i64,
    pub options: BackupOptions,
    pub files: Vec<String>, // 包含的文件列表
}
