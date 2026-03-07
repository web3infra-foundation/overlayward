use crate::proto::*;
use crate::registry::ServiceRegistry;
use ow_types as ot;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

macro_rules! svc_struct {
    ($name:ident) => {
        pub struct $name(pub Arc<ServiceRegistry>);
    };
}

macro_rules! to_status {
    ($e:expr) => {
        match $e {
            Ok(v) => Ok(v),
            Err(e) => Err(Status::new(
                tonic::Code::from_i32(e.grpc_code()),
                e.message.to_string(),
            )),
        }
    };
}

svc_struct!(SandboxSvc);
svc_struct!(SnapshotSvc);
svc_struct!(NetworkSvc);
svc_struct!(ExecSvc);
svc_struct!(FileSvc);
svc_struct!(VolumeSvc);
svc_struct!(AuditSvc);
svc_struct!(ResourceSvc);
svc_struct!(InterSvc);
svc_struct!(ApprovalSvc);
svc_struct!(EventSvc);

#[tonic::async_trait]
impl sandbox_service_server::SandboxService for SandboxSvc {
    async fn create(
        &self,
        req: Request<CreateSandboxRequest>,
    ) -> Result<Response<CreateSandboxResponse>, Status> {
        let r = req.into_inner();
        let sb = to_status!(self
            .0
            .sandbox
            .create(ot::CreateSandboxRequest {
                name: if r.name.is_empty() { None } else { Some(r.name) },
                cpu: r.cpu as u32,
                memory: if r.memory.is_empty() {
                    "4GB".into()
                } else {
                    r.memory.into()
                },
                disk: if r.disk.is_empty() {
                    "20GB".into()
                } else {
                    r.disk.into()
                },
                image: if r.image.is_empty() {
                    "ubuntu:24.04".into()
                } else {
                    r.image.into()
                },
                gpu: r.gpu.map(|g| ot::GpuConfig {
                    device: Some(g.device.into()),
                    count: g.count,
                }),
                labels: r.labels,
                network_policy: None,
            })
            .await)?;
        let conn = sb.connection.as_ref();
        Ok(Response::new(CreateSandboxResponse {
            sandbox_id: sb.sandbox_id.to_string(),
            name: sb.name.to_string(),
            status: sb.status.as_str().to_string(),
            created_at: sb.created_at.to_string(),
            connection: conn.map(|c| ConnectionInfo {
                vsock_cid: c.vsock_cid,
                api_endpoint: c.api_endpoint.to_string(),
            }),
        }))
    }

    async fn list(&self, req: Request<ListSandboxRequest>) -> Result<Response<ListSandboxResponse>, Status> {
        let r = req.into_inner();
        let sbs = to_status!(self
            .0
            .sandbox
            .list(ot::ListFilter {
                status: if r.status.is_empty() {
                    None
                } else {
                    Some(r.status)
                },
                labels: if r.labels.is_empty() {
                    None
                } else {
                    Some(r.labels)
                },
                owner: if r.owner.is_empty() { None } else { Some(r.owner) },
                limit: if r.limit == 0 { 20 } else { r.limit as u32 },
                offset: r.offset as u32,
            })
            .await)?;
        let total = sbs.len() as i32;
        Ok(Response::new(ListSandboxResponse {
            sandboxes: sbs
                .into_iter()
                .map(|s| SandboxInfo {
                    sandbox_id: s.sandbox_id.to_string(),
                    name: s.name.to_string(),
                    status: s.status.as_str().to_string(),
                    cpu: s.cpu as i32,
                    memory: s.memory.to_string(),
                    disk: s.disk.to_string(),
                    uptime: s.uptime.map(|u| u.to_string()).unwrap_or_default(),
                    owner: s.owner.to_string(),
                    created_at: s.created_at.to_string(),
                    labels: s.labels,
                })
                .collect(),
            total,
            has_more: false,
        }))
    }

