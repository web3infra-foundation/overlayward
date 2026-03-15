use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditLevel {
    Command,
    Syscall,
    Filesystem,
}

impl AuditLevel {
    #[inline(always)]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Syscall => "syscall",
            Self::Filesystem => "filesystem",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Box<str>,
    pub timestamp: Box<str>,
    pub level: AuditLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<Box<str>>,
    pub content: sonic_rs::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditQuery {
    #[serde(default)]
    pub sandbox_id: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default = "default_audit_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

#[inline(always)]
const fn default_audit_limit() -> u32 { 100 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueryResult {
    pub events: Vec<AuditEvent>,
    pub total: u64,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReplayRequest {
    #[serde(default)]
    pub sandbox_id: String,
    pub from: String,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default = "default_speed")]
    pub speed: f64,
}

#[inline(always)]
fn default_speed() -> f64 { 1.0 }
