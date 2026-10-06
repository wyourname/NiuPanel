use anyhow::Result;
use niupanel_common::constants::settings::{
    NPM_REGISTRY_MIRROR, PNPM_NODE_DIST_MIRROR, UV_PYPI_MIRROR, UV_PYTHON_MIRROR,
};
use niupanel_common::{error, info, warn};
use niupanel_core::event_bus::{EventBus, SystemEvent, SystemNotification};
use niupanel_core::handlers::{db_sync::DbSyncHandler, notification::NotificationHandler};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement,
};
use std::path::Path;
use std::sync::Arc;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};

#[path = "background/network_metrics.rs"]
mod network_metrics;
#[path = "background/telegram_agent.rs"]
mod telegram_agent;
#[path = "background/telegram_logs.rs"]
mod telegram_logs;
#[path = "background/telegram_plugin.rs"]
mod telegram_plugin;

use network_metrics::NetworkSampler;

pub type SharedSystemMetrics = Arc<tokio::sync::RwLock<niupanel_common::metrics::SystemMetrics>>;

pub async fn spawn_background_tasks(state: crate::common::state::AppState) -> Result<()> {
    let db = state.db.clone();
    let event_bus = state.event_bus.clone();
    spawn_notification_handler(db.clone(), event_bus.clone(), state.http_client.clone());
    spawn_db_sync_handler(db.clone(), event_bus.clone());

    crate::modules::settings::service::start_log_cleanup_scheduler(db.clone()).await;
    crate::modules::git_sync::service::GitService::start_auto_sync_scheduler(db.clone()).await;
    start_login_attempts_cleanup_scheduler(db.clone()).await;
    start_api_keys_cleanup_scheduler(db.clone()).await;
    spawn_upgrade_self_check(db.clone(), event_bus.clone());
    start_sandbox_environments_recovery(db.clone()).await;
    telegram_plugin::spawn(state);

    Ok(())
}

pub fn spawn_system_shell_recovery(db: DatabaseConnection) {
    tokio::spawn(async move {
        if let Ok(Some(env_model)) = niupanel_entity::environments::Entity::find()
            .filter(niupanel_entity::environments::Column::Name.eq("System"))
            .one(&db)
            .await
        {
            let Some(reqs) = env_model.requirements else {
                return;
            };

            let packages: Vec<String> = reqs
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if packages.is_empty() {
                return;
            }

            info!(
                "Starting background recovery of linux packages: {:?}",
                packages
            );

            let mut cmd_update = tokio::process::Command::new("apt-get");
            cmd_update.arg("update");
            let _ = cmd_update.output().await;

            let mut cmd = tokio::process::Command::new("apt-get");
            cmd.arg("install").arg("-y").args(&packages);
            cmd.env("DEBIAN_FRONTEND", "noninteractive");

            match cmd.output().await {
                Ok(output) if output.status.success() => {
                    info!("Successfully recovered Shell packages.");
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("Failed to recover Shell packages: {}", stderr);
                }
                Err(e) => {
                    error!("Error executing apt-get for package recovery: {}", e);
                }
            }
        }
    });
}

pub fn spawn_upgrade_self_check(db: DatabaseConnection, event_bus: EventBus) {
    tokio::spawn(async move {
        match collect_upgrade_self_check_report(&db).await {
            Ok(Some(report)) => {
                warn!("Upgrade self-check found issues: {}", report);
                event_bus.publish(SystemEvent::System(SystemNotification::Alert {
                    message: report,
                }));
            }
            Ok(None) => {
                info!("Upgrade self-check passed");
            }
            Err(e) => {
                warn!("Upgrade self-check failed to run: {}", e);
                event_bus.publish(SystemEvent::System(SystemNotification::Alert {
                    message: format!("升级后自检执行失败: {}", e),
                }));
            }
        }
    });
}

async fn collect_upgrade_self_check_report(db: &DatabaseConnection) -> Result<Option<String>> {
    let mut findings = Vec::new();

    if !has_index(db, "idx-users-email").await? {
        findings.push("用户邮箱唯一索引 `idx-users-email` 不存在".to_string());
    }

    let duplicate_emails = duplicate_user_emails(db).await?;
    if !duplicate_emails.is_empty() {
        let samples = duplicate_emails
            .into_iter()
            .take(10)
            .map(|(email, count)| format!("{}({})", email, count))
            .collect::<Vec<_>>()
            .join(", ");
        findings.push(format!("发现重复邮箱: {}", samples));
    }

    if findings.is_empty() {
        Ok(None)
    } else {
        Ok(Some(findings.join("; ")))
    }
}

async fn duplicate_user_emails(db: &DatabaseConnection) -> Result<Vec<(String, i64)>> {
    let sql = r#"
        SELECT email, COUNT(*) AS cnt
        FROM users
        WHERE email IS NOT NULL AND email != ''
        GROUP BY email
        HAVING COUNT(*) > 1
        ORDER BY cnt DESC, email ASC
    "#;

    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            sql.to_string(),
        ))
        .await?;

    let mut result = Vec::new();
    for row in rows {
        let email: String = row.try_get_by_index(0)?;
        let count: i64 = row.try_get_by_index(1)?;
        result.push((email, count));
    }
    Ok(result)
}