    async fn get_info(&self, req: Request<SandboxIdRequest>) -> Result<Response<SandboxInfo>, Status> {
        let s = to_status!(self.0.sandbox.info(&req.into_inner().sandbox_id).await)?;
        Ok(Response::new(SandboxInfo {
            sandbox_id: s.sandbox_id.to_string(),
            name: s.name.to_string(),
            status: s.status.as_str().to_string(),
            cpu: s.cpu as i32,
            memory: s.memory.to_string(),
            disk: s.disk.to_string(),
            uptime: s.uptime.map(|u| u.to_string()).unwrap_or_default(),
            owner: s.owner.to_string(),
            created_at: s.created_at.to_string(),
            labels: s.labels,
        }))
    }

    async fn start(&self, req: Request<SandboxIdRequest>) -> Result<Response<Empty>, Status> {
        to_status!(self.0.sandbox.start(&req.into_inner().sandbox_id).await)?;
        Ok(Response::new(Empty {}))
    }

    async fn pause(&self, req: Request<SandboxIdRequest>) -> Result<Response<Empty>, Status> {
        to_status!(self.0.sandbox.pause(&req.into_inner().sandbox_id).await)?;
        Ok(Response::new(Empty {}))
    }

    async fn resume(&self, req: Request<SandboxIdRequest>) -> Result<Response<Empty>, Status> {
        to_status!(self.0.sandbox.resume(&req.into_inner().sandbox_id).await)?;
        Ok(Response::new(Empty {}))
    }

    async fn stop(&self, req: Request<StopSandboxRequest>) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self.0.sandbox.stop(&r.sandbox_id, r.force).await)?;
        Ok(Response::new(Empty {}))
    }

    async fn destroy(&self, req: Request<DestroySandboxRequest>) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self
            .0
            .sandbox
            .destroy(
                &r.sandbox_id,
                ot::DestroyOptions {
                    keep_snapshots: r.keep_snapshots,
                    keep_audit_logs: r.keep_audit_logs,
                },
            )
            .await)?;
        Ok(Response::new(Empty {}))
    }
}

#[tonic::async_trait]
impl snapshot_service_server::SnapshotService for SnapshotSvc {
    async fn save(&self, req: Request<SaveSnapshotRequest>) -> Result<Response<SnapshotInfo>, Status> {
        let r = req.into_inner();
        let s = to_status!(self
            .0
            .snapshot
            .save(
                &r.sandbox_id,
                if r.name.is_empty() {
                    None
                } else {
                    Some(r.name.as_str())
                },
                if r.description.is_empty() {
                    None
                } else {
                    Some(r.description.as_str())
                },
            )
            .await)?;
        Ok(Response::new(SnapshotInfo {
            snapshot_id: s.snapshot_id.to_string(),
            name: s.name.to_string(),
            sandbox_id: s.sandbox_id.to_string(),
            created_at: s.created_at.to_string(),
            size: s.size.to_string(),
            audit_pointer: s
                .audit_pointer
                .map(|a| a.to_string())
                .unwrap_or_default(),
        }))
    }

    async fn list(
        &self,
        req: Request<SandboxIdRequest>,
    ) -> Result<Response<ListSnapshotResponse>, Status> {
        let snaps = to_status!(self.0.snapshot.list(&req.into_inner().sandbox_id).await)?;
        Ok(Response::new(ListSnapshotResponse {
            snapshots: snaps
                .into_iter()
                .map(|s| SnapshotInfo {
                    snapshot_id: s.snapshot_id.to_string(),
                    name: s.name.to_string(),
                    sandbox_id: s.sandbox_id.to_string(),
                    created_at: s.created_at.to_string(),
                    size: s.size.to_string(),
                    audit_pointer: s
                        .audit_pointer
                        .map(|a| a.to_string())
                        .unwrap_or_default(),
                })
                .collect(),
        }))
    }

    async fn restore(&self, req: Request<RestoreSnapshotRequest>) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self.0.snapshot.restore(&r.sandbox_id, &r.snapshot_id).await)?;
        Ok(Response::new(Empty {}))
    }

    async fn delete(&self, req: Request<DeleteSnapshotRequest>) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self.0.snapshot.delete(&r.sandbox_id, &r.snapshot_id).await)?;
        Ok(Response::new(Empty {}))
    }

    async fn diff(&self, req: Request<DiffSnapshotRequest>) -> Result<Response<DiffSnapshotResponse>, Status> {
        let r = req.into_inner();
        let d = to_status!(self.0.snapshot.diff(&r.sandbox_id, &r.from, &r.to).await)?;
        Ok(Response::new(DiffSnapshotResponse {
            files_added: d.files_added.into_iter().map(|s| s.to_string()).collect(),
            files_modified: d.files_modified.into_iter().map(|s| s.to_string()).collect(),
            files_deleted: d.files_deleted.into_iter().map(|s| s.to_string()).collect(),
            summary: d.summary.to_string(),
        }))
    }
}

