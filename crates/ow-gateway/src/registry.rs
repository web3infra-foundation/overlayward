use std::sync::Arc;
use crate::traits::*;

pub struct ServiceRegistry {
    pub guardian: Arc<dyn Guardian>,
    pub sandbox: Arc<dyn SandboxManager>,
    pub snapshot: Arc<dyn SnapshotManager>,
    pub network: Arc<dyn NetworkManager>,
    pub exec: Arc<dyn ExecManager>,
    pub file: Arc<dyn FileManager>,
    pub volume: Arc<dyn VolumeManager>,
    pub audit: Arc<dyn AuditManager>,
    pub resource: Arc<dyn ResourceManager>,
    pub inter: Arc<dyn InterManager>,
    pub approval: Arc<dyn ApprovalManager>,
    pub event: Arc<dyn EventManager>,
}
