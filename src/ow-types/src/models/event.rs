use serde::{Deserialize, Serialize};

pub mod event_types {
    pub const SANDBOX_STATUS_CHANGED: &str = "sandbox.status_changed";
    pub const SNAPSHOT_CREATED: &str = "snapshot.created";
    pub const SNAPSHOT_RESTORED: &str = "snapshot.restored";
    pub const GUARDIAN_VIOLATION: &str = "guardian.violation";
    pub const GUARDIAN_APPROVAL_REQUIRED: &str = "guardian.approval_required";
    pub const GUARDIAN_ALERT: &str = "guardian.alert";
    pub const AUDIT_COMMAND: &str = "audit.command";
    pub const RESOURCE_THRESHOLD: &str = "resource.threshold";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: Box<str>,
    pub timestamp: Box<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_id: Option<Box<str>>,
    pub data: sonic_rs::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventSubscribeRequest {
    #[serde(default)]
    pub sandbox_id: Option<String>,
    #[serde(default)]
    pub event_types: Vec<String>,
}