#[tonic::async_trait]
impl network_service_server::NetworkService for NetworkSvc {
    async fn get(&self, req: Request<SandboxIdRequest>) -> Result<Response<NetworkPolicy>, Status> {
        let p = to_status!(self.0.network.get(&req.into_inner().sandbox_id).await)?;
        Ok(Response::new(NetworkPolicy {
            default_action: p.default_action.to_string(),
            rules: p
                .rules
                .into_iter()
                .map(|r| NetworkRule {
                    rule_id: r.rule_id.to_string(),
                    domain: r.domain.map(|d| d.to_string()).unwrap_or_default(),
                    cidr: r.cidr.map(|c| c.to_string()).unwrap_or_default(),
                    ports: r.ports.iter().map(|&p| p as i32).collect(),
                    protocol: r.protocol.to_string(),
                })
                .collect(),
        }))
    }

    async fn allow_rule(
        &self,
        req: Request<AddNetworkRuleRequest>,
    ) -> Result<Response<AddNetworkRuleResponse>, Status> {
        let r = req.into_inner();
        let result = to_status!(self
            .0
            .network
            .allow(ot::AddNetworkRuleRequest {
                sandbox_id: r.sandbox_id,
                domain: if r.domain.is_empty() {
                    None
                } else {
                    Some(r.domain)
                },
                cidr: if r.cidr.is_empty() { None } else { Some(r.cidr) },
                ports: r.ports.into_iter().map(|p| p as u16).collect(),
                protocol: if r.protocol.is_empty() {
                    "tcp".into()
                } else {
                    r.protocol.into()
                },
                reason: if r.reason.is_empty() {
                    None
                } else {
                    Some(r.reason)
                },
            })
            .await)?;
        Ok(Response::new(match result {
            ot::AddRuleResult::Allowed { rule_id } => AddNetworkRuleResponse {
                result: Some(add_network_rule_response::Result::RuleId(rule_id.to_string())),
            },
            ot::AddRuleResult::ApprovalRequired {
                approval_id,
                status,
                timeout,
            } => AddNetworkRuleResponse {
                result: Some(add_network_rule_response::Result::Approval(ApprovalPending {
                    approval_id: approval_id.to_string(),
                    status: status.to_string(),
                    timeout: timeout.to_string(),
                })),
            },
        }))
    }

    async fn deny_rule(&self, req: Request<DeleteNetworkRuleRequest>) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self.0.network.deny(&r.sandbox_id, &r.rule_id).await)?;
        Ok(Response::new(Empty {}))
    }

    async fn set_default(&self, req: Request<SetDefaultNetworkRequest>) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self.0.network.set_default(&r.sandbox_id, &r.default_action).await)?;
        Ok(Response::new(Empty {}))
    }
}

#[tonic::async_trait]
impl exec_service_server::ExecService for ExecSvc {
    async fn run(&self, req: Request<ExecRunRequest>) -> Result<Response<ExecRunResponse>, Status> {
        let r = req.into_inner();
        let res = to_status!(self
            .0
            .exec
            .run(ot::ExecRequest {
                sandbox_id: r.sandbox_id,
                command: r.command,
                workdir: if r.workdir.is_empty() {
                    None
                } else {
                    Some(r.workdir)
                },
                env: r.env,
                timeout: if r.timeout.is_empty() {
                    None
                } else {
                    Some(r.timeout)
                },
                stdin: None,
            })
            .await)?;
        Ok(Response::new(ExecRunResponse {
            exit_code: res.exit_code,
            stdout: res.stdout,
            stderr: res.stderr,
            duration_ms: res.duration_ms as i64,
        }))
    }

    type ShellStream = ReceiverStream<Result<ShellOutput, Status>>;

