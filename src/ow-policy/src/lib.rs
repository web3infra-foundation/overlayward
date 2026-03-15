pub mod mock;

use ow_service_common::{HealthChecker, ServiceConfig, ServiceId, health_routes};
use std::net::SocketAddr;

pub struct PolicyService {
    pub port: u16,
}

impl PolicyService {
    pub fn new() -> Self {
        Self {
            port: ServiceConfig::from_env(ServiceId::Policy).port,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub async fn run(self) -> std::io::Result<()> {
        let app = health_routes(ServiceId::Policy);
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        tracing::info!("{} listening on {addr}", ServiceId::Policy.name());

        HealthChecker::new(vec![
            (ServiceId::Sandbox, ServiceId::Sandbox.default_port()),
            (ServiceId::Audit, ServiceId::Audit.default_port()),
            (ServiceId::Data, ServiceId::Data.default_port()),
        ]).spawn();

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await
    }
}
