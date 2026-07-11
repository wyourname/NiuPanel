use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub memory_total: u64,
    pub memory_used: u64,
    pub uptime: u64,
    pub os_info: String,
}
