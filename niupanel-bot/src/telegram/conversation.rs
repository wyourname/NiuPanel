use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use teloxide::types::MessageId;
use tokio::sync::RwLock;

const CONVERSATION_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AddTaskData {
    pub name: Option<String>,
    pub script_content: Option<String>,
    pub file_name: Option<String>,
    pub env_type: Option<String>,
    pub env_version: Option<String>,
    pub cron_schedule: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AddTaskStep {
    WaitingName,
    WaitingScript,
    WaitingEnvType,
    WaitingPythonVersion,
    WaitingCron,
    Confirming,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AddVarData {
    pub key: Option<String>,
    pub value: Option<String>,
    pub scope: Option<String>,
    pub selected_tasks: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AddVarStep {
    WaitingKey,
    WaitingValue,
    WaitingScope,
    WaitingScriptId,
    Confirming,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditVarStep {
    SelectingField,
    WaitingValue { field: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditTaskStep {
    SelectingField,
    WaitingValue { field: String },
}

#[derive(Debug, Clone)]
pub enum ConversationState {
    Idle,
    AddTask {
        step: AddTaskStep,
        data: AddTaskData,
        last_msg_id: Option<MessageId>,
        created_at: Instant,
    },
    AddVar {
        step: AddVarStep,
        data: AddVarData,
        last_msg_id: Option<MessageId>,
        created_at: Instant,
    },
    EditTask {
        task_id: i32,
        step: EditTaskStep,
        last_msg_id: Option<MessageId>,
        created_at: Instant,
    },
    EditVar {
        var_id: i32,
        step: EditVarStep,
        last_msg_id: Option<MessageId>,
        selected_tasks: Vec<i32>,
        created_at: Instant,
    },
}

impl ConversationState {
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    pub fn is_timed_out(&self) -> bool {
        match self {
            Self::Idle => false,
            Self::AddTask { created_at, .. }
            | Self::AddVar { created_at, .. }
            | Self::EditTask { created_at, .. }
            | Self::EditVar { created_at, .. } => created_at.elapsed() > CONVERSATION_TIMEOUT,
        }
    }
}

impl Default for ConversationState {
    fn default() -> Self {
        Self::Idle
    }
}

pub type ConversationStore = Arc<RwLock<HashMap<String, ConversationState>>>;

pub fn new_conversation_store() -> ConversationStore {
    Arc::new(RwLock::new(HashMap::new()))
}

pub fn now() -> Instant {
    Instant::now()
}
