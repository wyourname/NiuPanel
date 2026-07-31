use niupanel_entity::task_status::TaskStatus;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaskEvent {
    StatusChanged {
        task_id: i32,
        job_id: Option<i32>,
        run_id: Option<i32>,
        status: TaskStatus,
        is_system: bool,
        output: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cpu_usage: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        memory_usage: Option<u64>,
    },
    Log {
        task_id: i32,
        content: std::sync::Arc<String>,
        is_system: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SystemNotification {
    Notification {
        title: String,
        content: String,
        level: String, // info, error, etc.
    },
    SettingChanged {
        key: String,
        value: String,
    },
    Alert {
        message: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthEvent {
    LoginApprovalRequest {
        ticket: String,
        username: String,
        ip: String,
        timestamp: u64,
    },
    LoginApprovalResponse {
        ticket: String,
        approved: bool,
    },
    LoginOtpRequest {
        username: String,
        ip: String,
        code: String,
        timestamp: u64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TelegramEvent {
    MessageReceived {
        chat_id: String,
        username: Option<String>,
        text: Option<String>,
        file_name: Option<String>,
        file_id: Option<String>,
        timestamp: u64,
    },
    WorkflowTriggered {
        workflow_id: i32,
        task_id_context: Option<i32>,
    },
    MessageSent {
        chat_id: String,
        text: Option<String>,
        file_name: Option<String>,
        timestamp: u64,
    },
    PackagePreviewRequest {
        chat_id: String,
        url: String,
        staging_id: String,
    },
    PackageImportRequest {
        staging_id: String,
        user_id: i32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SystemEvent {
    Task(TaskEvent),
    System(SystemNotification),
    Telegram(TelegramEvent),
    Auth(AuthEvent),
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(10000);
        Self { tx }
    }

    pub fn publish(&self, event: SystemEvent) {
        // We ignore errors if there are no subscribers
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.tx.subscribe()
    }
}
