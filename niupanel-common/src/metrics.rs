use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub memory_total: u64,
    pub memory_used: u64,
    /// Bytes sent per second across non-loopback interfaces visible to the panel.
    #[serde(default)]
    pub network_upload_speed: Option<f64>,
    /// Bytes received per second; unavailable until two samples have been collected.
    #[serde(default)]
    pub network_download_speed: Option<f64>,
    pub uptime: u64,
    pub os_info: String,
}
