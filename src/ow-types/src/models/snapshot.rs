use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: Box<str>,
    pub name: Box<str>,
    pub sandbox_id: Box<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Box<str>>,
    pub created_at: Box<str>,
    pub size: Box<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_pointer: Option<Box<str>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSnapshotRequest {
    pub sandbox_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSnapshotRequest {
    pub sandbox_id: String,
    pub snapshot_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSnapshotRequest {
    pub sandbox_id: String,
    pub snapshot_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSnapshotRequest {
    pub sandbox_id: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    #[serde(default)]
    pub files_added: Vec<Box<str>>,
    #[serde(default)]
    pub files_modified: Vec<Box<str>>,
    #[serde(default)]
    pub files_deleted: Vec<Box<str>>,
    pub summary: Box<str>,
}
