use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

impl ApprovalStatus {
    #[inline(always)]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Expired => "expired",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    pub approval_id: Box<str>,
    pub requester: Box<str>,
    pub sandbox_id: Box<str>,
    pub operation: Box<str>,
    pub status: ApprovalStatus,
    pub created_at: Box<str>,
    pub timeout: Box<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Box<str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_reason: Option<Box<str>>,
    pub detail: sonic_rs::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalListFilter {
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    #[serde(default)]
    pub approval_id: String,
    pub decision: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub permanent: bool,
}
