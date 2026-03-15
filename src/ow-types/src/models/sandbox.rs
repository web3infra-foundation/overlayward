use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxStatus {
    Created,
    Running,
    Paused,
    Stopped,
}

macro_rules! define_status_machine {
    ($($from:ident => [$($to:ident),+]);* $(;)?) => {
        impl SandboxStatus {
            #[inline(always)]
            pub fn can_transition_to(self, target: Self) -> bool {
                match (self, target) {
                    $($((Self::$from, Self::$to) => true,)+)*
                    _ => false,
                }
            }
        }
    };
}

define_status_machine! {
    Created => [Running];
    Running => [Paused, Stopped];
    Paused  => [Running, Stopped];
    Stopped => [Running];
}

impl SandboxStatus {
    #[inline(always)]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub vsock_cid: u32,
    pub api_endpoint: Box<str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    #[serde(default)]
    pub device: Option<Box<str>>,
    #[serde(default = "default_gpu_count")]
    pub count: u32,
}

#[inline(always)]
const fn default_gpu_count() -> u32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sandbox {
    pub sandbox_id: Box<str>,
    pub name: Box<str>,
    pub status: SandboxStatus,
    #[serde(default = "default_cpu")]
    pub cpu: u32,
    #[serde(default = "default_memory")]
    pub memory: Box<str>,
    #[serde(default = "default_disk")]
    pub disk: Box<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<Box<str>>,
    pub owner: Box<str>,
    pub created_at: Box<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uptime: Option<Box<str>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection: Option<ConnectionInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu: Option<GpuConfig>,
}

#[inline(always)]
const fn default_cpu() -> u32 { 2 }
#[inline(always)]
fn default_memory() -> Box<str> { "4GB".into() }
#[inline(always)]
fn default_disk() -> Box<str> { "20GB".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSandboxRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default = "default_cpu")]
    pub cpu: u32,
    #[serde(default = "default_memory")]
    pub memory: Box<str>,
    #[serde(default = "default_disk")]
    pub disk: Box<str>,
    #[serde(default = "default_image")]
    pub image: Box<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_policy: Option<crate::NetworkPolicyConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu: Option<GpuConfig>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

#[inline(always)]
fn default_image() -> Box<str> { "ubuntu:24.04".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestroyOptions {
    #[serde(default)]
    pub keep_snapshots: bool,
    #[serde(default = "ret_true")]
    pub keep_audit_logs: bool,
}

impl Default for DestroyOptions {
    #[inline]
    fn default() -> Self {
        Self { keep_snapshots: false, keep_audit_logs: true }
    }
}

#[inline(always)]
const fn ret_true() -> bool { true }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListFilter {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub labels: Option<HashMap<String, String>>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

#[inline(always)]
const fn default_limit() -> u32 { 20 }
