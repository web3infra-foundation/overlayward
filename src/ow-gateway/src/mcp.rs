use crate::registry::ServiceRegistry;
use ow_types::*;
use rmcp::ServiceExt;
use rmcp::transport::StreamableHttpServerConfig;
use rmcp::transport::StreamableHttpService;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::*,
    service::RequestContext,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct OverlaywardMcp {
    reg: Arc<ServiceRegistry>,
}

impl OverlaywardMcp {
    pub fn new(reg: Arc<ServiceRegistry>) -> Self {
        Self { reg }
    }
}

#[inline(always)]
fn ok_text(s: &str) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(s)]))
}

#[inline(always)]
fn ok_json<T: serde::Serialize>(v: &T) -> Result<CallToolResult, McpError> {
    ok_text(&sonic_rs::to_string(v).unwrap_or_default())
}

#[inline(always)]
fn to_err(e: ApiError) -> McpError {
    McpError::internal_error(e.message.to_string(), None)
}

macro_rules! parse_args {
    ($req:expr, $T:ty) => {{
        let args = $req.arguments.unwrap_or_default();
        serde_json::from_value::<$T>(serde_json::Value::Object(args))
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?
    }};
}

#[derive(Deserialize)]
struct CreateP {
    name: Option<String>,
    cpu: Option<u32>,
    memory: Option<String>,
    disk: Option<String>,
    image: Option<String>,
}
#[derive(Deserialize)]
struct IdP {
    sandbox_id: String,
}
#[derive(Deserialize)]
struct StopP {
    sandbox_id: String,
    force: Option<bool>,
}
#[derive(Deserialize)]
struct ListP {
    status: Option<String>,
}
#[derive(Deserialize)]
struct ExecP {
    sandbox_id: String,
    command: String,
    workdir: Option<String>,
    timeout: Option<String>,
}
#[derive(Deserialize)]
struct FileRwP {
    sandbox_id: String,
    path: String,
    content: Option<String>,
}
#[derive(Deserialize)]
struct FileListP {
    sandbox_id: String,
    path: String,
    recursive: Option<bool>,
}
#[derive(Deserialize)]
struct SnapSaveP {
    sandbox_id: String,
    name: Option<String>,
}
#[derive(Deserialize)]
struct SnapOpP {
    sandbox_id: String,
    snapshot_id: String,
}
#[derive(Deserialize)]
struct DiffP {
    sandbox_id: String,
    from: String,
    to: String,
}
#[derive(Deserialize)]
struct NetAllowP {
    sandbox_id: String,
    domain: String,
    ports: Option<Vec<u16>>,
    reason: Option<String>,
}
#[derive(Deserialize)]
struct InterSendP {
    from_sandbox: String,
    to_sandbox: String,
    message: String,
}

