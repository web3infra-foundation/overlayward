use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub host_path: Box<str>,
    pub guest_path: Box<str>,
    #[serde(default = "default_ro")]
    pub mode: Box<str>,
}

#[inline(always)]
fn default_ro() -> Box<str> { "ro".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMountRequest {
    #[serde(default)]
    pub sandbox_id: String,
    pub host_path: String,
    pub guest_path: String,
    #[serde(default = "default_ro")]
    pub mode: Box<str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeUnmountRequest {
    pub sandbox_id: String,
    pub guest_path: String,
}
