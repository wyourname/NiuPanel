use crate::sys::actor::{ProcessActor, ProcessMsg, ProcessStartupArgs};
use crate::sys::status_manager::StatusManager;
use crate::task_log::LogSession;
use dashmap::DashMap;
use niupanel_common::error::{AppError, Result};
use niupanel_common::info;
use ractor::{Actor, ActorRef};
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::OwnedSemaphorePermit;
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct ProcessManager {
    processes: Arc<DashMap<i32, (Option<i32>, ActorRef<ProcessMsg>)>>,
    status_manager: StatusManager,
}

impl ProcessManager {
    pub fn new(status_manager: StatusManager) -> Self {
        Self {
            processes: Arc::new(DashMap::new()),
            status_manager,
        }
    }

    pub async fn insert(
        &self,
        task_id: i32,
        child: Child,
        permit: OwnedSemaphorePermit,
        run_id: Option<i32>,
        cpu_cgroup: Option<std::path::PathBuf>,
        output_drain: Option<JoinHandle<()>>,
        timeout_sec: Option<i32>,
        log_session: Option<LogSession>,
    ) -> Result<()> {
        let cleanup_cgroup = cpu_cgroup.clone();
        let args = ProcessStartupArgs {
            task_id,
            run_id,
            child,
            status_manager: self.status_manager.clone(),
            permit,
            cpu_cgroup,
            output_drain,
            timeout_sec,
            log_session,
        };

        // Spawn the actor
        match Actor::spawn(None, ProcessActor, args).await {
            Ok((actor_ref, join_handle)) => {
                // 1. Insert into map
                self.processes.insert(task_id, (run_id, actor_ref));

                // 2. Spawn cleaner to remove from map when actor dies
                let processes_map = self.processes.clone();
                tokio::spawn(async move {
                    let _ = join_handle.await;
                    // Only remove if run_id matches (avoid race condition with restart)
                    processes_map
                        .remove_if(&task_id, |_, (stored_run_id, _)| *stored_run_id == run_id);
                    info!("Actor 任务 {} 已经完成 移除调度", task_id);
                });
                Ok(())
            }
            Err(e) => {
                if let Some(path) = &cleanup_cgroup {
                    crate::sys::sandbox::cleanup_post_spawn_limits(path);
                }
                niupanel_common::error!("Failed to spawn actor for Task {}: {}", task_id, e);
                Err(AppError::Generic(format!("启动任务进程管理器失败: {}", e)))
            }
        }
    }

    pub fn is_running(&self, task_id: i32) -> bool {
        self.processes.contains_key(&task_id)
    }

    pub async fn stop(&self, task_id: i32, run_id: Option<i32>) -> Result<()> {
        info!("接收到停止任务id {}", task_id);
        self.send_message(task_id, ProcessMsg::Stop(run_id)).await
    }

    pub async fn pause(&self, task_id: i32) -> Result<()> {
        info!("接收到暂停任务id {}", task_id);
        self.send_message(task_id, ProcessMsg::Pause).await
    }

    pub async fn resume(&self, task_id: i32) -> Result<()> {
        info!("接收到恢复任务id {}", task_id);
        self.send_message(task_id, ProcessMsg::Resume).await
    }

    async fn send_message(&self, task_id: i32, msg: ProcessMsg) -> Result<()> {
        if let Some(entry) = self.processes.get(&task_id) {
            let (_, actor_ref) = entry.value();
            if actor_ref.cast(msg).is_err() {
                // Actor is likely dead or closed
                drop(entry);
                self.processes.remove(&task_id);
                return Err(AppError::Generic(
                    "进程 actor 是不存在或者未知状态.".to_string(),
                ));
            }
            Ok(())
        } else {
            Err(AppError::NotFound("进程不存在.".to_string()))
        }
    }
}
