use async_trait::async_trait;
use ow_types::*;
use papaya::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;

// === Store ===

pub struct DataStore {
    pub volumes: HashMap<String, Vec<Volume>>,
    pub connections: RwLock<Vec<InterConnection>>,
    pub network_policies: HashMap<String, NetworkPolicy>,
    pub approvals: HashMap<String, Approval>,
}

impl DataStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            volumes: HashMap::new(),
            connections: RwLock::new(Vec::new()),
            network_policies: HashMap::new(),
            approvals: HashMap::new(),
        })
    }
}

pub struct DataBackend {
    store: Arc<DataStore>,
}

impl DataBackend {
    pub fn new(store: Arc<DataStore>) -> Self {
        Self { store }
    }
}

#[inline(always)]
fn gen_id(prefix: &str) -> Box<str> {
    let u = uuid::Uuid::now_v7();
    format!("{prefix}-{}", &u.simple().to_string()[..8]).into()
}

#[inline(always)]
fn now_rfc3339() -> Box<str> {
    jiff::Timestamp::now().to_string().into_boxed_str()
}

// === VolumeManager ===

#[async_trait]
pub trait VolumeManager: Send + Sync + 'static {
    async fn mount(&self, req: VolumeMountRequest) -> Result<(), ApiError>;
    async fn unmount(&self, sandbox_id: &str, guest_path: &str) -> Result<(), ApiError>;
    async fn list(&self, sandbox_id: &str) -> Result<Vec<Volume>, ApiError>;
}

#[async_trait]
impl VolumeManager for DataBackend {
    async fn mount(&self, req: VolumeMountRequest) -> Result<(), ApiError> {
        let vol = Volume {
            host_path: req.host_path.into(),
            guest_path: req.guest_path.into(),
            mode: req.mode,
        };
        let guard = self.store.volumes.pin();
        let mut vols = guard.get(&req.sandbox_id).cloned().unwrap_or_default();
        vols.push(vol);
        guard.insert(req.sandbox_id, vols);
        Ok(())
    }

    async fn unmount(&self, sandbox_id: &str, guest_path: &str) -> Result<(), ApiError> {
        let guard = self.store.volumes.pin();
        let mut vols = guard.get(sandbox_id).cloned().unwrap_or_default();
        vols.retain(|v| &*v.guest_path != guest_path);
        guard.insert(sandbox_id.to_string(), vols);
        Ok(())
    }

    async fn list(&self, sandbox_id: &str) -> Result<Vec<Volume>, ApiError> {
        Ok(self.store.volumes.pin().get(sandbox_id).cloned().unwrap_or_default())
    }
}

// === InterManager ===

#[async_trait]
pub trait InterManager: Send + Sync + 'static {
    async fn connect(&self, req: InterConnectRequest) -> Result<(), ApiError>;
    async fn send(&self, msg: InterMessage) -> Result<(), ApiError>;
    async fn disconnect(&self, sandbox_a: &str, sandbox_b: &str) -> Result<(), ApiError>;
}

#[async_trait]
impl InterManager for DataBackend {
    async fn connect(&self, req: InterConnectRequest) -> Result<(), ApiError> {
        let conn = InterConnection {
            sandbox_a: req.sandbox_a.into(),
            sandbox_b: req.sandbox_b.into(),
            mode: req.mode,
            bidirectional: req.bidirectional,
        };
        self.store.connections.write().push(conn);
        Ok(())
    }

    async fn send(&self, msg: InterMessage) -> Result<(), ApiError> {
        let conns = self.store.connections.read();
        let has_conn = conns.iter().any(|c| {
            (&*c.sandbox_a == msg.from_sandbox && &*c.sandbox_b == msg.to_sandbox)
                || (c.bidirectional && &*c.sandbox_b == msg.from_sandbox && &*c.sandbox_a == msg.to_sandbox)
        });
        if !has_conn {
            return Err(ApiError::not_found("connection", &format!("{}->{}", msg.from_sandbox, msg.to_sandbox)));
        }
        Ok(())
    }

