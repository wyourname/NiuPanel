use crate::sys::status_manager::{StatusManager, TaskStatusEvent};
use crate::task_log::{LogKind, LogSession};
use niupanel_common::{debug, error, info, warn};
use niupanel_entity::task_status::TaskStatus;
#[cfg(unix)]
use nix::sys::signal::{Signal, kill};
#[cfg(unix)]
use nix::unistd::Pid;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::process::Child;
use tokio::sync::OwnedSemaphorePermit;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub enum ProcessMsg {
    Stop(Option<i32>),
    Pause,
    Resume,
    Stats(f32, u64),
    Timeout {
        run_id: Option<i32>,
        timeout_sec: i32,
    },
    /// Internal message when the child process exits
    Exit(std::io::Result<std::process::ExitStatus>),
}

pub struct ProcessStartupArgs {
    pub task_id: i32,
    pub run_id: Option<i32>,
    pub child: Child,
    pub status_manager: StatusManager,
    pub permit: OwnedSemaphorePermit,
    pub cpu_cgroup: Option<std::path::PathBuf>,
    pub output_drain: Option<JoinHandle<()>>,
    pub timeout_sec: Option<i32>,
    pub log_session: Option<LogSession>,
}

pub struct ProcessState {
    pub task_id: i32,
    pub run_id: Option<i32>,
    pub pid: Option<u32>, // OS PID
    pub status_manager: StatusManager,
    // Held to limit concurrency; dropped when Actor stops
    pub _permit: OwnedSemaphorePermit,
    pub cpu_cgroup: Option<std::path::PathBuf>,
    pub output_drain: Option<JoinHandle<()>>,
    pub timeout_task: Option<JoinHandle<()>>,
    pub log_session: Option<LogSession>,
    pub status: TaskStatus,
    pub stopping: bool,
}

pub struct ProcessActor;

impl Actor for ProcessActor {
    type Msg = ProcessMsg;
    type State = ProcessState;
    type Arguments = ProcessStartupArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let mut args = args;
        let pid = args.child.id();
        info!("Actor 启动来源于任务： {} (PID: {:?})", args.task_id, pid);

        // Initial status emission
        args.status_manager.emit(TaskStatusEvent {
            task_id: args.task_id,
            status: TaskStatus::Running,
            pid: pid.map(|p| p as i32),
            run_id: args.run_id,
            cpu_usage: None,
            memory_usage: None,
        });

        let myself_exit = myself.clone();
        tokio::spawn(async move {
            let result = args.child.wait().await;
            // Send the exit result back to the actor
            let _ = myself_exit.cast(ProcessMsg::Exit(result));
        });

