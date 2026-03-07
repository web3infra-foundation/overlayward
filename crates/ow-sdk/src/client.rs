use crate::OverlaywardError;
use ow_gateway::proto::*;
use tonic::transport::Channel;
use tonic::metadata::MetadataValue;
use tonic::Request;

type Result<T> = std::result::Result<T, OverlaywardError>;

#[derive(Debug, Clone)]
pub struct Config {
    pub endpoint: String,
    pub token: String,
}

impl Default for Config {
    fn default() -> Self {
        Self { endpoint: "http://localhost:8421".into(), token: String::new() }
    }
}

#[derive(Clone)]
pub struct Client {
    channel: Channel,
    token: String,
}

impl Client {
    pub async fn new(cfg: Config) -> Result<Self> {
        let channel = Channel::from_shared(cfg.endpoint)
            .map_err(|e| OverlaywardError::Internal { message: e.to_string() })?
            .connect()
            .await?;
        Ok(Self { channel, token: cfg.token })
    }

    fn inject_token<T>(&self, mut req: Request<T>) -> Request<T> {
        if !self.token.is_empty() {
            if let Ok(v) = format!("Bearer {}", self.token).parse::<MetadataValue<tonic::metadata::Ascii>>() {
                req.metadata_mut().insert("authorization", v);
            }
        }
        req
    }

    #[inline] pub fn sandbox(&self) -> SandboxClient { SandboxClient(self.clone()) }
    #[inline] pub fn snapshot(&self) -> SnapshotClient { SnapshotClient(self.clone()) }
    #[inline] pub fn network(&self) -> NetworkClient { NetworkClient(self.clone()) }
    #[inline] pub fn exec(&self) -> ExecClient { ExecClient(self.clone()) }
    #[inline] pub fn file(&self) -> FileClient { FileClient(self.clone()) }
    #[inline] pub fn volume(&self) -> VolumeClient { VolumeClient(self.clone()) }
    #[inline] pub fn audit(&self) -> AuditClient { AuditClient(self.clone()) }
    #[inline] pub fn resource(&self) -> ResourceClient { ResourceClient(self.clone()) }
    #[inline] pub fn inter(&self) -> InterClient { InterClient(self.clone()) }
    #[inline] pub fn approval(&self) -> ApprovalClient { ApprovalClient(self.clone()) }
}

macro_rules! define_sub_client {
    ($name:ident, $svc:path) => {
        pub struct $name(Client);
        impl $name {
            #[inline(always)]
            fn svc(&self) -> $svc { <$svc>::new(self.0.channel.clone()) }
        }
    };
}

define_sub_client!(SandboxClient, sandbox_service_client::SandboxServiceClient<Channel>);
define_sub_client!(SnapshotClient, snapshot_service_client::SnapshotServiceClient<Channel>);
define_sub_client!(NetworkClient, network_service_client::NetworkServiceClient<Channel>);
define_sub_client!(ExecClient, exec_service_client::ExecServiceClient<Channel>);
define_sub_client!(FileClient, file_service_client::FileServiceClient<Channel>);
define_sub_client!(VolumeClient, volume_service_client::VolumeServiceClient<Channel>);
define_sub_client!(AuditClient, audit_service_client::AuditServiceClient<Channel>);
define_sub_client!(ResourceClient, resource_service_client::ResourceServiceClient<Channel>);
define_sub_client!(InterClient, inter_service_client::InterServiceClient<Channel>);
define_sub_client!(ApprovalClient, approval_service_client::ApprovalServiceClient<Channel>);

