use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu: CpuUsage,
    pub memory: MemoryUsage,
    pub disk: DiskUsage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu: Option<GpuUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuUsage {
    pub allocated: u32,
    pub usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub allocated: Box<str>,
    pub used: Box<str>,
    pub usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub allocated: Box<str>,
    pub used: Box<str>,
    pub usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuUsage {
    pub device: Box<str>,
    pub memory_used: Box<str>,
    pub utilization_percent: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceResizeRequest {
    #[serde(default)]
    pub sandbox_id: String,
    #[serde(default)]
    pub cpu: Option<u32>,
    #[serde(default)]
    pub memory: Option<String>,
    #[serde(default)]
    pub disk: Option<String>,
}