async fn has_index(db: &DatabaseConnection, index_name: &str) -> Result<bool> {
    let sql = format!(
        "SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = '{}'",
        index_name.replace('\'', "''")
    );
    let row = db
        .query_one_raw(Statement::from_string(db.get_database_backend(), sql))
        .await?;
    Ok(row.is_some())
}

pub fn spawn_system_metrics_updater(system_metrics: SharedSystemMetrics) {
    tokio::spawn(async move {
        let refresh_kind = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::nothing().with_ram());
        let mut system = System::new_with_specifics(refresh_kind);
        let mut networks = Networks::new();
        let mut network_sampler = NetworkSampler::default();
        let os_info = System::long_os_version().unwrap_or_else(|| "Unknown".to_string());
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            networks.refresh(true);
            let (network_upload_speed, network_download_speed) = network_sampler.sample(
                networks.iter().map(|(name, data)| {
                    (
                        name.as_str(),
                        data.total_transmitted(),
                        data.total_received(),
                    )
                }),
                std::time::Instant::now(),
            );

            system.refresh_cpu_usage();
            system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
            let (memory_total, memory_used) =
                visible_memory(system.total_memory(), system.used_memory());

            let metrics = niupanel_common::metrics::SystemMetrics {
                cpu_usage: system.global_cpu_usage(),
                memory_total,
                memory_used,
                network_upload_speed,
                network_download_speed,
                uptime: System::uptime(),
                os_info: os_info.clone(),
            };

            let mut m = system_metrics.write().await;
            *m = metrics;
        }
    });
}

fn visible_memory(host_total: u64, host_used: u64) -> (u64, u64) {
    const CGROUP_V2: (&str, &str) = ("/sys/fs/cgroup/memory.max", "/sys/fs/cgroup/memory.current");
    const CGROUP_V1: (&str, &str) = (
        "/sys/fs/cgroup/memory/memory.limit_in_bytes",
        "/sys/fs/cgroup/memory/memory.usage_in_bytes",
    );

    for (limit_path, current_path) in [CGROUP_V2, CGROUP_V1] {
        let Some(limit) = read_cgroup_memory_value(limit_path) else {
            continue;
        };
        let Some(current) = read_cgroup_memory_value(current_path) else {
            continue;
        };

        // Bare-metal hosts commonly expose `max` or a sentinel much larger
        // than physical RAM. Only a real, tighter container limit should
        // replace host memory in the status UI.
        if limit > 0 && limit < host_total {
            return (limit, current.min(limit));
        }
    }

    (host_total, host_used)
}

fn read_cgroup_memory_value(path: impl AsRef<Path>) -> Option<u64> {
    let raw = std::fs::read_to_string(path).ok()?;
    parse_cgroup_memory_value(&raw)
}

fn parse_cgroup_memory_value(raw: &str) -> Option<u64> {
    let value = raw.trim();
    if value.is_empty() || value == "max" {
        return None;
    }
    value.parse().ok()
}

fn spawn_notification_handler(
    db: DatabaseConnection,
    event_bus: EventBus,
    http_client: reqwest::Client,
) {
    let notify_handler = NotificationHandler::new(event_bus, db, http_client);
    tokio::spawn(async move {
        notify_handler.run().await;
    });
}

fn spawn_db_sync_handler(db: DatabaseConnection, event_bus: EventBus) {
    let db_sync_handler = DbSyncHandler::new(event_bus, db);
    tokio::spawn(async move {
        db_sync_handler.run().await;
    });
}

async fn start_api_keys_cleanup_scheduler(db: DatabaseConnection) {
    use niupanel_entity::api_keys;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        loop {
            interval.tick().await;
            info!("运行过期 API Key 清理任务...");

            let now = chrono::Utc::now();
            let res = api_keys::Entity::delete_many()
                .filter(api_keys::Column::ExpiresAt.lt(now))
                .exec(&db)
                .await;

            match res {
                Ok(res) if res.rows_affected > 0 => {
                    info!("已清理 {} 个过期的 API Key", res.rows_affected);
                }
                Err(e) => error!("清理过期 API Key 失败: {}", e),
                _ => {}
            }
        }
    });
}

async fn start_login_attempts_cleanup_scheduler(db: DatabaseConnection) {
    use niupanel_entity::login_attempts;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        loop {
            interval.tick().await;
            info!("启动登录日志定时清除任务");

            let threshold = chrono::Utc::now() - chrono::Duration::days(7);
            let res = login_attempts::Entity::delete_many()
                .filter(login_attempts::Column::LastAttemptAt.lt(threshold))
                .exec(&db)
                .await;

            match res {
                Ok(res) if res.rows_affected > 0 => {
                    info!("Cleaned up {} old login attempt records", res.rows_affected);
                }
                Err(e) => error!("Failed to cleanup login attempts: {}", e),
                _ => {}
            }
        }
    });
}

