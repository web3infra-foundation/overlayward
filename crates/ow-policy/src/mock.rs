use async_trait::async_trait;
use ow_types::*;
use papaya::HashMap;
use std::sync::Arc;

// === Store ===

pub struct PolicyStore {
    pub approvals: HashMap<String, Approval>,
}

impl PolicyStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            approvals: HashMap::new(),
        })
    }
}

// === Guardian ===

#[async_trait]
pub trait Guardian: Send + Sync + 'static {
    async fn check(
        &self,
        operation: &str,
        params: &sonic_rs::Value,
        caller: &CallerIdentity,
    ) -> Result<GuardianVerdict, ApiError>;
}

#[derive(Debug, Clone, Copy)]
pub enum GuardianVerdict {
    Allow,
}

pub struct MockGuardian;

static OP_ACCESS_MAP: phf::Map<&'static str, AccessLevel> = phf::phf_map! {
    "sandbox.create"      => AccessLevel::Agent,
    "sandbox.start"       => AccessLevel::Agent,
    "sandbox.pause"       => AccessLevel::Agent,
    "sandbox.resume"      => AccessLevel::Agent,
    "sandbox.stop"        => AccessLevel::Agent,
    "sandbox.destroy"     => AccessLevel::Agent,
    "sandbox.list"        => AccessLevel::Agent,
    "sandbox.info"        => AccessLevel::Agent,
    "snapshot.save"       => AccessLevel::Agent,
    "snapshot.restore"    => AccessLevel::Agent,
    "snapshot.list"       => AccessLevel::Agent,
    "snapshot.delete"     => AccessLevel::User,
    "snapshot.diff"       => AccessLevel::Agent,
    "network.get"         => AccessLevel::Agent,
    "network.allow"       => AccessLevel::Agent,
    "network.deny"        => AccessLevel::User,
    "network.set_default" => AccessLevel::Admin,
    "exec.run"            => AccessLevel::Agent,
    "exec.shell"          => AccessLevel::Agent,
    "file.read"           => AccessLevel::Agent,
    "file.write"          => AccessLevel::Agent,
    "file.list"           => AccessLevel::Agent,
    "file.upload"         => AccessLevel::User,
    "file.download"       => AccessLevel::User,
    "volume.mount"        => AccessLevel::User,
    "volume.unmount"      => AccessLevel::User,
    "volume.list"         => AccessLevel::Agent,
    "audit.query"         => AccessLevel::User,
    "audit.detail"        => AccessLevel::User,
    "audit.replay"        => AccessLevel::User,
    "resource.usage"      => AccessLevel::Agent,
    "resource.resize"     => AccessLevel::User,
    "inter.connect"       => AccessLevel::User,
    "inter.send"          => AccessLevel::Agent,
    "inter.disconnect"    => AccessLevel::User,
    "approval.list"       => AccessLevel::Human,
    "approval.decide"     => AccessLevel::Human,
    "events.subscribe"    => AccessLevel::User,
};

#[async_trait]
impl Guardian for MockGuardian {
    #[inline]
    async fn check(
        &self,
        operation: &str,
        _params: &sonic_rs::Value,
        caller: &CallerIdentity,
    ) -> Result<GuardianVerdict, ApiError> {
        let required = OP_ACCESS_MAP
            .get(operation)
            .copied()
            .unwrap_or(AccessLevel::Admin);

        if caller.can(required) {
            Ok(GuardianVerdict::Allow)
        } else {
            Err(ApiError::permission_denied(format!(
                "operation '{operation}' requires {required_level} access, caller has {caller_level}",
                required_level = required.as_str(),
                caller_level = caller.access_level.as_str(),
            )))
        }
    }
}

// === ApprovalManager ===

#[async_trait]
pub trait ApprovalManager: Send + Sync + 'static {
    async fn list(&self, filter: ApprovalListFilter) -> Result<Vec<Approval>, ApiError>;
    async fn decide(&self, decision: ApprovalDecision) -> Result<(), ApiError>;
}

pub struct PolicyBackend {
    store: Arc<PolicyStore>,
}

impl PolicyBackend {
    pub fn new(store: Arc<PolicyStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl ApprovalManager for PolicyBackend {
    async fn list(&self, filter: ApprovalListFilter) -> Result<Vec<Approval>, ApiError> {
        let guard = self.store.approvals.pin();
        Ok(guard
            .iter()
            .map(|(_k, v)| v.clone())
            .filter(|a| filter.status.as_ref().map_or(true, |s| a.status.as_str() == s.as_str()))
            .collect())
    }

    async fn decide(&self, decision: ApprovalDecision) -> Result<(), ApiError> {
        let guard = self.store.approvals.pin();
        let mut approval = guard
            .get(&decision.approval_id)
            .cloned()
            .ok_or_else(|| ApiError::not_found("approval", &decision.approval_id))?;
        approval.status = match decision.decision.as_str() {
            "approve" => ApprovalStatus::Approved,
            "deny" => ApprovalStatus::Denied,
            _ => return Err(ApiError::invalid_argument("decision must be 'approve' or 'deny'")),
        };
        approval.decision_reason = decision.reason.map(Into::into);
        guard.insert(decision.approval_id, approval);
        Ok(())
    }
}
