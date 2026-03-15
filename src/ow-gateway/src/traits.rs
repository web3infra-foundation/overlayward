use async_trait::async_trait;
use bytes::Bytes;
use ow_types::*;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardianVerdict { Allow, Deny }

#[async_trait]
pub trait Guardian: Send + Sync + 'static {
    async fn check(&self, operation: &str, params: &sonic_rs::Value, caller: &CallerIdentity) -> Result<GuardianVerdict, ApiError>;
}

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
pub trait SnapshotManager: Send + Sync + 'static {
    async fn save(&self, sandbox_id: &str, name: Option<&str>, description: Option<&str>) -> Result<Snapshot, ApiError>;
    async fn restore(&self, sandbox_id: &str, snapshot_id: &str) -> Result<(), ApiError>;
    async fn list(&self, sandbox_id: &str) -> Result<Vec<Snapshot>, ApiError>;
    async fn delete(&self, sandbox_id: &str, snapshot_id: &str) -> Result<(), ApiError>;
    async fn diff(&self, sandbox_id: &str, from: &str, to: &str) -> Result<SnapshotDiff, ApiError>;
}

#[async_trait]
pub trait NetworkManager: Send + Sync + 'static {
    async fn get(&self, sandbox_id: &str) -> Result<NetworkPolicy, ApiError>;
    async fn allow(&self, req: AddNetworkRuleRequest) -> Result<AddRuleResult, ApiError>;
    async fn deny(&self, sandbox_id: &str, rule_id: &str) -> Result<(), ApiError>;
    async fn set_default(&self, sandbox_id: &str, default: &str) -> Result<(), ApiError>;
}

#[async_trait]
pub trait ExecManager: Send + Sync + 'static {
    async fn run(&self, req: ExecRequest) -> Result<ExecResult, ApiError>;
}

#[async_trait]
pub trait FileManager: Send + Sync + 'static {
    async fn read(&self, sandbox_id: &str, path: &str, offset: Option<u64>, limit: Option<u64>) -> Result<FileContent, ApiError>;
    async fn write(&self, sandbox_id: &str, path: &str, content: &[u8], mode: Option<&str>) -> Result<(), ApiError>;
    async fn list(&self, sandbox_id: &str, path: &str, recursive: bool) -> Result<Vec<FileEntry>, ApiError>;
    async fn upload(&self, sandbox_id: &str, dest: &str, data: Bytes) -> Result<(), ApiError>;
    async fn download(&self, sandbox_id: &str, path: &str) -> Result<Bytes, ApiError>;
}

#[async_trait]
pub trait VolumeManager: Send + Sync + 'static {
    async fn mount(&self, req: VolumeMountRequest) -> Result<(), ApiError>;
    async fn unmount(&self, sandbox_id: &str, guest_path: &str) -> Result<(), ApiError>;
    async fn list(&self, sandbox_id: &str) -> Result<Vec<Volume>, ApiError>;
}

#[async_trait]
pub trait AuditManager: Send + Sync + 'static {
    async fn query(&self, query: AuditQuery) -> Result<AuditQueryResult, ApiError>;
    async fn detail(&self, sandbox_id: &str, event_id: &str) -> Result<AuditEvent, ApiError>;
    async fn replay(&self, req: AuditReplayRequest) -> Result<Vec<AuditEvent>, ApiError>;
}

#[async_trait]
pub trait ResourceManager: Send + Sync + 'static {
    async fn usage(&self, sandbox_id: &str) -> Result<ResourceUsage, ApiError>;
    async fn resize(&self, req: ResourceResizeRequest) -> Result<(), ApiError>;
}

#[async_trait]
pub trait InterManager: Send + Sync + 'static {
    async fn connect(&self, req: InterConnectRequest) -> Result<(), ApiError>;
    async fn send(&self, msg: InterMessage) -> Result<(), ApiError>;
    async fn disconnect(&self, sandbox_a: &str, sandbox_b: &str) -> Result<(), ApiError>;
}

#[async_trait]
pub trait ApprovalManager: Send + Sync + 'static {
    async fn list(&self, filter: ApprovalListFilter) -> Result<Vec<Approval>, ApiError>;
    async fn decide(&self, decision: ApprovalDecision) -> Result<(), ApiError>;
}

#[async_trait]
pub trait EventManager: Send + Sync + 'static {
    fn subscribe(&self) -> broadcast::Receiver<Event>;
    async fn emit(&self, event: Event);
}