async fn start_sandbox_environments_recovery(db: DatabaseConnection) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        info!("Running sandbox environments auto-recovery check...");

        use niupanel_core::runtime::RuntimeManager;
        use niupanel_core::settings::SettingsManager;
        use std::path::PathBuf;

        match niupanel_entity::environments::Entity::find().all(&db).await {
            Ok(envs) => {
                let mut mirrors = std::collections::HashMap::new();
                if let Ok(m) = SettingsManager::get(&db, UV_PYTHON_MIRROR).await {
                    mirrors.insert("UV_PYTHON_INSTALL_MIRROR".to_string(), m);
                }
                if let Ok(m) = SettingsManager::get(&db, UV_PYPI_MIRROR).await {
                    mirrors.insert("UV_INDEX_URL".to_string(), m);
                }
                if let Ok(m) = SettingsManager::get(&db, PNPM_NODE_DIST_MIRROR).await {
                    mirrors.insert("PNPM_NODE_DIST_MIRROR".to_string(), m);
                }
                if let Ok(m) = SettingsManager::get(&db, NPM_REGISTRY_MIRROR).await {
                    mirrors.insert("npm_config_registry".to_string(), m);
                }

                for env_model in envs {
                    if env_model.name == "System"
                        || env_model.name == "System Node"
                        || env_model.name == "System Shell"
                    {
                        continue;
                    }

                    let base_folder = if env_model.env_type == "python" {
                        "python"
                    } else if env_model.env_type == "node" {
                        "node"
                    } else {
                        continue;
                    };

                    if env_model.env_type == "python" {
                        let env_dir =
                            PathBuf::from(&niupanel_common::config::Config::global().runtimes_dir)
                                .join(base_folder)
                                .join(&env_model.name);

                        if env_dir.exists() {
                            continue;
                        }
                        info!(
                            "Python environment '{}' is missing. Initiating auto-recovery...",
                            env_model.name
                        );
                    }

                    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                    let log_name = env_model.name.clone();

                    tokio::spawn(async move {
                        while let Some(msg) = rx.recv().await {
                            niupanel_common::debug!("[Recovery {}] {}", log_name, msg);
                        }
                    });

                    let req_text = env_model.requirements.clone().unwrap_or_default();
                    let requirements: Vec<String> = req_text
                        .lines()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    if env_model.env_type == "python" {
                        let env_dir =
                            PathBuf::from(&niupanel_common::config::Config::global().runtimes_dir)
                                .join(base_folder)
                                .join(&env_model.name);
                        match RuntimeManager::open_python_environment(
                            env_dir,
                            Some(&env_model.version),
                            Some(mirrors.clone()),
                        )
                        .await
                        {
                            Ok(env) => {
                                if !req_text.is_empty() {
                                    let _ = env
                                        .install_requirements(&req_text, false, tx.clone())
                                        .await;
                                }
                                info!("Successfully recovered python sandbox: {}", env_model.name);
                            }
                            Err(e) => {
                                error!("Failed to recover python sandbox {}: {}", env_model.name, e)
                            }
                        }
                    } else if env_model.env_type == "node" {
                        let local_versions = RuntimeManager::list_local_node_versions()
                            .await
                            .unwrap_or_default();
                        let version_installed =
                            local_versions.iter().any(|(v, _)| v == &env_model.version);
                        let mut can_restore_node_packages = version_installed;

                        if !version_installed {
                            info!(
                                "Node environment '{}' is missing. Initiating auto-recovery...",
                                env_model.name
                            );
                            match RuntimeManager::create_node_environment(
                                env_model.version.clone(),
                                tx.clone(),
                                Some(mirrors.clone()),
                            )
                            .await
                            {
                                Ok(_) => {
                                    can_restore_node_packages = true;
                                    info!(
                                        "Successfully recovered node environment: {}",
                                        env_model.name
                                    );
                                }
                                Err(e) => error!(
                                    "Failed to recover node environment {}: {}",
                                    env_model.name, e
                                ),
                            }
                        } else {
                            info!(
                                "Node environment '{}' is already installed. Skipping recovery.",
                                env_model.name
                            );
                        }

                        if can_restore_node_packages && !requirements.is_empty() {
                            let env = match RuntimeManager::open_node_environment(
                                Some(&env_model.version),
                                Some(mirrors.clone()),
                            ) {
                                Ok(env) => env,
                                Err(e) => {
                                    error!(
                                        "Failed to open node environment during recovery {}: {}",
                                        env_model.name, e
                                    );
                                    continue;
                                }
                            };
                            let _ = env.install_packages(&requirements, tx.clone()).await;
                        }
                    }
                }
            }
            Err(e) => error!("Recovery check failed to fetch environments: {}", e),
        }

        info!("Sandbox environments auto-recovery check completed.");
    });
}

#[cfg(test)]
mod metrics_tests {
    use super::parse_cgroup_memory_value;

    #[test]
    fn parses_finite_cgroup_memory_values() {
        assert_eq!(parse_cgroup_memory_value("536870912\n"), Some(536_870_912));
        assert_eq!(parse_cgroup_memory_value("max\n"), None);
        assert_eq!(parse_cgroup_memory_value("invalid"), None);
    }
}