    async fn shell(
        &self,
        req: Request<tonic::Streaming<ShellInput>>,
    ) -> Result<Response<Self::ShellStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        let mut stream = req.into_inner();
        tokio::spawn(async move {
            while let Ok(Some(input)) = stream.message().await {
                let echo = format!("(mock) $ {}", String::from_utf8_lossy(&input.data));
                let _ = tx
                    .send(Ok(ShellOutput {
                        data: echo.into_bytes(),
                        exit_code: 0,
                        exited: false,
                    }))
                    .await;
            }
            let _ = tx
                .send(Ok(ShellOutput {
                    data: vec![],
                    exit_code: 0,
                    exited: true,
                }))
                .await;
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tonic::async_trait]
impl file_service_server::FileService for FileSvc {
    async fn read(&self, req: Request<FileReadRequest>) -> Result<Response<FileContent>, Status> {
        let r = req.into_inner();
        let off = if r.offset == 0 {
            None
        } else {
            Some(r.offset as u64)
        };
        let lim = if r.limit == 0 {
            None
        } else {
            Some(r.limit as u64)
        };
        let fc = to_status!(self.0.file.read(&r.sandbox_id, &r.path, off, lim).await)?;
        Ok(Response::new(FileContent {
            content: fc.content.to_vec(),
            size: fc.size as i64,
            mode: fc.mode.map(|m| m.to_string()).unwrap_or_default(),
        }))
    }

    async fn write(&self, req: Request<FileWriteRequest>) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let mode = if r.mode.is_empty() {
            None
        } else {
            Some(r.mode.as_str())
        };
        to_status!(self.0.file.write(&r.sandbox_id, &r.path, &r.content, mode).await)?;
        Ok(Response::new(Empty {}))
    }

    async fn list(&self, req: Request<FileListRequest>) -> Result<Response<FileListResponse>, Status> {
        let r = req.into_inner();
        let entries = to_status!(self.0.file.list(&r.sandbox_id, &r.path, r.recursive).await)?;
        Ok(Response::new(FileListResponse {
            entries: entries
                .into_iter()
                .map(|e| FileEntry {
                    name: e.name.to_string(),
                    path: e.path.to_string(),
                    is_dir: e.is_dir,
                    size: e.size as i64,
                    mode: e.mode.map(|m| m.to_string()).unwrap_or_default(),
                    modified_at: e.modified_at.map(|m| m.to_string()).unwrap_or_default(),
                })
                .collect(),
        }))
    }

    async fn upload(&self, req: Request<tonic::Streaming<FileUploadChunk>>) -> Result<Response<Empty>, Status> {
        let mut stream = req.into_inner();
        let mut sandbox_id = String::new();
        let mut dest = String::new();
        let mut buf = Vec::new();
        while let Some(chunk) = stream
            .message()
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            if sandbox_id.is_empty() {
                sandbox_id = chunk.sandbox_id;
                dest = chunk.dest_path;
            }
            buf.extend_from_slice(&chunk.data);
        }
        to_status!(self
            .0
            .file
            .upload(&sandbox_id, &dest, bytes::Bytes::from(buf))
            .await)?;
        Ok(Response::new(Empty {}))
    }

    type DownloadStream = ReceiverStream<Result<FileDownloadChunk, Status>>;

    async fn download(
        &self,
        req: Request<FileDownloadRequest>,
    ) -> Result<Response<Self::DownloadStream>, Status> {
        let r = req.into_inner();
        let data = to_status!(self.0.file.download(&r.sandbox_id, &r.path).await)?;
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            for chunk in data.chunks(65536) {
                let _ = tx.send(Ok(FileDownloadChunk { data: chunk.to_vec() })).await;
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tonic::async_trait]
impl volume_service_server::VolumeService for VolumeSvc {
    async fn mount(
        &self,
        req: Request<crate::proto::VolumeMountRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self
            .0
            .volume
            .mount(ot::VolumeMountRequest {
                sandbox_id: r.sandbox_id,
                host_path: r.host_path,
                guest_path: r.guest_path,
                mode: if r.mode.is_empty() {
                    "ro".into()
                } else {
                    r.mode.into()
                },
            })
            .await)?;
        Ok(Response::new(Empty {}))
    }

    async fn unmount(&self, req: Request<VolumeUnmountRequest>) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self.0.volume.unmount(&r.sandbox_id, &r.guest_path).await)?;
        Ok(Response::new(Empty {}))
    }

    async fn list(&self, req: Request<SandboxIdRequest>) -> Result<Response<VolumeListResponse>, Status> {
        let vols = to_status!(self.0.volume.list(&req.into_inner().sandbox_id).await)?;
        Ok(Response::new(VolumeListResponse {
            volumes: vols
                .into_iter()
                .map(|v| VolumeInfo {
                    host_path: v.host_path.to_string(),
                    guest_path: v.guest_path.to_string(),
                    mode: v.mode.to_string(),
                })
                .collect(),
        }))
    }
}

