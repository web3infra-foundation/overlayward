use async_trait::async_trait;
use bytes::Bytes;
use ow_types::*;
use papaya::HashMap;
use std::sync::Arc;

pub struct SandboxStore {
    pub sandboxes: HashMap<String, Sandbox>,
    pub snapshots: HashMap<String, Vec<Snapshot>>,
    pub resource_usage: HashMap<String, ResourceUsage>,
    pub files: HashMap<String, std::collections::HashMap<String, Vec<u8>>>,
}

impl SandboxStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            sandboxes: HashMap::new(),
            snapshots: HashMap::new(),
            resource_usage: HashMap::new(),
            files: HashMap::new(),
        })
    }
}

pub struct SandboxBackend {
    store: Arc<SandboxStore>,
}

impl SandboxBackend {
    pub fn new(store: Arc<SandboxStore>) -> Self {
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

macro_rules! get_sandbox_or_404 {
    ($store:expr, $id:expr) => {{
        let guard = $store.sandboxes.pin();
        guard.get($id).cloned().ok_or_else(|| ApiError::not_found("sandbox", $id))?
    }};
}

// === SandboxManager ===

#[async_trait]
pub trait SandboxManager: Send + Sync + 'static {
    async fn create(&self, req: CreateSandboxRequest) -> Result<Sandbox, ApiError>;
    async fn start(&self, id: &str) -> Result<(), ApiError>;
    async fn pause(&self, id: &str) -> Result<(), ApiError>;
    async fn resume(&self, id: &str) -> Result<(), ApiError>;
    async fn stop(&self, id: &str, force: bool) -> Result<(), ApiError>;
    async fn destroy(&self, id: &str, opts: DestroyOptions) -> Result<(), ApiError>;
    async fn list(&self, filter: ListFilter) -> Result<Vec<Sandbox>, ApiError>;
    async fn info(&self, id: &str) -> Result<Sandbox, ApiError>;
}

#[async_trait]
impl SandboxManager for SandboxBackend {
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
        self.store.resource_usage.pin().insert(
            id.to_string(),
            ResourceUsage {
                cpu: CpuUsage { allocated: sandbox.cpu, usage_percent: 0.0 },
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
        guard.remove(id).ok_or_else(|| ApiError::not_found("sandbox", id))?;
        self.store.snapshots.pin().remove(id);
        self.store.resource_usage.pin().remove(id);
        self.store.files.pin().remove(id);
        Ok(())
    }

    async fn list(&self, filter: ListFilter) -> Result<Vec<Sandbox>, ApiError> {
        let guard = self.store.sandboxes.pin();
        let mut result: Vec<Sandbox> = guard
            .iter()
            .map(|(_k, v)| v.clone())
            .filter(|s| filter.status.as_ref().map_or(true, |st| s.status.as_str() == st.as_str()))
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
fn transition_status(store: &SandboxStore, id: &str, target: SandboxStatus) -> Result<(), ApiError> {
    let guard = store.sandboxes.pin();
    let mut sb = guard.get(id).cloned().ok_or_else(|| ApiError::not_found("sandbox", id))?;
    if !sb.status.can_transition_to(target) {
        return Err(ApiError::invalid_transition(sb.status.as_str(), target.as_str()));
    }
    sb.status = target;
    guard.insert(id.to_string(), sb);
    Ok(())
}

// === ExecManager ===

#[async_trait]
pub trait ExecManager: Send + Sync + 'static {
    async fn run(&self, req: ExecRequest) -> Result<ExecResult, ApiError>;
}

#[async_trait]
impl ExecManager for SandboxBackend {
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

// === FileManager ===

#[async_trait]
pub trait FileManager: Send + Sync + 'static {
    async fn read(&self, sandbox_id: &str, path: &str, offset: Option<u64>, limit: Option<u64>) -> Result<FileContent, ApiError>;
    async fn write(&self, sandbox_id: &str, path: &str, content: &[u8], mode: Option<&str>) -> Result<(), ApiError>;
    async fn list(&self, sandbox_id: &str, path: &str, recursive: bool) -> Result<Vec<FileEntry>, ApiError>;
    async fn upload(&self, sandbox_id: &str, dest: &str, data: Bytes) -> Result<(), ApiError>;
    async fn download(&self, sandbox_id: &str, path: &str) -> Result<Bytes, ApiError>;
}

#[async_trait]
impl FileManager for SandboxBackend {
    async fn read(&self, sandbox_id: &str, path: &str, _offset: Option<u64>, _limit: Option<u64>) -> Result<FileContent, ApiError> {
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

    async fn write(&self, sandbox_id: &str, path: &str, content: &[u8], _mode: Option<&str>) -> Result<(), ApiError> {
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

// === SnapshotManager ===

#[async_trait]
pub trait SnapshotManager: Send + Sync + 'static {
    async fn save(&self, sandbox_id: &str, name: Option<&str>, description: Option<&str>) -> Result<Snapshot, ApiError>;
    async fn restore(&self, sandbox_id: &str, snapshot_id: &str) -> Result<(), ApiError>;
    async fn list(&self, sandbox_id: &str) -> Result<Vec<Snapshot>, ApiError>;
    async fn delete(&self, sandbox_id: &str, snapshot_id: &str) -> Result<(), ApiError>;
    async fn diff(&self, sandbox_id: &str, from: &str, to: &str) -> Result<SnapshotDiff, ApiError>;
}

#[async_trait]
impl SnapshotManager for SandboxBackend {
    async fn save(&self, sandbox_id: &str, name: Option<&str>, description: Option<&str>) -> Result<Snapshot, ApiError> {
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
        Ok(self.store.snapshots.pin().get(sandbox_id).cloned().unwrap_or_default())
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

// === ResourceManager ===

#[async_trait]
pub trait ResourceManager: Send + Sync + 'static {
    async fn usage(&self, sandbox_id: &str) -> Result<ResourceUsage, ApiError>;
    async fn resize(&self, req: ResourceResizeRequest) -> Result<(), ApiError>;
}

#[async_trait]
impl ResourceManager for SandboxBackend {
    async fn usage(&self, sandbox_id: &str) -> Result<ResourceUsage, ApiError> {
        let _ = get_sandbox_or_404!(self.store, sandbox_id);
        Ok(self.store.resource_usage.pin().get(sandbox_id).cloned().unwrap_or_else(|| ResourceUsage {
            cpu: CpuUsage { allocated: 2, usage_percent: 0.0 },
            memory: MemoryUsage { allocated: "4GB".into(), used: "0MB".into(), usage_percent: 0.0 },
            disk: DiskUsage { allocated: "20GB".into(), used: "0MB".into(), usage_percent: 0.0 },
            gpu: None,
        }))
    }

    async fn resize(&self, req: ResourceResizeRequest) -> Result<(), ApiError> {
        let _ = get_sandbox_or_404!(self.store, &req.sandbox_id);
        let guard = self.store.resource_usage.pin();
        if let Some(mut usage) = guard.get(&req.sandbox_id).cloned() {
            if let Some(cpu) = req.cpu { usage.cpu.allocated = cpu; }
            if let Some(ref mem) = req.memory { usage.memory.allocated = mem.as_str().into(); }
            if let Some(ref disk) = req.disk { usage.disk.allocated = disk.as_str().into(); }
            guard.insert(req.sandbox_id, usage);
        }
        Ok(())
    }
}
