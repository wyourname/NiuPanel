#[path = "startup/background.rs"]
mod background;
#[path = "startup/bootstrap.rs"]
pub(crate) mod bootstrap;
#[path = "startup/commands.rs"]
pub mod commands;
#[path = "startup/sdk.rs"]
mod sdk;
#[path = "startup/session.rs"]
mod session;

use crate::app;
use crate::common::state::AppState;
use anyhow::{Context, Result};
use bootstrap::{connect_database, init_session_key, run_migrations, set_global_config};
use niupanel_common::config::Config;
use niupanel_common::info;
use niupanel_core as core;
use niupanel_core::event_bus::EventBus;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tower_cookies::CookieManagerLayer;

pub async fn run() -> Result<Option<tracing_appender::non_blocking::WorkerGuard>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut config = Config::from_env().context("Failed to load configuration")?;
    initialize_sdk_paths(&mut config).await?;

    let guard = init_logger(&config);

    let db = connect_database(&config.database_url)
        .await
        .context("Database connection failed")?;
    run_migrations(&db).await.context("Migration failed")?;
    init_session_key(&db, &mut config)
        .await
        .context("Failed to initialize session key")?;
    set_global_config(config.clone());

    let http_client = build_http_client()?;
    let event_bus = EventBus::new();
    let settings_service = core::settings::SettingsService::new(db.clone(), event_bus.clone());
    let task_manager =
        core::task_manager::service::TaskManagerService::new(db.clone(), event_bus.clone())
            .await
            .context("Failed to start TaskManagerService")?;

    let sys = Arc::new(tokio::sync::Mutex::new(sysinfo::System::new_all()));
    let system_metrics = Arc::new(tokio::sync::RwLock::new(
        niupanel_common::metrics::SystemMetrics::default(),
    ));
    let update_service = build_update_service(db.clone(), http_client.clone());
    let public_ip = Arc::new(std::sync::RwLock::new(None));
    let api_key_cache = build_api_key_cache();
    let login_2fa_cache = build_login_2fa_cache();

    background::spawn_background_tasks(
        db.clone(),
        event_bus.clone(),
        http_client.clone(),
        task_manager.clone(),
        update_service.clone(),
        system_metrics.clone(),
    )
    .await?;
    background::spawn_system_shell_recovery(db.clone());
    background::spawn_telegram_import_listener(db.clone(), event_bus.clone());

    let state = AppState::new(
        db.clone(),
        task_manager,
        event_bus,
        settings_service,
        http_client,
        update_service,
        system_metrics.clone(),
        public_ip,
        api_key_cache,
        login_2fa_cache,
    );
    let _ = crate::common::state::GLOBAL_STATE.set(state.clone());

    background::spawn_system_metrics_updater(sys, system_metrics);

    let session_layer =
        session::build_session_layer(&db, &config.session_key, config.session_cookie_secure)
            .await?;
    let app = app::create_router(state)
        .layer(session_layer)
        .layer(CookieManagerLayer::new());

    serve(app, &config.server_addr).await?;
    Ok(guard)
}

async fn initialize_sdk_paths(config: &mut Config) -> Result<()> {
    let (python_path, node_path) = sdk::init_sdk().await.context("Failed to initialize SDK")?;
    config.extra_python_path = Some(python_path);
    config.extra_node_path = Some(node_path);
    Ok(())
}

fn init_logger(config: &Config) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    niupanel_common::logger::init(niupanel_common::logger::LogConfig {
        level: config.log_level.clone(),
        filters: config.log_filters.clone(),
        log_dir: Some(config.logs_dir.clone()),
        file_prefix: "niupanel".to_string(),
    })
}

fn build_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("NiuPanel/1.0.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")
}

fn build_update_service(
    db: sea_orm::DatabaseConnection,
    http_client: reqwest::Client,
) -> core::settings::update::UpdateService {
    let update_status = Arc::new(std::sync::RwLock::new(
        niupanel_common::models::update::UpdateStatus::default(),
    ));
    let update_cancellation_token = Arc::new(std::sync::Mutex::new(None));
    let app_version = env!("CARGO_PKG_VERSION").to_string();

    core::settings::update::UpdateService::new(
        db,
        http_client,
        app_version,
        update_status,
        update_cancellation_token,
    )
}

fn build_api_key_cache() -> moka::future::Cache<String, crate::common::state::CachedApiKey> {
    moka::future::Cache::builder()
        .max_capacity(1000)
        .time_to_live(std::time::Duration::from_secs(3600))
        .build()
}

fn build_login_2fa_cache() -> moka::future::Cache<String, (String, i32)> {
    moka::future::Cache::builder()
        .max_capacity(1000)
        .time_to_live(std::time::Duration::from_secs(300))
        .build()
}

async fn serve(app: axum::Router, server_addr: &str) -> Result<()> {
    let addr: SocketAddr = server_addr.parse().context("Invalid server address")?;
    let listener = TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;
    info!("NiuPanel listening on {}", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("Server error")?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .unwrap_or_else(|e| tracing::error!("Failed to install Ctrl+C handler: {}", e));
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            sig.recv().await;
        } else {
            tracing::error!("Failed to install SIGTERM handler");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Signal received, starting graceful shutdown");
}