impl ServerHandler for OverlaywardMcp {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.server_info = Implementation::new("overlayward", env!("CARGO_PKG_VERSION"));
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        match request.name.as_ref() {
            "overlayward_create" => {
                let p = parse_args!(request, CreateP);
                let sb = self
                    .reg
                    .sandbox
                    .create(CreateSandboxRequest {
                        name: p.name,
                        cpu: p.cpu.unwrap_or(2),
                        memory: p.memory.unwrap_or("4GB".into()).into(),
                        disk: p.disk.unwrap_or("20GB".into()).into(),
                        image: p.image.unwrap_or("ubuntu:24.04".into()).into(),
                        network_policy: None,
                        gpu: None,
                        labels: Default::default(),
                    })
                    .await
                    .map_err(to_err)?;
                ok_json(&sb)
            }
            "overlayward_start" => {
                let p = parse_args!(request, IdP);
                self.reg
                    .sandbox
                    .start(&p.sandbox_id)
                    .await
                    .map_err(to_err)?;
                ok_text(&format!("started {}", p.sandbox_id))
            }
            "overlayward_stop" => {
                let p = parse_args!(request, StopP);
                self.reg
                    .sandbox
                    .stop(&p.sandbox_id, p.force.unwrap_or(false))
                    .await
                    .map_err(to_err)?;
                ok_text(&format!("stopped {}", p.sandbox_id))
            }
            "overlayward_destroy" => {
                let p = parse_args!(request, IdP);
                self.reg
                    .sandbox
                    .destroy(&p.sandbox_id, DestroyOptions::default())
                    .await
                    .map_err(to_err)?;
                ok_text(&format!("destroyed {}", p.sandbox_id))
            }
            "overlayward_list" => {
                let p = parse_args!(request, ListP);
                ok_json(
                    &self
                        .reg
                        .sandbox
                        .list(ListFilter {
                            status: p.status,
                            ..Default::default()
                        })
                        .await
                        .map_err(to_err)?,
                )
            }
            "overlayward_info" => {
                let p = parse_args!(request, IdP);
                ok_json(&self.reg.sandbox.info(&p.sandbox_id).await.map_err(to_err)?)
            }
            "overlayward_exec" => {
                let p = parse_args!(request, ExecP);
                ok_json(
                    &self
                        .reg
                        .exec
                        .run(ExecRequest {
                            sandbox_id: p.sandbox_id,
                            command: p.command,
                            workdir: p.workdir,
                            env: Default::default(),
                            timeout: p.timeout,
                            stdin: None,
                        })
                        .await
                        .map_err(to_err)?,
                )
            }
            "overlayward_file_read" => {
                let p = parse_args!(request, FileRwP);
                let fc = self
                    .reg
                    .file
                    .read(&p.sandbox_id, &p.path, None, None)
                    .await
                    .map_err(to_err)?;
                ok_text(&String::from_utf8_lossy(&fc.content))
            }
            "overlayward_file_write" => {
                let p = parse_args!(request, FileRwP);
                self.reg
                    .file
                    .write(
                        &p.sandbox_id,
                        &p.path,
                        p.content.unwrap_or_default().as_bytes(),
                        None,
                    )
                    .await
                    .map_err(to_err)?;
                ok_text(&format!("wrote {}", p.path))
            }
            "overlayward_file_list" => {
                let p = parse_args!(request, FileListP);
                ok_json(
                    &self
                        .reg
                        .file
                        .list(&p.sandbox_id, &p.path, p.recursive.unwrap_or(false))
                        .await
                        .map_err(to_err)?,
                )
            }
            "overlayward_snapshot_save" => {
                let p = parse_args!(request, SnapSaveP);
                ok_json(
                    &self
                        .reg
                        .snapshot
                        .save(&p.sandbox_id, p.name.as_deref(), None)
                        .await
                        .map_err(to_err)?,
                )
            }
            "overlayward_snapshot_restore" => {
                let p = parse_args!(request, SnapOpP);
                self.reg
                    .snapshot
                    .restore(&p.sandbox_id, &p.snapshot_id)
                    .await
                    .map_err(to_err)?;
                ok_text(&format!("restored to {}", p.snapshot_id))
            }
            "overlayward_snapshot_list" => {
                let p = parse_args!(request, IdP);
                ok_json(&self.reg.snapshot.list(&p.sandbox_id).await.map_err(to_err)?)
            }
            "overlayward_snapshot_diff" => {
                let p = parse_args!(request, DiffP);
                ok_json(
                    &self
                        .reg
                        .snapshot
                        .diff(&p.sandbox_id, &p.from, &p.to)
                        .await
                        .map_err(to_err)?,
                )
            }
            "overlayward_network_get" => {
                let p = parse_args!(request, IdP);
                ok_json(&self.reg.network.get(&p.sandbox_id).await.map_err(to_err)?)
            }
            "overlayward_network_allow" => {
                let p = parse_args!(request, NetAllowP);
                ok_json(
                    &self
                        .reg
                        .network
                        .allow(AddNetworkRuleRequest {
                            sandbox_id: p.sandbox_id,
                            domain: Some(p.domain),
                            cidr: None,
                            ports: p.ports.unwrap_or_default().into(),
                            protocol: "tcp".into(),
                            reason: p.reason,
                        })
                        .await
                        .map_err(to_err)?,
                )
            }
            "overlayward_resource_usage" => {
                let p = parse_args!(request, IdP);
                ok_json(&self.reg.resource.usage(&p.sandbox_id).await.map_err(to_err)?)
            }
            "overlayward_volume_list" => {
                let p = parse_args!(request, IdP);
                ok_json(&self.reg.volume.list(&p.sandbox_id).await.map_err(to_err)?)
            }
            "overlayward_inter_send" => {
                let p = parse_args!(request, InterSendP);
                self.reg
                    .inter
                    .send(InterMessage {
                        from_sandbox: p.from_sandbox,
                        to_sandbox: p.to_sandbox,
                        message: p.message,
                    })
                    .await
                    .map_err(to_err)?;
                ok_text("message sent")
            }
            other => Err(McpError::invalid_params(
                format!("unknown tool: {other}"),
                None,
            )),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            tools: build_tool_list(),
            next_cursor: None,
            ..Default::default()
        })
    }
}

macro_rules! p {
    ($($k:literal : $v:literal),* $(,)?) => {
        serde_json::json!({ $($k: {"type": $v}),* })
    };
}