    async fn disconnect(&self, sandbox_a: &str, sandbox_b: &str) -> Result<(), ApiError> {
        let mut conns = self.store.connections.write();
        conns.retain(|c| !(&*c.sandbox_a == sandbox_a && &*c.sandbox_b == sandbox_b));
        Ok(())
    }
}

// === NetworkManager ===

#[async_trait]
pub trait NetworkManager: Send + Sync + 'static {
    async fn get(&self, sandbox_id: &str) -> Result<NetworkPolicy, ApiError>;
    async fn allow(&self, req: AddNetworkRuleRequest) -> Result<AddRuleResult, ApiError>;
    async fn deny(&self, sandbox_id: &str, rule_id: &str) -> Result<(), ApiError>;
    async fn set_default(&self, sandbox_id: &str, default: &str) -> Result<(), ApiError>;
}

#[async_trait]
impl NetworkManager for DataBackend {
    async fn get(&self, sandbox_id: &str) -> Result<NetworkPolicy, ApiError> {
        Ok(self.store.network_policies.pin().get(sandbox_id).cloned().unwrap_or_default())
    }

    async fn allow(&self, req: AddNetworkRuleRequest) -> Result<AddRuleResult, ApiError> {
        if let Some(ref cidr) = req.cidr {
            if cidr.starts_with("10.") || cidr.starts_with("192.168.") || cidr.starts_with("172.") {
                return Err(ApiError::new(
                    codes::GUARDIAN_NETWORK_VIOLATION,
                    "target address belongs to host network segment",
                ));
            }
        }
        static WHITELIST: &[&str] = &[
            "api.github.com", "*.npmjs.org", "*.crates.io",
            "registry.npmjs.org", "pypi.org", "*.pypi.org",
        ];
        let is_whitelisted = req.domain.as_ref().map_or(false, |d| {
            WHITELIST.iter().any(|w| {
                if let Some(suffix) = w.strip_prefix("*.") {
                    d.ends_with(suffix)
                } else {
                    d.as_str() == *w
                }
            })
        });
        if !is_whitelisted && req.domain.is_some() {
            let approval_id = gen_id("apr");
            let guard = self.store.approvals.pin();
            guard.insert(
                approval_id.to_string(),
                Approval {
                    approval_id: approval_id.clone(),
                    requester: "caller".into(),
                    sandbox_id: req.sandbox_id.clone().into(),
                    operation: "network.allow".into(),
                    status: ApprovalStatus::Pending,
                    created_at: now_rfc3339(),
                    timeout: "30m".into(),
                    reason: req.reason.map(Into::into),
                    decision_reason: None,
                    detail: sonic_rs::json!({ "domain": req.domain }),
                },
            );
            return Ok(AddRuleResult::ApprovalRequired {
                approval_id,
                status: "pending".into(),
                timeout: "30m".into(),
            });
        }
        let rule_id = gen_id("rule");
        let rule = NetworkRule {
            rule_id: rule_id.clone(),
            domain: req.domain.map(Into::into),
            cidr: req.cidr.map(Into::into),
            ports: req.ports,
            protocol: req.protocol,
        };
        let guard = self.store.network_policies.pin();
        let mut policy = guard.get(&req.sandbox_id).cloned().unwrap_or_default();
        policy.rules.push(rule);
        guard.insert(req.sandbox_id, policy);
        Ok(AddRuleResult::Allowed { rule_id })
    }

    async fn deny(&self, sandbox_id: &str, rule_id: &str) -> Result<(), ApiError> {
        let guard = self.store.network_policies.pin();
        let mut policy = guard.get(sandbox_id).cloned().unwrap_or_default();
        let before = policy.rules.len();
        policy.rules.retain(|r| &*r.rule_id != rule_id);
        if policy.rules.len() == before {
            return Err(ApiError::not_found("rule", rule_id));
        }
        guard.insert(sandbox_id.to_string(), policy);
        Ok(())
    }

    async fn set_default(&self, sandbox_id: &str, default: &str) -> Result<(), ApiError> {
        let guard = self.store.network_policies.pin();
        let mut policy = guard.get(sandbox_id).cloned().unwrap_or_default();
        policy.default_action = default.into();
        guard.insert(sandbox_id.to_string(), policy);
        Ok(())
    }
}
