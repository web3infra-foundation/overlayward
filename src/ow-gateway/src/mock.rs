use async_trait::async_trait;
use bytes::Bytes;
use ow_types::*;
use papaya::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::traits::*;
use crate::traits::{Guardian, GuardianVerdict};

pub struct InMemoryStore {
    pub sandboxes: HashMap<String, Sandbox>,
    pub snapshots: HashMap<String, Vec<Snapshot>>,
    pub network_policies: HashMap<String, NetworkPolicy>,
    pub audit_events: HashMap<String, Vec<AuditEvent>>,
    pub resource_usage: HashMap<String, ResourceUsage>,
    pub volumes: HashMap<String, Vec<Volume>>,
    pub approvals: HashMap<String, Approval>,
    pub connections: RwLock<Vec<InterConnection>>,
    pub files: HashMap<String, std::collections::HashMap<String, Vec<u8>>>,
    pub event_tx: broadcast::Sender<Event>,
}

impl InMemoryStore {
    pub fn new() -> Arc<Self> {
        let (event_tx, _) = broadcast::channel(256);
        Arc::new(Self {
            sandboxes: HashMap::new(),
            snapshots: HashMap::new(),
            network_policies: HashMap::new(),
            audit_events: HashMap::new(),
            resource_usage: HashMap::new(),
            volumes: HashMap::new(),
            approvals: HashMap::new(),
            connections: RwLock::new(Vec::new()),
            files: HashMap::new(),
            event_tx,
        })
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            sandboxes: HashMap::new(),
            snapshots: HashMap::new(),
            network_policies: HashMap::new(),
            audit_events: HashMap::new(),
            resource_usage: HashMap::new(),
            volumes: HashMap::new(),
            approvals: HashMap::new(),
            connections: RwLock::new(Vec::new()),
            files: HashMap::new(),
            event_tx,
        }
    }
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

pub struct MockBackend {
    store: Arc<InMemoryStore>,
}

impl MockBackend {
    pub fn new(store: Arc<InMemoryStore>) -> Self {
        Self { store }
    }
}

#[inline(always)]
fn gen_id(prefix: &str) -> Box<str> {
    let u = uuid::Uuid::now_v7();
    let s = u.simple().to_string();
    format!("{prefix}-{}{}", &s[..8], &s[24..32]).into()
}

#[inline(always)]
fn now_rfc3339() -> Box<str> {
    jiff::Timestamp::now().to_string().into_boxed_str()
}

macro_rules! get_sandbox_or_404 {
    ($store:expr, $id:expr) => {{
        let guard = $store.sandboxes.pin();
        guard.get($id).cloned().ok_or_else(|| ApiError::not_found("sandbox", $id))?
    }};
}

#[async_trait]
impl SandboxManager for MockBackend {
    async fn create(&self, req: CreateSandboxRequest) -> Result<Sandbox, ApiError> {
        let id = gen_id("sb");
        let name: Box<str> = req.name.map(Into::into).unwrap_or_else(|| id.clone());
        let sandbox = Sandbox {
            sandbox_id: id.clone(),
            name,
            status: SandboxStatus::Created,
            cpu: req.cpu,
            memory: req.memory,
            disk: req.disk,
            image: Some(req.image),
            owner: "caller".into(),
            created_at: now_rfc3339(),
            uptime: None,
            labels: req.labels,
            connection: Some(ConnectionInfo {
                vsock_cid: 3,
                api_endpoint: format!("unix:///var/run/overlayward/{id}.sock").into(),
            }),
            gpu: req.gpu,
        };
        let guard = self.store.sandboxes.pin();
        guard.insert(id.to_string(), sandbox.clone());
        if let Some(policy_cfg) = req.network_policy {
            let policy = NetworkPolicy {
                default_action: policy_cfg.default_action,
                rules: policy_cfg
                    .allow
                    .into_iter()
                    .map(|s| NetworkRule {
                        rule_id: gen_id("rule"),
                        domain: s.domain.map(Into::into),
                        cidr: s.cidr.map(Into::into),
                        ports: s.ports,
                        protocol: s.protocol,
                    })
                    .collect(),
            };
            self.store.network_policies.pin().insert(id.to_string(), policy);
        } else {
            self.store
                .network_policies
                .pin()
                .insert(id.to_string(), NetworkPolicy::default());
        }
        self.store.resource_usage.pin().insert(
            id.to_string(),
            ResourceUsage {
                cpu: CpuUsage {
                    allocated: sandbox.cpu,
                    usage_percent: 0.0,
                },
                memory: MemoryUsage {
                    allocated: sandbox.memory.clone(),
                    used: "0MB".into(),
                    usage_percent: 0.0,
                },
                disk: DiskUsage {
                    allocated: sandbox.disk.clone(),
                    used: "0MB".into(),
                    usage_percent: 0.0,
                },
                gpu: None,
            },
        );
        Ok(sandbox)
    }