impl SandboxClient {
    pub async fn create(&self, req: CreateSandboxRequest) -> Result<CreateSandboxResponse> {
        Ok(self.svc().create(self.0.inject_token(Request::new(req))).await?.into_inner())
    }
    pub async fn list(&self, req: ListSandboxRequest) -> Result<ListSandboxResponse> {
        Ok(self.svc().list(self.0.inject_token(Request::new(req))).await?.into_inner())
    }
    pub async fn info(&self, sandbox_id: &str) -> Result<SandboxInfo> {
        Ok(self.svc().get_info(self.0.inject_token(Request::new(SandboxIdRequest { sandbox_id: sandbox_id.into() }))).await?.into_inner())
    }
    pub async fn start(&self, sandbox_id: &str) -> Result<()> {
        self.svc().start(self.0.inject_token(Request::new(SandboxIdRequest { sandbox_id: sandbox_id.into() }))).await?;
        Ok(())
    }
    pub async fn pause(&self, sandbox_id: &str) -> Result<()> {
        self.svc().pause(self.0.inject_token(Request::new(SandboxIdRequest { sandbox_id: sandbox_id.into() }))).await?;
        Ok(())
    }
    pub async fn resume(&self, sandbox_id: &str) -> Result<()> {
        self.svc().resume(self.0.inject_token(Request::new(SandboxIdRequest { sandbox_id: sandbox_id.into() }))).await?;
        Ok(())
    }
    pub async fn stop(&self, sandbox_id: &str, force: bool) -> Result<()> {
        self.svc().stop(self.0.inject_token(Request::new(StopSandboxRequest { sandbox_id: sandbox_id.into(), force }))).await?;
        Ok(())
    }
    pub async fn destroy(&self, sandbox_id: &str, keep_snapshots: bool, keep_audit_logs: bool) -> Result<()> {
        self.svc().destroy(self.0.inject_token(Request::new(DestroySandboxRequest { sandbox_id: sandbox_id.into(), keep_snapshots, keep_audit_logs }))).await?;
        Ok(())
    }
}

impl SnapshotClient {
    pub async fn save(&self, sandbox_id: &str, name: &str) -> Result<SnapshotInfo> {
        Ok(self.svc().save(self.0.inject_token(Request::new(SaveSnapshotRequest { sandbox_id: sandbox_id.into(), name: name.into(), description: String::new() }))).await?.into_inner())
    }
    pub async fn list(&self, sandbox_id: &str) -> Result<Vec<SnapshotInfo>> {
        Ok(self.svc().list(self.0.inject_token(Request::new(SandboxIdRequest { sandbox_id: sandbox_id.into() }))).await?.into_inner().snapshots)
    }
    pub async fn restore(&self, sandbox_id: &str, snapshot_id: &str) -> Result<()> {
        self.svc().restore(self.0.inject_token(Request::new(RestoreSnapshotRequest { sandbox_id: sandbox_id.into(), snapshot_id: snapshot_id.into() }))).await?;
        Ok(())
    }
    pub async fn delete(&self, sandbox_id: &str, snapshot_id: &str) -> Result<()> {
        self.svc().delete(self.0.inject_token(Request::new(DeleteSnapshotRequest { sandbox_id: sandbox_id.into(), snapshot_id: snapshot_id.into() }))).await?;
        Ok(())
    }
    pub async fn diff(&self, sandbox_id: &str, from: &str, to: &str) -> Result<DiffSnapshotResponse> {
        Ok(self.svc().diff(self.0.inject_token(Request::new(DiffSnapshotRequest { sandbox_id: sandbox_id.into(), from: from.into(), to: to.into() }))).await?.into_inner())
    }
}

impl NetworkClient {
    pub async fn get(&self, sandbox_id: &str) -> Result<NetworkPolicy> {
        Ok(self.svc().get(self.0.inject_token(Request::new(SandboxIdRequest { sandbox_id: sandbox_id.into() }))).await?.into_inner())
    }
    pub async fn allow(&self, req: AddNetworkRuleRequest) -> Result<AddNetworkRuleResponse> {
        Ok(self.svc().allow_rule(self.0.inject_token(Request::new(req))).await?.into_inner())
    }
    pub async fn deny(&self, sandbox_id: &str, rule_id: &str) -> Result<()> {
        self.svc().deny_rule(self.0.inject_token(Request::new(DeleteNetworkRuleRequest { sandbox_id: sandbox_id.into(), rule_id: rule_id.into() }))).await?;
        Ok(())
    }
    pub async fn set_default(&self, sandbox_id: &str, default_action: &str) -> Result<()> {
        self.svc().set_default(self.0.inject_token(Request::new(SetDefaultNetworkRequest { sandbox_id: sandbox_id.into(), default_action: default_action.into() }))).await?;
        Ok(())
    }
}

