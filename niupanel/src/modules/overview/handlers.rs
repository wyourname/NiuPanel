use crate::common::state::AppState;
use axum::extract::State;
use chrono::Utc;
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::ApiResponse;
use niupanel_entity::task_status::TaskStatus;
use niupanel_entity::{task_runs, tasks};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Serialize;
use sysinfo::Disks;
use utoipa::ToSchema;

use axum::response::sse::{Event, KeepAlive, Sse};
use futures::Stream;
use std::convert::Infallible;

#[utoipa::path(
    get,
    path = "/api/v1/overview/events",
    responses(
        (status = 200, description = "SSE stream of system events", content_type = "text/event-stream")
    ),
    tag = "Overview",
    security(("session_cookie" = []))
)]
pub async fn events_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let mut rx = state.event_bus.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Ok(json) = serde_json::to_string(&event) {
                        yield Ok(Event::default().data(json));
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[derive(Serialize, ToSchema)]
pub struct SystemOverview {
    cpu_usage: f32,
    memory_total: u64,
    memory_used: u64,
    disk_total: u64,
    disk_used: u64,
    uptime: u64,
    os_info: String,
    public_ip: Option<String>,
    task_stats: TaskStats,
    recent_activity: Vec<ActivityItem>,
    chart_data: ChartData,
}

#[derive(Serialize, ToSchema)]
pub struct TaskStats {
    total: i64,
    running: i64,
    failed_today: i64,
    next_run: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct ActivityItem {
    id: i32,
    task_name: String,
    status: TaskStatus,
    time: i64,
    duration: Option<String>, // simplified for display
}

#[derive(Serialize, ToSchema)]
pub struct ChartData {
    hours: Vec<String>,
    success: Vec<i64>,
    failed: Vec<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/overview",
    responses(
        (status = 200, description = "Get system overview data")
    ),
    tag = "Overview",
    security(("session_cookie" = []))
)]
pub async fn get_overview(State(state): State<AppState>) -> Result<ApiResponse<SystemOverview>> {
    // 1. System Metrics (Read from cache)
    let (cpu_usage, memory_total, memory_used, uptime, os_info) = {
        let metrics = state.system_metrics.read().await.clone();
        (
            metrics.cpu_usage,
            metrics.memory_total,
            metrics.memory_used,
            metrics.uptime,
            metrics.os_info.clone(),
        )
    };

    let disks = Disks::new_with_refreshed_list();

    // Find the disk that contains the data directory
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let target_path = current_dir.join("data");
    let search_path = if target_path.exists() {
        target_path
    } else {
        current_dir
    };

    let mut best_disk = None;
    let mut max_len = 0;

    for disk in &disks {
        let mount = disk.mount_point();
        if search_path.starts_with(mount) {
            let len = mount.as_os_str().len();
            if len >= max_len {
                max_len = len;
                best_disk = Some(disk);
            }
        }
    }

    let (disk_total, disk_used) = if let Some(disk) = best_disk {
        (
            disk.total_space(),
            disk.total_space() - disk.available_space(),
        )
    } else {
        disks
            .iter()
            .find(|d| d.mount_point() == std::path::Path::new("/"))
            .map(|d| (d.total_space(), d.total_space() - d.available_space()))
            .unwrap_or((0, 0))
    };

    // 2. Task Stats
    let total_tasks = tasks::Entity::find().count(&state.db).await?;

    // Running count (from DB is safer than memory for now, though memory is more real-time)
    let running_tasks = task_runs::Entity::find()
        .filter(task_runs::Column::Status.eq(TaskStatus::Running))
        .count(&state.db)
        .await?;

    // Failed Today
    let now = Utc::now();
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::Internal("Failed to calculate start of day".to_string()))?
        .and_utc();

    let failed_today = task_runs::Entity::find()
        .filter(task_runs::Column::Status.eq(TaskStatus::Failed))
        .filter(task_runs::Column::StartedAt.gte(today_start))
        .count(&state.db)
        .await?;

    let next_task = tasks::Entity::find()
        .filter(tasks::Column::Enabled.eq(true))
        .filter(tasks::Column::NextRunAt.is_not_null())
        .filter(tasks::Column::NextRunAt.gt(now))
        .order_by_asc(tasks::Column::NextRunAt)
        .one(&state.db)
        .await?;

    let next_run = next_task.and_then(|t| t.next_run_at.map(|d| d.timestamp()));

    // Public IP with Cache
    let public_ip = {
        let cache_guard = state
            .public_ip
            .read()
            .map_err(|_| AppError::Internal("Public IP cache lock poisoned".to_string()))?;
        let mut cached_ip = None;
        let mut needs_update = true;

        if let Some((ip, last_update)) = &*cache_guard {
            cached_ip = Some(ip.clone());
            if last_update.elapsed() < std::time::Duration::from_secs(3600) {
                needs_update = false;
            }
        }

        if needs_update {
            let public_ip_store = state.public_ip.clone();
            let http_client = state.http_client.clone();
            let settings = state.settings.clone();
            tokio::spawn(async move {
                let url = settings
                    .get("system.ip_check_url")
                    .await
                    .unwrap_or_else(|_| "https://ipcheck.365676.xyz".to_string());
                if let Ok(resp) = http_client
                    .get(&url)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                {
                    if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                        if let Some(ip) = json_data
                            .get("Info")
                            .and_then(|info| info.get("IP"))
                            .and_then(|v| v.as_str())
                        {
                            if let Ok(mut guard) = public_ip_store.write() {
                                *guard = Some((ip.to_string(), std::time::Instant::now()));
                            }
                        }
                    }
                }
            });
        }
        cached_ip
    };

    // 3. Recent Activity (Last 5 finished/failed/stopped)
    let recent_runs = task_runs::Entity::find()
        .find_also_related(tasks::Entity)
        .order_by_desc(task_runs::Column::EndedAt)
        .limit(5)
        .all(&state.db)
        .await?;

    let recent_activity = recent_runs
        .into_iter()
        .map(|(run, task_opt)| {
            let task_name = task_opt
                .map(|t| t.name)
                .unwrap_or_else(|| "Deleted Task".to_string());
            let duration = if let (Some(end), start) = (run.ended_at, run.started_at) {
                let diff = end - start;
                Some(format!("{}s", diff.num_seconds()))
            } else {
                None
            };

            ActivityItem {
                id: run.id,
                task_name,
                status: run.status,
                time: run
                    .ended_at
                    .map(|t| t.timestamp())
                    .unwrap_or(run.started_at.timestamp()),
                duration,
            }
        })
        .collect();

    let yesterday = now - chrono::Duration::hours(24);

    let last_24h_runs = task_runs::Entity::find()
        .filter(task_runs::Column::StartedAt.gte(yesterday))
        .all(&state.db)
        .await?;

    let mut hours_map: std::collections::HashMap<String, (i64, i64)> =
        std::collections::HashMap::new();

    // Initialize map with last 24 hours
    for i in 0..24 {
        let h = now - chrono::Duration::hours(i);
        let key = h.format("%H:00").to_string();
        hours_map.insert(key, (0, 0));
    }

    for run in last_24h_runs {
        let key = run
            .started_at
            .with_timezone(&chrono::Local)
            .format("%H:00")
            .to_string();
        if let Some(entry) = hours_map.get_mut(&key) {
            if run.status == TaskStatus::Finished {
                entry.0 += 1;
            } else if run.status == TaskStatus::Failed {
                entry.1 += 1;
            }
        } else {
            // Maybe fell into a gap or different timezone issue, insert if missing?
            // Ideally we use the initialized map keys.
            // If local time varies, we should stick to UTC or fixed offset.
            // Let's rely on chrono::Local for display.
            hours_map.entry(key).or_insert((0, 0));
        }
    }

    // Sort by time (hacky but works for display)
    let chart_entries: Vec<(String, (i64, i64))> = hours_map.into_iter().collect();
    // To sort correctly we need to know the order.
    // Let's regenerate the keys in order.
    let mut hours = Vec::new();
    let mut success = Vec::new();
    let mut failed = Vec::new();

    for i in (0..24).rev() {
        let h = now - chrono::Duration::hours(i);
        let key = h.with_timezone(&chrono::Local).format("%H:00").to_string();

        // Find in entries (inefficient but 24 items is tiny)
        let found = chart_entries.iter().find(|(k, _)| *k == key);
        if let Some((_, (s, f))) = found {
            hours.push(key);
            success.push(*s);
            failed.push(*f);
        } else {
            hours.push(key);
            success.push(0);
            failed.push(0);
        }
    }

    Ok(ApiResponse::success(SystemOverview {
        cpu_usage,
        memory_total,
        memory_used,
        disk_total,
        disk_used,
        uptime,
        os_info,
        public_ip,
        task_stats: TaskStats {
            total: total_tasks as i64,
            running: running_tasks as i64,
            failed_today: failed_today as i64,
            next_run,
        },
        recent_activity,
        chart_data: ChartData {
            hours,
            success,
            failed,
        },
    }))
}