#[tonic::async_trait]
impl audit_service_server::AuditService for AuditSvc {
    async fn query(&self, req: Request<AuditQueryRequest>) -> Result<Response<AuditQueryResponse>, Status> {
        let r = req.into_inner();
        let res = to_status!(self
            .0
            .audit
            .query(ot::AuditQuery {
                sandbox_id: r.sandbox_id,
                from: if r.from.is_empty() { None } else { Some(r.from) },
                to: if r.to.is_empty() { None } else { Some(r.to) },
                level: if r.level.is_empty() {
                    None
                } else {
                    Some(r.level)
                },
                limit: if r.limit == 0 { 100 } else { r.limit as u32 },
                offset: r.offset as u32,
            })
            .await)?;
        Ok(Response::new(AuditQueryResponse {
            events: res.events.into_iter().map(audit_event_to_proto).collect(),
            total: res.total as i64,
            has_more: res.has_more,
        }))
    }

    async fn get_detail(
        &self,
        req: Request<AuditDetailRequest>,
    ) -> Result<Response<crate::proto::AuditEvent>, Status> {
        let r = req.into_inner();
        let e = to_status!(self.0.audit.detail(&r.sandbox_id, &r.event_id).await)?;
        Ok(Response::new(audit_event_to_proto(e)))
    }

    type ReplayStream = ReceiverStream<Result<crate::proto::AuditEvent, Status>>;

    async fn replay(
        &self,
        req: Request<AuditReplayRequest>,
    ) -> Result<Response<Self::ReplayStream>, Status> {
        let r = req.into_inner();
        let events = to_status!(self
            .0
            .audit
            .replay(ot::AuditReplayRequest {
                sandbox_id: r.sandbox_id,
                from: r.from,
                to: if r.to.is_empty() { None } else { Some(r.to) },
                speed: if r.speed == 0.0 { 1.0 } else { r.speed },
            })
            .await)?;
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        tokio::spawn(async move {
            for e in events {
                let _ = tx.send(Ok(audit_event_to_proto(e))).await;
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

fn audit_event_to_proto(e: ot::AuditEvent) -> crate::proto::AuditEvent {
    crate::proto::AuditEvent {
        id: e.id.to_string(),
        timestamp: e.timestamp.to_string(),
        level: e.level.as_str().to_string(),
        agent_id: e.agent_id.map(|a| a.to_string()).unwrap_or_default(),
        content: sonic_rs::to_vec(&e.content).unwrap_or_default(),
    }
}

#[tonic::async_trait]
impl resource_service_server::ResourceService for ResourceSvc {
    async fn get_usage(
        &self,
        req: Request<SandboxIdRequest>,
    ) -> Result<Response<crate::proto::ResourceUsage>, Status> {
        let u = to_status!(self.0.resource.usage(&req.into_inner().sandbox_id).await)?;
        Ok(Response::new(crate::proto::ResourceUsage {
            cpu: Some(crate::proto::CpuUsage {
                allocated: u.cpu.allocated,
                usage_percent: u.cpu.usage_percent,
            }),
            memory: Some(crate::proto::MemoryUsage {
                allocated: u.memory.allocated.to_string(),
                used: u.memory.used.to_string(),
                usage_percent: u.memory.usage_percent,
            }),
            disk: Some(crate::proto::DiskUsage {
                allocated: u.disk.allocated.to_string(),
                used: u.disk.used.to_string(),
                usage_percent: u.disk.usage_percent,
            }),
            gpu: u.gpu.map(|g| crate::proto::GpuUsage {
                device: g.device.to_string(),
                memory_used: g.memory_used.to_string(),
                utilization_percent: g.utilization_percent,
            }),
        }))
    }

    async fn resize(
        &self,
        req: Request<crate::proto::ResourceResizeRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self
            .0
            .resource
            .resize(ot::ResourceResizeRequest {
                sandbox_id: r.sandbox_id,
                cpu: if r.cpu == 0 { None } else { Some(r.cpu as u32) },
                memory: if r.memory.is_empty() {
                    None
                } else {
                    Some(r.memory)
                },
                disk: if r.disk.is_empty() { None } else { Some(r.disk) },
            })
            .await)?;
        Ok(Response::new(Empty {}))
    }
}

#[tonic::async_trait]
impl inter_service_server::InterService for InterSvc {
    async fn open_channel(
        &self,
        req: Request<crate::proto::InterConnectRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self
            .0
            .inter
            .connect(ot::InterConnectRequest {
                sandbox_a: r.sandbox_a,
                sandbox_b: r.sandbox_b,
                mode: if r.mode.is_empty() {
                    "message".into()
                } else {
                    r.mode.into()
                },
                bidirectional: r.bidirectional,
            })
            .await)?;
        Ok(Response::new(Empty {}))
    }

    async fn send(&self, req: Request<InterSendRequest>) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self
            .0
            .inter
            .send(ot::InterMessage {
                from_sandbox: r.from_sandbox,
                to_sandbox: r.to_sandbox,
                message: String::from_utf8_lossy(&r.message).to_string(),
            })
            .await)?;
        Ok(Response::new(Empty {}))
    }

    async fn disconnect(
        &self,
        req: Request<InterDisconnectRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self.0.inter.disconnect(&r.sandbox_a, &r.sandbox_b).await)?;
        Ok(Response::new(Empty {}))
    }
}

