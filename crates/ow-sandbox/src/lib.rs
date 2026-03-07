pub mod mock;
pub mod routes;

use ow_service_common::{ServiceConfig, ServiceId, health_routes};
use std::net::SocketAddr;

pub struct SandboxService {
    pub backend: std::sync::Arc<mock::SandboxBackend>,
    pub port: u16,
}

impl SandboxService {
    pub fn new() -> Self {
        let store = mock::SandboxStore::new();
        Self {
            backend: std::sync::Arc::new(mock::SandboxBackend::new(store)),
            port: ServiceConfig::from_env(ServiceId::Sandbox).port,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub async fn run(self) -> std::io::Result<()> {
        let app = routes::sandbox_routes(self.backend.clone())
            .merge(health_routes(ServiceId::Sandbox));
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        tracing::info!("{} listening on {addr}", ServiceId::Sandbox.name());
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await
    }
}