impl ExecClient {
    pub async fn run(&self, req: ExecRunRequest) -> Result<ExecRunResponse> {
        Ok(self.svc().run(self.0.inject_token(Request::new(req))).await?.into_inner())
    }
}

impl FileClient {
    pub async fn read(&self, sandbox_id: &str, path: &str) -> Result<FileContent> {
        Ok(self.svc().read(self.0.inject_token(Request::new(FileReadRequest { sandbox_id: sandbox_id.into(), path: path.into(), offset: 0, limit: 0 }))).await?.into_inner())
    }
    pub async fn write(&self, sandbox_id: &str, path: &str, content: &[u8], mode: &str) -> Result<()> {
        self.svc().write(self.0.inject_token(Request::new(FileWriteRequest { sandbox_id: sandbox_id.into(), path: path.into(), content: content.to_vec(), mode: mode.into() }))).await?;
        Ok(())
    }
    pub async fn list(&self, sandbox_id: &str, path: &str, recursive: bool) -> Result<Vec<FileEntry>> {
        Ok(self.svc().list(self.0.inject_token(Request::new(FileListRequest { sandbox_id: sandbox_id.into(), path: path.into(), recursive }))).await?.into_inner().entries)
    }
}

impl VolumeClient {
    pub async fn mount(&self, req: VolumeMountRequest) -> Result<()> {
        self.svc().mount(self.0.inject_token(Request::new(req))).await?; Ok(())
    }
    pub async fn unmount(&self, sandbox_id: &str, guest_path: &str) -> Result<()> {
        self.svc().unmount(self.0.inject_token(Request::new(VolumeUnmountRequest { sandbox_id: sandbox_id.into(), guest_path: guest_path.into() }))).await?; Ok(())
    }
    pub async fn list(&self, sandbox_id: &str) -> Result<Vec<VolumeInfo>> {
        Ok(self.svc().list(self.0.inject_token(Request::new(SandboxIdRequest { sandbox_id: sandbox_id.into() }))).await?.into_inner().volumes)
    }
}

impl AuditClient {
    pub async fn query(&self, req: AuditQueryRequest) -> Result<AuditQueryResponse> {
        Ok(self.svc().query(self.0.inject_token(Request::new(req))).await?.into_inner())
    }
    pub async fn detail(&self, sandbox_id: &str, event_id: &str) -> Result<AuditEvent> {
        Ok(self.svc().get_detail(self.0.inject_token(Request::new(AuditDetailRequest { sandbox_id: sandbox_id.into(), event_id: event_id.into() }))).await?.into_inner())
    }
}

impl ResourceClient {
    pub async fn usage(&self, sandbox_id: &str) -> Result<ResourceUsage> {
        Ok(self.svc().get_usage(self.0.inject_token(Request::new(SandboxIdRequest { sandbox_id: sandbox_id.into() }))).await?.into_inner())
    }
    pub async fn resize(&self, req: ResourceResizeRequest) -> Result<()> {
        self.svc().resize(self.0.inject_token(Request::new(req))).await?; Ok(())
    }
}

impl InterClient {
    pub async fn open_channel(&self, req: InterConnectRequest) -> Result<()> {
        self.svc().open_channel(self.0.inject_token(Request::new(req))).await?; Ok(())
    }
    pub async fn send(&self, req: InterSendRequest) -> Result<()> {
        self.svc().send(self.0.inject_token(Request::new(req))).await?; Ok(())
    }
    pub async fn disconnect(&self, sandbox_a: &str, sandbox_b: &str) -> Result<()> {
        self.svc().disconnect(self.0.inject_token(Request::new(InterDisconnectRequest { sandbox_a: sandbox_a.into(), sandbox_b: sandbox_b.into() }))).await?; Ok(())
    }
}

impl ApprovalClient {
    pub async fn list(&self, status: &str) -> Result<Vec<ApprovalInfo>> {
        Ok(self.svc().list(self.0.inject_token(Request::new(ApprovalListRequest { status: status.into() }))).await?.into_inner().approvals)
    }
    pub async fn decide(&self, req: ApprovalDecideRequest) -> Result<()> {
        self.svc().decide(self.0.inject_token(Request::new(req))).await?; Ok(())
    }
}
