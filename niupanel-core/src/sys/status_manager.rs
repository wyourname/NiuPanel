use niupanel_entity::task_status::TaskStatus;
use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub struct TaskStatusEvent {
    pub task_id: i32,
    pub status: TaskStatus,
    pub pid: Option<i32>,
    pub run_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_usage: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_usage: Option<u64>,
}

#[derive(Clone)]
pub struct StatusManager {
    tx: broadcast::Sender<TaskStatusEvent>,
}

impl StatusManager {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100); // Buffer size 100
        Self { tx }
    }

    pub fn emit(&self, event: TaskStatusEvent) {
        // Ignore error if no active listeners
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TaskStatusEvent> {
        self.tx.subscribe()
    }
}
