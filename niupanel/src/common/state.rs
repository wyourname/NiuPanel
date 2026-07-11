use niupanel_common::auth::permissions::Permission;
use niupanel_core::event_bus::EventBus;
use niupanel_core::settings::SettingsService;
use niupanel_core::settings::update::UpdateService;
use niupanel_core::task_manager::service::TaskManagerService;
use sea_orm::DatabaseConnection;
use std::collections::HashSet;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Instant;

use niupanel_common::metrics::SystemMetrics;

#[derive(Clone)]
pub struct CachedApiKey {
    pub key_id: i32,
    pub user_id: i32,
    pub permissions: HashSet<Permission>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub static GLOBAL_STATE: OnceLock<AppState> = OnceLock::new();

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

pub struct AppStateInner {
    pub db: DatabaseConnection,
    pub task_manager: TaskManagerService,
    pub event_bus: EventBus,
    pub settings: SettingsService,
    pub http_client: reqwest::Client,
    pub update_service: UpdateService,
    pub system_metrics: Arc<tokio::sync::RwLock<SystemMetrics>>,
    pub public_ip: Arc<RwLock<Option<(String, Instant)>>>,
    pub api_key_cache: moka::future::Cache<String, CachedApiKey>,
    pub login_2fa_cache: moka::future::Cache<String, (String, i32)>, // ticket -> (code, user_id)
}

impl AppState {
    pub fn new(
        db: DatabaseConnection,
        task_manager: TaskManagerService,
        event_bus: EventBus,
        settings: SettingsService,
        http_client: reqwest::Client,
        update_service: UpdateService,
        system_metrics: Arc<tokio::sync::RwLock<SystemMetrics>>,
        public_ip: Arc<RwLock<Option<(String, Instant)>>>,
        api_key_cache: moka::future::Cache<String, CachedApiKey>,
        login_2fa_cache: moka::future::Cache<String, (String, i32)>,
    ) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                db,
                task_manager,
                event_bus,
                settings,
                http_client,
                update_service,
                system_metrics,
                public_ip,
                api_key_cache,
                login_2fa_cache,
            }),
        }
    }
}

// 通过 Deref 使得 state.db 依然能直接访问，不需要 state.inner.db
impl std::ops::Deref for AppState {
    type Target = AppStateInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