        let timeout_task = args
            .timeout_sec
            .filter(|timeout| *timeout > 0)
            .map(|timeout_sec| {
                let myself_timeout = myself.clone();
                let run_id = args.run_id;
                info!("Task {} timeout set to {}s", args.task_id, timeout_sec);
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(timeout_sec as u64)).await;
                    let _ = myself_timeout.cast(ProcessMsg::Timeout {
                        run_id,
                        timeout_sec,
                    });
                })
            });

        // Spawn stats monitor
        if let Some(pid_val) = pid {
            let myself_stats = myself.clone();
            tokio::spawn(async move {
                use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
                // Minimal refresh config
                let mut sys = System::new_with_specifics(
                    RefreshKind::nothing()
                        .with_processes(ProcessRefreshKind::nothing().with_cpu().with_memory()),
                );
                let sys_pid = Pid::from(pid_val as usize);

                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                    sys.refresh_processes(ProcessesToUpdate::Some(&[sys_pid]), true);

                    if let Some(process) = sys.process(sys_pid) {
                        let cpu = process.cpu_usage();
                        let mem = process.memory();
                        if myself_stats.cast(ProcessMsg::Stats(cpu, mem)).is_err() {
                            break;
                        }
                    } else {
                        // Process might have exited
                        break;
                    }
                }
            });
        }

        Ok(ProcessState {
            task_id: args.task_id,
            run_id: args.run_id,
            pid,
            status_manager: args.status_manager,
            _permit: args.permit,
            cpu_cgroup: args.cpu_cgroup,
            output_drain: args.output_drain,
            timeout_task,
            log_session: args.log_session,
            status: TaskStatus::Running,
            stopping: false,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ProcessMsg::Stop(req_run_id) => {
                if let Some(req_id) = req_run_id {
                    if state.run_id != Some(req_id) {
                        warn!(
                            "忽略停止任务 {}: Run ID 不匹配 (Current: {:?}, target: {:?})",
                            state.task_id, state.run_id, req_id
                        );
                        return Ok(());
                    }
                }

                debug!("停止 Task {}", state.task_id);
                self.abort_timeout_task(state);
                self.request_stop(state);
            }
            ProcessMsg::Timeout {
                run_id,
                timeout_sec,
            } => {
                if state.run_id != run_id || state.stopping {
                    return Ok(());
                }

                if let Some(log_session) = &state.log_session {
                    if let Err(e) = log_session
                        .write_line(
                            LogKind::System,
                            format!(
                                "[System] Task timed out after {}s, stopping...",
                                timeout_sec
                            ),
                        )
                        .await
                    {
                        error!("Failed to write timeout log: {}", e);
                    }
                }

                warn!(
                    "Task {} timed out, stop signal sent (Run ID: {:?}).",
                    state.task_id, state.run_id
                );
                self.request_stop(state);
            }
            ProcessMsg::Pause => {
                debug!("暂停任务 {}", state.task_id);
                #[cfg(unix)]
                {
                    if let Some(pid) = state.pid {
                        if let Err(e) = kill(Pid::from_raw(-(pid as i32)), Signal::SIGSTOP) {
                            error!("暂停任务 {}失败: {}", state.task_id, e);
                        } else {
                            state.status = TaskStatus::Paused;
                            self.emit_status(state, None);
                        }
                    }
                }
                #[cfg(not(unix))]
                warn!("Pause not supported on this OS");
            }
            ProcessMsg::Resume => {
                debug!("恢复 {}", state.task_id);
                #[cfg(unix)]
                {
                    if let Some(pid) = state.pid {
                        if let Err(e) = kill(Pid::from_raw(-(pid as i32)), Signal::SIGCONT) {
                            error!("恢复任务 {}成功: {}", state.task_id, e);
                        } else {
                            state.status = TaskStatus::Running;
                            self.emit_status(state, None);
                        }
                    }
                }
                #[cfg(not(unix))]
                warn!("Resume not supported on this OS");
            }
            ProcessMsg::Stats(cpu, mem) => {
                if !state.stopping {
                    self.emit_status(state, Some((cpu, mem)));
                }
            }
            ProcessMsg::Exit(wait_result) => {
                state.status = match wait_result {
                    Ok(status) if status.success() => {
                        info!("任务 {} 执行完成", state.task_id);
                        TaskStatus::Finished
                    }
                    Ok(status) => {
                        if state.stopping {
                            info!("停止请求来源于 Task {} ", state.task_id);
                            TaskStatus::Stopped
                        } else {
                            warn!("执行失败 Task {} code: {:?}", state.task_id, status.code());
                            TaskStatus::Failed
                        }
                    }
                    Err(e) => {
                        error!("Task {} wait error: {}", state.task_id, e);
                        TaskStatus::Failed
                    }
                };

                if let Some(path) = &state.cpu_cgroup {
                    crate::sys::sandbox::cleanup_post_spawn_limits(path);
                }

                self.abort_timeout_task(state);

                if let Some(output_drain) = state.output_drain.take() {
                    let _ = output_drain.await;
                }

                self.emit_status(state, None);
                _myself.stop(None);
            }
        }
        Ok(())
    }
}

impl ProcessActor {
    fn abort_timeout_task(&self, state: &mut ProcessState) {
        if let Some(timeout_task) = state.timeout_task.take() {
            timeout_task.abort();
        }
    }

    fn request_stop(&self, state: &mut ProcessState) {
        state.stopping = true;

        if let Some(pid) = state.pid {
            #[cfg(unix)]
            {
                let pgid = Pid::from_raw(-(pid as i32));

                if let Err(e) = kill(pgid, Signal::SIGTERM) {
                    error!("发送 SIGTERM 给任务 {} 失败: {}", state.task_id, e);
                }

                if state.status == TaskStatus::Paused {
                    let _ = kill(pgid, Signal::SIGCONT);
                }

                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    let _ = kill(pgid, Signal::SIGKILL);
                });
            }

            #[cfg(not(unix))]
            {
                warn!(
                    "Stop not fully implemented for non-Unix in this actor refactor without Child handle."
                );
            }
        }
    }

    fn emit_status(&self, state: &ProcessState, stats: Option<(f32, u64)>) {
        if stats.is_none() {
            info!("任务 {} 提交状态: {:?}", state.task_id, state.status);
        }

        let (cpu, mem) = match stats {
            Some((c, m)) => (Some(c), Some(m)),
            None => (None, None),
        };

        state.status_manager.emit(TaskStatusEvent {
            task_id: state.task_id,
            status: state.status, // Clone is implicit for Copy types, or we use reference
            pid: state.pid.map(|id| id as i32),
            run_id: state.run_id,
            cpu_usage: cpu,
            memory_usage: mem,
        });
    }
}