    async fn start(&self, id: &str) -> Result<(), ApiError> {
        transition_status(&self.store, id, SandboxStatus::Running)
    }

    async fn pause(&self, id: &str) -> Result<(), ApiError> {
        transition_status(&self.store, id, SandboxStatus::Paused)
    }

    async fn resume(&self, id: &str) -> Result<(), ApiError> {
        transition_status(&self.store, id, SandboxStatus::Running)
    }

    async fn stop(&self, id: &str, _force: bool) -> Result<(), ApiError> {
        transition_status(&self.store, id, SandboxStatus::Stopped)
    }

    async fn destroy(&self, id: &str, _opts: DestroyOptions) -> Result<(), ApiError> {
        let guard = self.store.sandboxes.pin();
        guard
            .remove(id)
            .ok_or_else(|| ApiError::not_found("sandbox", id))?;
        self.store.network_policies.pin().remove(id);
        self.store.snapshots.pin().remove(id);
        self.store.audit_events.pin().remove(id);
        self.store.resource_usage.pin().remove(id);
        self.store.volumes.pin().remove(id);
        self.store.files.pin().remove(id);
        Ok(())
    }

    async fn list(&self, filter: ListFilter) -> Result<Vec<Sandbox>, ApiError> {
        let guard = self.store.sandboxes.pin();
        let mut result: Vec<Sandbox> = guard
            .iter()
            .map(|(_k, v)| v.clone())
            .filter(|s| {
                filter
                    .status
                    .as_ref()
                    .map_or(true, |st| s.status.as_str() == st.as_str())
            })
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let offset = filter.offset as usize;
        let limit = filter.limit as usize;
        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn info(&self, id: &str) -> Result<Sandbox, ApiError> {
        Ok(get_sandbox_or_404!(self.store, id))
    }
}

#[inline]
fn transition_status(store: &InMemoryStore, id: &str, target: SandboxStatus) -> Result<(), ApiError> {
    let guard = store.sandboxes.pin();
    let mut sb = guard
        .get(id)
        .cloned()
        .ok_or_else(|| ApiError::not_found("sandbox", id))?;
    if !sb.status.can_transition_to(target) {
        return Err(ApiError::invalid_transition(
            sb.status.as_str(),
            target.as_str(),
        ));
    }
    sb.status = target;
    guard.insert(id.to_string(), sb);
    Ok(())
}

#[async_trait]
impl SnapshotManager for MockBackend {
    async fn save(
        &self,
        sandbox_id: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<Snapshot, ApiError> {
        let _ = get_sandbox_or_404!(self.store, sandbox_id);
        let snap_id = gen_id("snap");
        let snap = Snapshot {
            snapshot_id: snap_id,
            name: name.unwrap_or("unnamed").into(),
            sandbox_id: sandbox_id.into(),
            description: description.map(Into::into),
            created_at: now_rfc3339(),
            size: "128MB".into(),
            audit_pointer: Some(gen_id("audit-evt")),
        };
        let guard = self.store.snapshots.pin();
        let mut snaps = guard.get(sandbox_id).cloned().unwrap_or_default();
        snaps.push(snap.clone());
        guard.insert(sandbox_id.to_string(), snaps);
        Ok(snap)
    }

    async fn restore(&self, sandbox_id: &str, snapshot_id: &str) -> Result<(), ApiError> {
        let guard = self.store.snapshots.pin();
        let snaps = guard.get(sandbox_id).cloned().unwrap_or_default();
        if !snaps.iter().any(|s| &*s.snapshot_id == snapshot_id) {
            return Err(ApiError::not_found("snapshot", snapshot_id));
        }
        Ok(())
    }

    async fn list(&self, sandbox_id: &str) -> Result<Vec<Snapshot>, ApiError> {
        let _ = get_sandbox_or_404!(self.store, sandbox_id);
        Ok(self
            .store
            .snapshots
            .pin()
            .get(sandbox_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn delete(&self, sandbox_id: &str, snapshot_id: &str) -> Result<(), ApiError> {
        let guard = self.store.snapshots.pin();
        let mut snaps = guard.get(sandbox_id).cloned().unwrap_or_default();
        let len_before = snaps.len();
        snaps.retain(|s| &*s.snapshot_id != snapshot_id);
        if snaps.len() == len_before {
            return Err(ApiError::not_found("snapshot", snapshot_id));
        }
        guard.insert(sandbox_id.to_string(), snaps);
        Ok(())
    }

    async fn diff(&self, sandbox_id: &str, _from: &str, _to: &str) -> Result<SnapshotDiff, ApiError> {
        let _ = get_sandbox_or_404!(self.store, sandbox_id);
        Ok(SnapshotDiff {
            files_added: vec!["src/new_file.rs".into()],
            files_modified: vec!["Cargo.toml".into()],
            files_deleted: vec![],
            summary: "+1 added, ~1 modified, -0 deleted".into(),
        })
    }
}

#[async_trait]
impl NetworkManager for MockBackend {
    async fn get(&self, sandbox_id: &str) -> Result<NetworkPolicy, ApiError> {
        let _ = get_sandbox_or_404!(self.store, sandbox_id);
        Ok(self
            .store
            .network_policies
            .pin()
            .get(sandbox_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn allow(&self, req: AddNetworkRuleRequest) -> Result<AddRuleResult, ApiError> {
        let _ = get_sandbox_or_404!(self.store, &req.sandbox_id);
        if let Some(ref cidr) = req.cidr {
            if cidr.starts_with("10.") || cidr.starts_with("192.168.") || cidr.starts_with("172.") {
                return Err(ApiError::new(
                    codes::GUARDIAN_NETWORK_VIOLATION,
                    "target address belongs to host network segment",
                ));
            }
        }
        static WHITELIST: &[&str] = &[
            "api.github.com",
            "*.npmjs.org",
            "*.crates.io",
            "registry.npmjs.org",
            "pypi.org",
            "*.pypi.org",
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
        let _ = get_sandbox_or_404!(self.store, sandbox_id);
        let guard = self.store.network_policies.pin();
        let mut policy = guard.get(sandbox_id).cloned().unwrap_or_default();
        policy.default_action = default.into();
        guard.insert(sandbox_id.to_string(), policy);
        Ok(())
    }
}

#[async_trait]
impl ExecManager for MockBackend {
    async fn run(&self, req: ExecRequest) -> Result<ExecResult, ApiError> {
        let _ = get_sandbox_or_404!(self.store, &req.sandbox_id);
        Ok(ExecResult {
            exit_code: 0,
            stdout: format!("(mock) executed: {}\n", req.command),
            stderr: String::new(),
            duration_ms: 42,
        })
    }
}

#[async_trait]
impl FileManager for MockBackend {
    async fn read(
        &self,
        sandbox_id: &str,
        path: &str,
        _offset: Option<u64>,
        _limit: Option<u64>,
    ) -> Result<FileContent, ApiError> {
        let _ = get_sandbox_or_404!(self.store, sandbox_id);
        let guard = self.store.files.pin();
        let fs = guard.get(sandbox_id).cloned().unwrap_or_default();
        let data = fs.get(path).cloned().unwrap_or_default();
        Ok(FileContent {
            size: data.len() as u64,
            content: Bytes::from(data),
            mode: Some("0644".into()),
        })
    }

    async fn write(
        &self,
        sandbox_id: &str,
        path: &str,
        content: &[u8],
        _mode: Option<&str>,
    ) -> Result<(), ApiError> {
        let _ = get_sandbox_or_404!(self.store, sandbox_id);
        let guard = self.store.files.pin();
        let mut fs = guard.get(sandbox_id).cloned().unwrap_or_default();
        fs.insert(path.to_string(), content.to_vec());
        guard.insert(sandbox_id.to_string(), fs);
        Ok(())
    }

    async fn list(&self, sandbox_id: &str, path: &str, _recursive: bool) -> Result<Vec<FileEntry>, ApiError> {
        let _ = get_sandbox_or_404!(self.store, sandbox_id);
        let guard = self.store.files.pin();
        let fs = guard.get(sandbox_id).cloned().unwrap_or_default();
        Ok(fs
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, data)| FileEntry {
                name: p.rsplit('/').next().unwrap_or(p).into(),
                path: p.as_str().into(),
                is_dir: false,
                size: data.len() as u64,
                mode: Some("0644".into()),
                modified_at: Some(now_rfc3339()),
            })
            .collect())
    }

    async fn upload(&self, sandbox_id: &str, dest: &str, data: Bytes) -> Result<(), ApiError> {
        self.write(sandbox_id, dest, &data, None).await
    }

    async fn download(&self, sandbox_id: &str, path: &str) -> Result<Bytes, ApiError> {
        let fc = self.read(sandbox_id, path, None, None).await?;
        Ok(fc.content)
    }
}

#[async_trait]
impl VolumeManager for MockBackend {
    async fn mount(&self, req: VolumeMountRequest) -> Result<(), ApiError> {
        let _ = get_sandbox_or_404!(self.store, &req.sandbox_id);
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
        let _ = get_sandbox_or_404!(self.store, sandbox_id);
        Ok(self
            .store
            .volumes
            .pin()
            .get(sandbox_id)
            .cloned()
            .unwrap_or_default())
    }
}

#[async_trait]
impl AuditManager for MockBackend {
    async fn query(&self, query: AuditQuery) -> Result<AuditQueryResult, ApiError> {
        let _ = get_sandbox_or_404!(self.store, &query.sandbox_id);
        let guard = self.store.audit_events.pin();
        let events = guard.get(&query.sandbox_id).cloned().unwrap_or_default();
        let total = events.len() as u64;
        let items: Vec<AuditEvent> = events
            .into_iter()
            .skip(query.offset as usize)
            .take(query.limit as usize)
            .collect();
        let has_more = (query.offset as u64 + items.len() as u64) < total;
        Ok(AuditQueryResult {
            events: items,
            total,
            has_more,
        })
    }

    async fn detail(&self, sandbox_id: &str, event_id: &str) -> Result<AuditEvent, ApiError> {
        let guard = self.store.audit_events.pin();
        let events = guard.get(sandbox_id).cloned().unwrap_or_default();
        events
            .into_iter()
            .find(|e| &*e.id == event_id)
            .ok_or_else(|| ApiError::not_found("event", event_id))
    }

    async fn replay(&self, req: AuditReplayRequest) -> Result<Vec<AuditEvent>, ApiError> {
        let _ = get_sandbox_or_404!(self.store, &req.sandbox_id);
        Ok(self
            .store
            .audit_events
            .pin()
            .get(&req.sandbox_id)
            .cloned()
            .unwrap_or_default())
    }
}

#[async_trait]
impl ResourceManager for MockBackend {
    async fn usage(&self, sandbox_id: &str) -> Result<ResourceUsage, ApiError> {
        let _ = get_sandbox_or_404!(self.store, sandbox_id);
        Ok(self.store.resource_usage.pin().get(sandbox_id).cloned().unwrap_or_else(|| {
            ResourceUsage {
                cpu: CpuUsage {
                    allocated: 2,
                    usage_percent: 0.0,
                },
                memory: MemoryUsage {
                    allocated: "4GB".into(),
                    used: "0MB".into(),
                    usage_percent: 0.0,
                },
                disk: DiskUsage {
                    allocated: "20GB".into(),
                    used: "0MB".into(),
                    usage_percent: 0.0,
                },
                gpu: None,
            }
        }))
    }

    async fn resize(&self, req: ResourceResizeRequest) -> Result<(), ApiError> {
        let _ = get_sandbox_or_404!(self.store, &req.sandbox_id);
        let guard = self.store.resource_usage.pin();
        if let Some(mut usage) = guard.get(&req.sandbox_id).cloned() {
            if let Some(cpu) = req.cpu {
                usage.cpu.allocated = cpu;
            }
            if let Some(ref mem) = req.memory {
                usage.memory.allocated = mem.as_str().into();
            }
            if let Some(ref disk) = req.disk {
                usage.disk.allocated = disk.as_str().into();
            }
            guard.insert(req.sandbox_id, usage);
        }
        Ok(())
    }
}

#[async_trait]
impl InterManager for MockBackend {
    async fn connect(&self, req: InterConnectRequest) -> Result<(), ApiError> {
        let _ = get_sandbox_or_404!(self.store, &req.sandbox_a);
        let _ = get_sandbox_or_404!(self.store, &req.sandbox_b);
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
                || (c.bidirectional
                    && &*c.sandbox_b == msg.from_sandbox
                    && &*c.sandbox_a == msg.to_sandbox)
        });
        if !has_conn {
            return Err(ApiError::not_found(
                "connection",
                &format!("{}->{}", msg.from_sandbox, msg.to_sandbox),
            ));
        }
        Ok(())
    }

    async fn disconnect(&self, sandbox_a: &str, sandbox_b: &str) -> Result<(), ApiError> {
        let mut conns = self.store.connections.write();
        conns.retain(|c| !(&*c.sandbox_a == sandbox_a && &*c.sandbox_b == sandbox_b));
        Ok(())
    }
}

#[async_trait]
impl ApprovalManager for MockBackend {
    async fn list(&self, filter: ApprovalListFilter) -> Result<Vec<Approval>, ApiError> {
        let guard = self.store.approvals.pin();
        Ok(guard
            .iter()
            .map(|(_k, v)| v.clone())
            .filter(|a| {
                filter
                    .status
                    .as_ref()
                    .map_or(true, |s| a.status.as_str() == s.as_str())
            })
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
            _ => {
                return Err(ApiError::invalid_argument(
                    "decision must be 'approve' or 'deny'",
                ))
            }
        };
        approval.decision_reason = decision.reason.map(Into::into);
        guard.insert(decision.approval_id, approval);
        Ok(())
    }
}

#[async_trait]
impl EventManager for MockBackend {
    fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.store.event_tx.subscribe()
    }

    async fn emit(&self, event: Event) {
        let _ = self.store.event_tx.send(event);
    }
}
