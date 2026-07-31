mod cleanup;
mod execution;
mod logging;
mod scheduling;
mod system_jobs;

use crate::event_bus::EventBus;
use crate::settings::SettingsManager;
use crate::sys::process::ProcessManager;
use crate::sys::status_manager::StatusManager;
use crate::task_log::LogSession;
use niupanel_common::constants::defaults;
use niupanel_common::constants::script::INTERPRETERS;
use niupanel_common::constants::settings::SYSTEM_MAX_CONCURRENCY;
use niupanel_common::error::AppError;
use niupanel_common::{error as error_msg, info, warn};
use sea_orm::DatabaseConnection;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use tokio_cron_scheduler::JobScheduler;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RandomTaskConfig {
    pub start: String,
    pub end: String,
    pub count: i32,
}

#[derive(Clone)]
pub struct TaskManagerService {
    event_bus: EventBus,
    process_manager: ProcessManager,
    status_manager: StatusManager,
    db: DatabaseConnection,
    log_sessions: Arc<dashmap::DashMap<String, LogSession>>,
    active_task_runs: Arc<dashmap::DashMap<i32, i32>>,
    starting_tasks: Arc<dashmap::DashSet<i32>>,
    scheduler: Arc<JobScheduler>,
    scheduled_jobs: Arc<dashmap::DashMap<i32, Uuid>>,
    task_schedule_lock: Arc<tokio::sync::Mutex<()>>,
    workflow_schedule_lock: Arc<tokio::sync::Mutex<()>>,
    running_permits: Arc<tokio::sync::Semaphore>,
    current_max_concurrency: Arc<Mutex<usize>>,
    system_job_handles: Arc<dashmap::DashMap<i32, tokio::task::AbortHandle>>,
}

pub fn filter_args(args: &[String], script_path: &std::path::Path) -> Vec<String> {
    if args.is_empty() {
        return Vec::new();
    }

    let mut start_index = 0;

    if let Some(first_arg) = args.get(start_index) {
        if INTERPRETERS.contains(&first_arg.as_str()) {
            start_index += 1;
        }
    }

    if let Some(next_arg) = args.get(start_index) {
        let arg_path = std::path::Path::new(next_arg);
        let name_match = match (script_path.file_name(), arg_path.file_name()) {
            (Some(s), Some(a)) => s == a,
            _ => false,
        };
        let path_match = script_path.ends_with(arg_path);

        if name_match || path_match {
            start_index += 1;
        }
    }

    args[start_index..].to_vec()
}

pub(super) fn task_run_log_key(run_id: i32) -> String {
    format!("task-run:{}", run_id)
}

pub(super) fn job_log_key(job_id: i32) -> String {
    format!("job:{}", job_id)
}

impl TaskManagerService {
    pub async fn new(db: DatabaseConnection, event_bus: EventBus) -> Result<Self, AppError> {
        let status_manager = StatusManager::new();
        let process_manager = ProcessManager::new(status_manager.clone());
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| AppError::Generic(format!("创建调度器失败: {}", e)))?;

        let max_concurrency_str = SettingsManager::get(&db, SYSTEM_MAX_CONCURRENCY).await?;
        let max_concurrency = max_concurrency_str
            .parse::<usize>()
            .ok()
            .filter(|value| (1..=1024).contains(value))
            .unwrap_or_else(|| {
                warn!(
                    "无效的最大并发数 '{}'，回退为 {}",
                    max_concurrency_str,
                    defaults::MAX_CONCURRENCY
                );
                defaults::MAX_CONCURRENCY
            });
        info!("任务管理器初始化，最大并发数: {}", max_concurrency);

        let service = Self {
            event_bus,
            process_manager,
            status_manager,
            db,
            log_sessions: Arc::new(dashmap::DashMap::new()),
            active_task_runs: Arc::new(dashmap::DashMap::new()),
            starting_tasks: Arc::new(dashmap::DashSet::new()),
            scheduler: Arc::new(scheduler),
            scheduled_jobs: Arc::new(dashmap::DashMap::new()),
            task_schedule_lock: Arc::new(tokio::sync::Mutex::new(())),
            workflow_schedule_lock: Arc::new(tokio::sync::Mutex::new(())),
            running_permits: Arc::new(Semaphore::new(max_concurrency)),
            current_max_concurrency: Arc::new(Mutex::new(max_concurrency)),
            system_job_handles: Arc::new(dashmap::DashMap::new()),
        };

        service.spawn_status_listener();
        service.spawn_settings_listener();

        if let Err(e) = service.cleanup_zombie_tasks().await {
            warn!("Failed to cleanup zombie tasks: {}", e);
        }

        service
            .scheduler
            .start()
            .await
            .map_err(|e| AppError::Generic(format!("启动调度器失败: {}", e)))?;

        if let Err(e) = service.schedule_daily_random_rebalancer().await {
            error_msg!("Failed to schedule daily random rebalancer: {}", e);
        }

        match service.load_scheduled_tasks().await {
            Ok(_) => {
                info!("已加载并计划 {} 个任务", service.scheduled_jobs.len());
            }
            Err(e) => {
                error_msg!("Failed to load scheduled tasks: {}", e);
            }
        };

        if let Err(e) = service.load_scheduled_workflows().await {
            error_msg!("Failed to load scheduled workflows: {}", e);
        }

        Ok(service)
    }
}
