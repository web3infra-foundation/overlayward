pub mod mock;

use ow_service_common::{ServiceConfig, ServiceId, health_routes};
use std::net::SocketAddr;

pub struct DataService {
    pub port: u16,
}

impl DataService {
    pub fn new() -> Self {
        Self {
            port: ServiceConfig::from_env(ServiceId::Data).port,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub async fn run(self) -> std::io::Result<()> {
        let app = health_routes(ServiceId::Data);
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        tracing::info!("{} listening on {addr}", ServiceId::Data.name());
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await
    }
}
