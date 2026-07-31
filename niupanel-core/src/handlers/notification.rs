use crate::event_bus::{EventBus, SystemEvent, SystemNotification, TaskEvent};
use crate::settings::SettingsManager;
use niupanel_common::constants::settings::*;
use niupanel_common::{info, warn};
use niupanel_entity::task_status::TaskStatus;
use niupanel_entity::tasks;
use sea_orm::{DatabaseConnection, EntityTrait};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct NotificationHandler {
    event_bus: EventBus,
    db: DatabaseConnection,
    http_client: reqwest::Client,
    settings_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl NotificationHandler {
    pub fn new(event_bus: EventBus, db: DatabaseConnection, http_client: reqwest::Client) -> Self {
        Self {
            event_bus,
            db,
            http_client,
            settings_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(self) {
        // Initial load
        self.refresh_settings().await;

        let mut rx = self.event_bus.subscribe();
        loop {
            match rx.recv().await {
                Ok(event) => match event {
                    SystemEvent::Task(TaskEvent::StatusChanged {
                        task_id,
                        status,
                        is_system,
                        job_id: _,
                        run_id: _,
                        output,
                        ..
                    }) => {
                        if !is_system {
                            let handler = self.clone();
                            let status = status.clone();
                            tokio::spawn(async move {
                                handler
                                    .handle_task_notification(task_id, status, output)
                                    .await;
                            });
                        }
                    }
                    SystemEvent::System(SystemNotification::Notification {
                        title,
                        content,
                        level: _,
                    }) => {
                        let handler = self.clone();
                        tokio::spawn(async move {
                            handler.send_notification(&title, &content).await;
                        });
                    }
                    SystemEvent::System(SystemNotification::SettingChanged { key, value }) => {
                        if self.is_relevant_setting(&key) {
                            let mut cache = self.settings_cache.write().await;
                            cache.insert(key, value);
                            info!("NotificationHandler updated setting cache");
                        }
                    }
                    SystemEvent::System(SystemNotification::Alert { message }) => {
                        let handler = self.clone();
                        tokio::spawn(async move {
                            handler
                                .send_notification("NiuPanel 启动自检告警", &message)
                                .await;
                        });
                    }
                    _ => {}
                },
                Err(e) => match e {
                    tokio::sync::broadcast::error::RecvError::Closed => {
                        info!("NotificationHandler channel closed, exiting.");
                        break;
                    }
                    tokio::sync::broadcast::error::RecvError::Lagged(skipped) => {
                        warn!("NotificationHandler lagged, skipped {} events", skipped);
                    }
                },
            }
        }
    }

    fn is_relevant_setting(&self, key: &str) -> bool {
        matches!(
            key,
            NOTIFY_WEBHOOK_URL
                | NOTIFY_EVENTS
                | MAIL_HOST
                | MAIL_TO
                | MAIL_USERNAME
                | MAIL_PASSWORD
        )
    }

    async fn refresh_settings(&self) {
        let keys = vec![
            NOTIFY_WEBHOOK_URL,
            NOTIFY_EVENTS,
            MAIL_HOST,
            MAIL_TO,
            MAIL_USERNAME,
            MAIL_PASSWORD,
        ];

        match SettingsManager::get_many(&self.db, &keys).await {
            Ok(map) => {
                let mut cache = self.settings_cache.write().await;
                *cache = map;
                info!("通知设置已加载");
            }
            Err(e) => {
                warn!("加载通知设置失败: {}", e);
            }
        }
    }

    async fn get_setting(&self, key: &str) -> Option<String> {
        let cache = self.settings_cache.read().await;
        cache.get(key).cloned()
    }

    async fn send_notification(&self, title: &str, content: &str) {
        self.perform_send(title, content).await;
    }

    async fn perform_send(&self, title: &str, content: &str) {
        // Webhook
        if let Some(webhook_url) = self.get_setting(NOTIFY_WEBHOOK_URL).await {
            if !webhook_url.trim().is_empty() {
                let _ = crate::notification::service::NotificationService::send_webhook(
                    &self.http_client,
                    &webhook_url,
                    title,
                    content,
                )
                .await;
            }
        }

        // Email
        if let (Some(host), Some(to)) = (
            self.get_setting(MAIL_HOST).await,
            self.get_setting(MAIL_TO).await,
        ) {
            if !host.trim().is_empty() && !to.trim().is_empty() {
                let user = self.get_setting(MAIL_USERNAME).await.unwrap_or_default();
                let pass = self.get_setting(MAIL_PASSWORD).await.unwrap_or_default();
                //  info!("Sending email to {}", pass);
                let _ = crate::notification::service::NotificationService::send_email(
                    &host, &user, &pass, &to, title, content,
                )
                .await;
            }
        }
    }

    async fn handle_task_notification(
        &self,
        task_id: i32,
        status: TaskStatus,
        output: Option<String>,
    ) {
        let events = self.get_setting(NOTIFY_EVENTS).await.unwrap_or_default();
        let notify_on_fail = events.contains("failed");
        let notify_on_success = events.contains("success");

        // Check Task Setting
        let task = match tasks::Entity::find_by_id(task_id).one(&self.db).await {
            Ok(Some(t)) => t,
            _ => return,
        };

        if !task.notify {
            return;
        }

        // Determine if we should send
        let should_send = match status {
            TaskStatus::Failed => notify_on_fail,
            TaskStatus::Finished => notify_on_success,
            _ => false,
        };

        if should_send {
            let title = format!("Task {:?} {}", status, task.name);
            let mut content = format!(
                "Task: {}\nStatus: {:?}\nTime: {}\n",
                task.name,
                status,
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            );

            if let Some(log) = output {
                content.push_str(&log);
            }

            self.perform_send(&title, &content).await;
        }
    }
}