#[tonic::async_trait]
impl approval_service_server::ApprovalService for ApprovalSvc {
    async fn list(&self, req: Request<ApprovalListRequest>) -> Result<Response<ApprovalListResponse>, Status> {
        let r = req.into_inner();
        let approvals = to_status!(self
            .0
            .approval
            .list(ot::ApprovalListFilter {
                status: if r.status.is_empty() {
                    None
                } else {
                    Some(r.status)
                },
            })
            .await)?;
        Ok(Response::new(ApprovalListResponse {
            approvals: approvals
                .into_iter()
                .map(|a| ApprovalInfo {
                    approval_id: a.approval_id.to_string(),
                    requester: a.requester.to_string(),
                    sandbox_id: a.sandbox_id.to_string(),
                    operation: a.operation.to_string(),
                    status: a.status.as_str().to_string(),
                    created_at: a.created_at.to_string(),
                    timeout: a.timeout.to_string(),
                    reason: a.reason.map(|r| r.to_string()).unwrap_or_default(),
                    decision_reason: a.decision_reason.map(|r| r.to_string()).unwrap_or_default(),
                })
                .collect(),
        }))
    }

    async fn decide(&self, req: Request<ApprovalDecideRequest>) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        to_status!(self
            .0
            .approval
            .decide(ot::ApprovalDecision {
                approval_id: r.approval_id,
                decision: r.decision,
                reason: if r.reason.is_empty() {
                    None
                } else {
                    Some(r.reason)
                },
                permanent: r.permanent,
            })
            .await)?;
        Ok(Response::new(Empty {}))
    }
}

#[tonic::async_trait]
impl event_service_server::EventService for EventSvc {
    type SubscribeStream = ReceiverStream<Result<crate::proto::Event, Status>>;

    async fn subscribe(
        &self,
        _req: Request<EventSubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let mut rx = self.0.event.subscribe();
        let (tx, out_rx) = tokio::sync::mpsc::channel(64);
        tokio::spawn(async move {
            while let Ok(evt) = rx.recv().await {
                let pe = crate::proto::Event {
                    r#type: evt.event_type.to_string(),
                    timestamp: evt.timestamp.to_string(),
                    sandbox_id: evt.sandbox_id.map(|s| s.to_string()).unwrap_or_default(),
                    data: sonic_rs::to_vec(&evt.data).unwrap_or_default(),
                };
                if tx.send(Ok(pe)).await.is_err() {
                    break;
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(out_rx)))
    }
}
