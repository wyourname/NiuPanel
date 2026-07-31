use super::{RandomTaskConfig, TaskManagerService};
use crate::event_bus::{SystemEvent, SystemNotification, TelegramEvent};
use crate::settings::SettingsManager;
use chrono::{Datelike, NaiveTime, Timelike};
use chrono_tz::Tz;
use niupanel_common::constants::settings::{SYSTEM_MAX_CONCURRENCY, SYSTEM_TIMEZONE};
use niupanel_common::error::AppError;
use niupanel_common::{error as error_msg, info, warn};
use rand::Rng;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::str::FromStr;
use tokio::sync::broadcast;
use tokio_cron_scheduler::Job;

impl TaskManagerService {
    pub async fn get_system_timezone(&self) -> Tz {
        let default_tz = chrono_tz::Asia::Shanghai;

        if let Ok(val) = SettingsManager::get(&self.db, SYSTEM_TIMEZONE).await {
            if let Ok(tz) = Tz::from_str(&val) {
                return tz;
            } else if val == "UTC" {
                return chrono_tz::UTC;
            }
        }

        default_tz
    }

    pub(crate) fn spawn_settings_listener(&self) {
        let initial_max = *self.current_max_concurrency.lock().expect("Mutex poisoned");
        let (desired_tx, mut desired_rx) = tokio::sync::watch::channel(initial_max);
        let event_service = self.clone();
        tokio::spawn(async move {
            let mut rx = event_service.event_bus.subscribe();
            loop {
                match rx.recv().await {
                    Ok(SystemEvent::System(SystemNotification::SettingChanged { key, value })) => {
                        if key == SYSTEM_MAX_CONCURRENCY {
                            if let Ok(new_max @ 1..=1024) = value.parse::<usize>() {
                                desired_tx.send_replace(new_max);
                            } else {
                                warn!("无法解析新的最大并发数 '{}'。", value);
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("设置监听器处理落后，跳过了 {} 个事件", skipped);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("设置监听器通道关闭");
                        break;
                    }
                }
            }
        });

        let controller_service = self.clone();
        tokio::spawn(async move {
            loop {
                let desired = *desired_rx.borrow_and_update();
                let current = *controller_service
                    .current_max_concurrency
                    .lock()
                    .expect("Mutex poisoned");

                if desired > current {
                    controller_service
                        .running_permits
                        .add_permits(desired - current);
                    *controller_service
                        .current_max_concurrency
                        .lock()
                        .expect("Mutex poisoned") = desired;
                    info!("动态调整任务最大并发数: {} -> {}", current, desired);
                    continue;
                }

                if desired < current {
                    let reduction = current - desired;
                    info!(
                        "动态缩减任务最大并发数: {} -> {}，等待回收 {} 个许可...",
                        current, desired, reduction
                    );
                    let acquire = controller_service
                        .running_permits
                        .clone()
                        .acquire_many_owned(reduction as u32);
                    tokio::pin!(acquire);
                    tokio::select! {
                        result = &mut acquire => {
                            match result {
                                Ok(permits) => {
                                    permits.forget();
                                    *controller_service
                                        .current_max_concurrency
                                        .lock()
                                        .expect("Mutex poisoned") = desired;
                                    info!("任务最大并发数已成功缩减至 {}", desired);
                                }
                                Err(_) => {
                                    warn!("任务并发控制器已关闭");
                                    break;
                                }
                            }
                        }
                        changed = desired_rx.changed() => {
                            if changed.is_err() {
                                break;
                            }
                            // Cancelling the acquire future returns any partially
                            // acquired permits before recomputing the latest target.
                            continue;
                        }
                    }
                    continue;
                }

                if desired_rx.changed().await.is_err() {
                    break;
                }
            }
        });
    }

    pub(crate) async fn load_scheduled_tasks(&self) -> Result<(), AppError> {
        let tasks = niupanel_entity::tasks::Entity::find().all(&self.db).await?;

        for task in tasks {
            if task.enabled {
                if let Some(cron) = &task.cron_schedule {
                    if !cron.trim().is_empty() {
                        self.schedule_task(&task).await?;
                    }
                }

                if task.random_config.is_some() {
                    self.schedule_random_task_for_day(&task).await?;
                }
            }

            if let Some(cron) = &task.cron_schedule {
                if !cron.trim().is_empty() {
                    let next_run = self.calculate_next_run(&task.cron_schedule).await;
                    let mut task_active: niupanel_entity::tasks::ActiveModel = task.clone().into();
                    task_active.next_run_at = Set(next_run);
                    let _ = task_active.update(&self.db).await;
                }
            }
        }
        Ok(())
    }

    async fn schedule_task(&self, task: &niupanel_entity::tasks::Model) -> Result<(), AppError> {
        let service = self.clone();
        let task_id = task.id;
        let task_clone = task.clone();

        let cron = match &task.cron_schedule {
            Some(c) if !c.trim().is_empty() => Self::normalize_cron(c),
            _ => return Ok(()),
        };

        let tz = self.get_system_timezone().await;
        let job = Job::new_async_tz(cron.as_str(), tz, move |_uuid, _l| {
            let service = service.clone();
            let task = task_clone.clone();
            Box::pin(async move {
                info!("执行计划任务: {} ({})", task.name, task_id);
                if let Err(e) = service.start_task(&task).await {
                    error_msg!("执行计划任务 {} 失败: {}", task_id, e);
                }
            })
        })
        .map_err(|e| {
            AppError::Generic(format!(
                "无效的Cron表达式 '{}'，任务 {}: {}",
                cron, task_id, e
            ))
        })?;

        let job_uuid = self.scheduler.add(job).await.map_err(|e| {
            AppError::Generic(format!("为任务 {} 添加调度任务失败: {}", task_id, e))
        })?;
        self.scheduled_jobs.insert(task_id, job_uuid);
        info!("已调度任务 {}，Cron: {} (时区: {:?})", task.name, cron, tz);
        Ok(())
    }

    async fn schedule_random_task_for_day(
        &self,
        task: &niupanel_entity::tasks::Model,
    ) -> Result<(), AppError> {
        let config_json = match &task.random_config {
            Some(c) => c,
            None => return Ok(()),
        };

        let config: RandomTaskConfig = serde_json::from_value(config_json.clone())
            .map_err(|e| AppError::Generic(format!("无效的随机运行配置: {}", e)))?;

        let tz = self.get_system_timezone().await;
        let now = chrono::Utc::now().with_timezone(&tz);

        let start_time = NaiveTime::parse_from_str(&config.start, "%H:%M")
            .map_err(|e| AppError::Generic(format!("无效的开始时间 {}: {}", config.start, e)))?;
        let end_time = NaiveTime::parse_from_str(&config.end, "%H:%M")
            .map_err(|e| AppError::Generic(format!("无效结束时间 {}: {}", config.end, e)))?;

        if start_time >= end_time {
            return Err(AppError::Generic(
                "随机运行的结束时间必须晚于开始时间".to_string(),
            ));
        }

        let start_seconds = start_time.hour() * 3600 + start_time.minute() * 60;
        let end_seconds = end_time.hour() * 3600 + end_time.minute() * 60;

        let service = self.clone();
        let task_clone = task.clone();

        for i in 0..config.count {
            let random_offset = rand::rng().random_range(0..(end_seconds - start_seconds));
            let target_seconds = start_seconds + random_offset;
            let Some(target_time) =
                NaiveTime::from_num_seconds_from_midnight_opt(target_seconds, 0)
            else {
                warn!("任务 {} 生成了无效的随机执行时间", task.id);
                continue;
            };
            let Some(target_dt) = now
                .date_naive()
                .and_time(target_time)
                .and_local_timezone(tz)
                .earliest()
            else {
                warn!(
                    "任务 {} 的随机执行时间落在时区 {} 的无效本地时间，已跳过",
                    task.id, tz
                );
                continue;
            };

            if target_dt <= now {
                continue;
            }

            let service = service.clone();
            let task_for_closure = task_clone.clone();
            let once_cron = format!(
                "{} {} {} {} {} * {}",
                target_time.second(),
                target_time.minute(),
                target_time.hour(),
                target_dt.day(),
                target_dt.month(),
                target_dt.year()
            );

            info!(
                "安排随机任务点: {} (ID: {}), 预计执行时间: {}, Cron: {}",
                task.name, task.id, target_dt, once_cron
            );

            let job = Job::new_async_tz(once_cron.as_str(), tz, move |_uuid, _l| {
                let service = service.clone();
                let task = task_for_closure.clone();
                Box::pin(async move {
                    info!("正在触发随机计划任务: {} (ID: {})", task.name, task.id);
                    if let Err(e) = service.start_task(&task).await {
                        error_msg!("随机计划任务 {} 启动失败: {}", task.id, e);
                    }
                })
            })
            .map_err(|e| AppError::Generic(format!("创建随机作业失败: {}", e)))?;

            let job_uuid = self
                .scheduler
                .add(job)
                .await
                .map_err(|e| AppError::Generic(format!("添加随机作业失败: {}", e)))?;

            self.scheduled_jobs
                .insert(task_clone.id + (i + 1) * 1000000, job_uuid);
        }

        info!(
            "已为任务 {} 安排了 {} 次随机运行点",
            task.name, config.count
        );
        Ok(())
    }

    pub(crate) async fn schedule_daily_random_rebalancer(&self) -> Result<(), AppError> {
        let service = self.clone();
        let cron = "1 0 0 * * *";
        let tz = self.get_system_timezone().await;

        let job = Job::new_async_tz(cron, tz, move |_uuid, _l| {
            let service = service.clone();
            Box::pin(async move {
                info!("正在重新平衡每日随机任务...");
                if let Err(e) = service.rebalance_all_random_tasks().await {
                    error_msg!("重新平衡随机任务失败: {}", e);
                }
            })
        })
        .map_err(|e| AppError::Generic(format!("创建再平衡作业失败: {}", e)))?;

        self.scheduler
            .add(job)
            .await
            .map_err(|e| AppError::Generic(format!("添加再平衡作业失败: {}", e)))?;
        Ok(())
    }

    async fn rebalance_all_random_tasks(&self) -> Result<(), AppError> {
        let _guard = self.task_schedule_lock.lock().await;
        let tasks = niupanel_entity::tasks::Entity::find()
            .filter(niupanel_entity::tasks::Column::Enabled.eq(true))
            .filter(niupanel_entity::tasks::Column::RandomConfig.is_not_null())
            .all(&self.db)
            .await?;

        for task in tasks {
            let mut keys_to_remove = Vec::new();
            for item in self.scheduled_jobs.iter() {
                let key = *item.key();
                if key % 1000000 == task.id && key != task.id {
                    keys_to_remove.push(key);
                }
            }

            for key in keys_to_remove {
                if let Some((_, uuid)) = self.scheduled_jobs.remove(&key) {
                    let _ = self.scheduler.remove(&uuid).await;
                }
            }

            let _ = self.schedule_random_task_for_day(&task).await;
        }
        Ok(())
    }

    pub async fn update_task_schedule(
        &self,
        task: &niupanel_entity::tasks::Model,
    ) -> Result<(), AppError> {
        let _guard = self.task_schedule_lock.lock().await;
        self.remove_task_schedule_locked(task.id).await?;
        if task.enabled {
            self.schedule_task(task).await?;
            self.schedule_random_task_for_day(task).await?;
        }

        let next_run = self.calculate_next_run(&task.cron_schedule).await;
        let mut task_active: niupanel_entity::tasks::ActiveModel = task.clone().into();
        task_active.next_run_at = Set(next_run);
        task_active.update(&self.db).await?;
        Ok(())
    }

    pub async fn remove_task_schedule(&self, task_id: i32) -> Result<(), AppError> {
        let _guard = self.task_schedule_lock.lock().await;
        self.remove_task_schedule_locked(task_id).await
    }

    async fn remove_task_schedule_locked(&self, task_id: i32) -> Result<(), AppError> {
        let job_uuid = self
            .scheduled_jobs
            .get(&task_id)
            .map(|entry| *entry.value());
        if let Some(job_uuid) = job_uuid {
            self.scheduler
                .remove(&job_uuid)
                .await
                .map_err(|e| AppError::Generic(format!("从调度器移除任务失败: {}", e)))?;
            self.scheduled_jobs.remove(&task_id);
            info!("已移除计划任务 {}", task_id);
        }

        let mut keys_to_remove = Vec::new();
        for item in self.scheduled_jobs.iter() {
            let key = *item.key();
            if key % 1000000 == task_id && key != task_id {
                keys_to_remove.push(key);
            }
        }
        for key in keys_to_remove {
            let uuid = self.scheduled_jobs.get(&key).map(|entry| *entry.value());
            if let Some(uuid) = uuid {
                self.scheduler.remove(&uuid).await.map_err(|e| {
                    AppError::Generic(format!(
                        "从调度器移除任务 {} 的随机计划失败: {}",
                        task_id, e
                    ))
                })?;
                self.scheduled_jobs.remove(&key);
            }
        }

        Ok(())
    }

    pub(crate) async fn load_scheduled_workflows(&self) -> Result<(), AppError> {
        let workflows = niupanel_entity::tg_workflows::Entity::find()
            .filter(niupanel_entity::tg_workflows::Column::EventType.eq("cron"))
            .all(&self.db)
            .await?;

        for wf in workflows {
            if let Err(err) = self.schedule_workflow(&wf).await {
                warn!("加载工作流 {} 的调度失败: {}", wf.id, err);
            }
        }
        Ok(())
    }

    pub async fn schedule_workflow(
        &self,
        wf: &niupanel_entity::tg_workflows::Model,
    ) -> Result<(), AppError> {
        let _guard = self.workflow_schedule_lock.lock().await;
        self.replace_workflow_schedule(wf).await
    }

    async fn replace_workflow_schedule(
        &self,
        wf: &niupanel_entity::tg_workflows::Model,
    ) -> Result<(), AppError> {
        if wf.event_type != "cron" {
            return self.remove_workflow_schedule_locked(wf.id).await;
        }

        let wf_id = wf.id;
        let config: serde_json::Value = serde_json::from_str(&wf.config_json).map_err(|err| {
            AppError::ValidationError(format!("工作流 {} 的配置不是有效 JSON: {}", wf_id, err))
        })?;
        let cron = match config.get("cron").and_then(|v| v.as_str()) {
            Some(c) if !c.trim().is_empty() => Self::normalize_cron(c),
            _ => {
                return Err(AppError::ValidationError(format!(
                    "工作流 {} 缺少有效的 cron 表达式",
                    wf_id
                )));
            }
        };

        let tz = self.get_system_timezone().await;
        let eb = self.event_bus.clone();

        let job = Job::new_async_tz(cron.as_str(), tz, move |_uuid, _l| {
            let eb = eb.clone();
            let wf_id_clone = wf_id;
            Box::pin(async move {
                info!("执行定时工作流: {}", wf_id_clone);
                eb.publish(SystemEvent::Telegram(TelegramEvent::WorkflowTriggered {
                    workflow_id: wf_id_clone,
                    task_id_context: None,
                }));
            })
        })
        .map_err(|e| {
            AppError::Generic(format!(
                "无效的Cron表达式 '{}'，工作流 {}: {}",
                cron, wf_id, e
            ))
        })?;

        let job_uuid = self
            .scheduler
            .add(job)
            .await
            .map_err(|e| AppError::Generic(format!("工作流 {} 调度失败: {}", wf_id, e)))?;

        let old_job_uuid = self.scheduled_jobs.get(&-wf_id).map(|entry| *entry.value());
        if let Some(old_job_uuid) = old_job_uuid
            && let Err(remove_err) = self.scheduler.remove(&old_job_uuid).await
        {
            let cleanup_result = self.scheduler.remove(&job_uuid).await;
            let cleanup_message = cleanup_result
                .err()
                .map(|err| format!("；新调度清理也失败: {}", err))
                .unwrap_or_default();
            return Err(AppError::Generic(format!(
                "替换工作流 {} 的旧调度失败: {}{}",
                wf_id, remove_err, cleanup_message
            )));
        }

        self.scheduled_jobs.insert(-wf_id, job_uuid);
        info!("已调度工作流 {}，Cron: {} (时区: {:?})", wf_id, cron, tz);
        Ok(())
    }

    pub async fn update_workflow_schedule(
        &self,
        wf: &niupanel_entity::tg_workflows::Model,
    ) -> Result<(), AppError> {
        self.schedule_workflow(wf).await
    }

    pub async fn remove_workflow_schedule(&self, wf_id: i32) -> Result<(), AppError> {
        let _guard = self.workflow_schedule_lock.lock().await;
        self.remove_workflow_schedule_locked(wf_id).await
    }

    async fn remove_workflow_schedule_locked(&self, wf_id: i32) -> Result<(), AppError> {
        let job_uuid = self.scheduled_jobs.get(&-wf_id).map(|entry| *entry.value());
        if let Some(job_uuid) = job_uuid {
            self.scheduler
                .remove(&job_uuid)
                .await
                .map_err(|e| AppError::Generic(format!("移除工作流 {} 失败: {}", wf_id, e)))?;
            self.scheduled_jobs.remove(&-wf_id);
            info!("已移除工作流 {} 的计划", wf_id);
        }
        Ok(())
    }

    fn normalize_cron(cron_expression: &str) -> String {
        let parts: Vec<&str> = cron_expression.split_whitespace().collect();
        if parts.len() == 5 {
            format!("0 {}", cron_expression)
        } else {
            cron_expression.to_string()
        }
    }

    pub(crate) async fn calculate_next_run(
        &self,
        cron_schedule: &Option<String>,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        if let Some(cron_str) = cron_schedule {
            if !cron_str.trim().is_empty() {
                let normalized = Self::normalize_cron(cron_str);
                let tz = self.get_system_timezone().await;
                if let Ok(schedule) = normalized.parse::<cron::Schedule>() {
                    return schedule
                        .upcoming(tz)
                        .next()
                        .map(|t| t.with_timezone(&chrono::Utc));
                }
            }
        }
        None
    }
}