fn build_tool_list() -> Vec<Tool> {
    let defs: &[(&str, &str, serde_json::Value, &[&str])] = &[
        (
            "overlayward_create",
            "创建一个新的隔离沙箱环境",
            p!("name":"string","cpu":"integer","memory":"string","disk":"string","image":"string"),
            &[],
        ),
        (
            "overlayward_start",
            "启动一个已创建的沙箱",
            p!("sandbox_id":"string"),
            &["sandbox_id"],
        ),
        (
            "overlayward_stop",
            "停止一个运行中的沙箱",
            p!("sandbox_id":"string","force":"boolean"),
            &["sandbox_id"],
        ),
        (
            "overlayward_destroy",
            "销毁沙箱并清理资源",
            p!("sandbox_id":"string"),
            &["sandbox_id"],
        ),
        (
            "overlayward_list",
            "列出当前可见的沙箱",
            p!("status":"string"),
            &[],
        ),
        (
            "overlayward_info",
            "获取沙箱的详细信息",
            p!("sandbox_id":"string"),
            &["sandbox_id"],
        ),
        (
            "overlayward_exec",
            "在沙箱内执行命令并返回结果",
            p!("sandbox_id":"string","command":"string","workdir":"string","timeout":"string"),
            &["sandbox_id", "command"],
        ),
        (
            "overlayward_file_read",
            "读取沙箱内的文件内容",
            p!("sandbox_id":"string","path":"string"),
            &["sandbox_id", "path"],
        ),
        (
            "overlayward_file_write",
            "向沙箱内写入文件",
            p!("sandbox_id":"string","path":"string","content":"string"),
            &["sandbox_id", "path", "content"],
        ),
        (
            "overlayward_file_list",
            "列出沙箱内目录的文件",
            p!("sandbox_id":"string","path":"string","recursive":"boolean"),
            &["sandbox_id", "path"],
        ),
        (
            "overlayward_snapshot_save",
            "保存沙箱当前状态的快照",
            p!("sandbox_id":"string","name":"string"),
            &["sandbox_id"],
        ),
        (
            "overlayward_snapshot_restore",
            "将沙箱回滚到指定快照的状态",
            p!("sandbox_id":"string","snapshot_id":"string"),
            &["sandbox_id", "snapshot_id"],
        ),
        (
            "overlayward_snapshot_list",
            "列出沙箱的所有快照",
            p!("sandbox_id":"string"),
            &["sandbox_id"],
        ),
        (
            "overlayward_snapshot_diff",
            "比较两个快照之间的文件差异",
            p!("sandbox_id":"string","from":"string","to":"string"),
            &["sandbox_id", "from", "to"],
        ),
        (
            "overlayward_network_get",
            "查看沙箱的当前网络策略",
            p!("sandbox_id":"string"),
            &["sandbox_id"],
        ),
        (
            "overlayward_network_allow",
            "请求添加网络访问规则",
            p!("sandbox_id":"string","domain":"string","ports":"array","reason":"string"),
            &["sandbox_id", "domain"],
        ),
        (
            "overlayward_resource_usage",
            "查看沙箱的资源使用情况",
            p!("sandbox_id":"string"),
            &["sandbox_id"],
        ),
        (
            "overlayward_volume_list",
            "列出沙箱的共享卷",
            p!("sandbox_id":"string"),
            &["sandbox_id"],
        ),
        (
            "overlayward_inter_send",
            "向另一个沙箱发送消息",
            p!("from_sandbox":"string","to_sandbox":"string","message":"string"),
            &["from_sandbox", "to_sandbox", "message"],
        ),
    ];
    defs.iter()
        .map(|(name, desc, props, required)| {
            let schema = serde_json::json!({ "type": "object", "properties": props, "required": required });
            let schema_map: serde_json::Map<String, serde_json::Value> =
                serde_json::from_value(schema).unwrap_or_default();
            Tool::new(name.to_string(), desc.to_string(), schema_map)
        })
        .collect()
}

/// Run MCP server over stdio transport.
pub async fn run_stdio(registry: Arc<ServiceRegistry>) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("MCP server starting on stdio");
    let transport = rmcp::transport::io::stdio();
    let server = OverlaywardMcp::new(registry).serve(transport).await?;
    server.waiting().await?;
    Ok(())
}

/// Run MCP server over Streamable HTTP transport (binds to the given port).
///
/// The MCP endpoint will be available at `http://0.0.0.0:{port}/mcp`.
pub async fn run_http(
    registry: Arc<ServiceRegistry>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let service = StreamableHttpService::new(
        move || Ok(OverlaywardMcp::new(registry.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    let app = axum::Router::new().nest_service("/mcp", service);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("MCP HTTP server listening on {addr} (endpoint: /mcp)");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
