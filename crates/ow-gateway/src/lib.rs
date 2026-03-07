pub mod proto {
    include!("proto/overlayward.v1.rs");
}

pub mod traits;
pub mod auth;
pub mod registry;
pub mod mock;
pub mod error;
pub mod extract;
pub mod middleware;
pub mod routes;
pub mod grpc_services;
pub mod mcp;

pub use registry::ServiceRegistry;
pub use auth::{TokenResolver, MockTokenResolver};
pub use mock::{InMemoryStore, MockBackend, MockGuardian};

use ow_service_common::{HealthChecker, ServiceId};
use std::sync::Arc;
use std::net::SocketAddr;

pub struct GatewayService {
    pub registry: Arc<ServiceRegistry>,
    pub token_resolver: Arc<dyn TokenResolver>,
    pub rest_port: u16,
    pub grpc_port: u16,
    pub mcp_port: u16,
}

impl GatewayService {
    pub fn new(registry: Arc<ServiceRegistry>, token_resolver: Arc<dyn TokenResolver>) -> Self {
        Self {
            registry,
            token_resolver,
            rest_port: ServiceId::Gateway.default_port(),
            grpc_port: 8425,
            mcp_port: 8426,
        }
    }

    #[inline]
    pub fn with_ports(mut self, rest: u16, grpc: u16, mcp: u16) -> Self {
        self.rest_port = rest;
        self.grpc_port = grpc;
        self.mcp_port = mcp;
        self
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        // REST
        let rest_reg = self.registry.clone();
        let rest_resolver = self.token_resolver.clone();
        let rest_port = self.rest_port;
        let rest = tokio::spawn(async move {
            run_rest(rest_reg, rest_resolver, rest_port).await.expect("REST server failed");
        });

        // gRPC
        let grpc_reg = self.registry.clone();
        let grpc_port = self.grpc_port;
        let grpc = tokio::spawn(async move {
            run_grpc(grpc_reg, grpc_port).await.expect("gRPC server failed");
        });

        // MCP HTTP
        let mcp_reg = self.registry.clone();
        let mcp_port = self.mcp_port;
        let mcp = tokio::spawn(async move {
            mcp::run_http(mcp_reg, mcp_port).await.expect("MCP HTTP server failed");
        });

        tracing::info!("Overlayward Gateway started — REST :{rest_port} | gRPC :{grpc_port} | MCP :{mcp_port}",
            rest_port = self.rest_port, grpc_port = self.grpc_port, mcp_port = self.mcp_port);

        HealthChecker::new(vec![
            (ServiceId::Policy, ServiceId::Policy.default_port()),
        ]).spawn();

        tokio::select! {
            r = rest => { if let Err(e) = r { tracing::error!("REST: {e}"); } }
            r = grpc => { if let Err(e) = r { tracing::error!("gRPC: {e}"); } }
            r = mcp => { if let Err(e) = r { tracing::error!("MCP: {e}"); } }
        }
        Ok(())
    }
}

async fn run_rest(registry: Arc<ServiceRegistry>, resolver: Arc<dyn TokenResolver>, port: u16) -> std::io::Result<()> {
    use axum::{Extension, middleware as axum_mw};
    use tower_http::cors::CorsLayer;
    use tower_http::trace::TraceLayer;

    let auth_state = middleware::AuthState { resolver };
    let api = routes::api_routes(registry)
        .layer(axum_mw::from_fn(middleware::auth_middleware))
        .layer(Extension(auth_state));
    let app = axum::Router::new()
        .merge(ow_service_common::health_routes(ServiceId::Gateway))
        .merge(api)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("REST API listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}

async fn run_grpc(registry: Arc<ServiceRegistry>, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("gRPC API listening on {addr}");
    let r = registry;
    tonic::transport::Server::builder()
        .add_service(proto::sandbox_service_server::SandboxServiceServer::new(grpc_services::SandboxSvc(r.clone())))
        .add_service(proto::snapshot_service_server::SnapshotServiceServer::new(grpc_services::SnapshotSvc(r.clone())))
        .add_service(proto::network_service_server::NetworkServiceServer::new(grpc_services::NetworkSvc(r.clone())))
        .add_service(proto::exec_service_server::ExecServiceServer::new(grpc_services::ExecSvc(r.clone())))
        .add_service(proto::file_service_server::FileServiceServer::new(grpc_services::FileSvc(r.clone())))
        .add_service(proto::volume_service_server::VolumeServiceServer::new(grpc_services::VolumeSvc(r.clone())))
        .add_service(proto::audit_service_server::AuditServiceServer::new(grpc_services::AuditSvc(r.clone())))
        .add_service(proto::resource_service_server::ResourceServiceServer::new(grpc_services::ResourceSvc(r.clone())))
        .add_service(proto::inter_service_server::InterServiceServer::new(grpc_services::InterSvc(r.clone())))
        .add_service(proto::approval_service_server::ApprovalServiceServer::new(grpc_services::ApprovalSvc(r.clone())))
        .add_service(proto::event_service_server::EventServiceServer::new(grpc_services::EventSvc(r)))
        .serve(addr)
        .await?;
    Ok(())
}

/// Run MCP server over stdio transport.
pub async fn run_mcp_stdio(registry: Arc<ServiceRegistry>) -> Result<(), Box<dyn std::error::Error>> {
    mcp::run_stdio(registry).await
}
