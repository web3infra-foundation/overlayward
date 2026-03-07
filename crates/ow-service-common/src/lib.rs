use axum::{Router, response::IntoResponse, routing::get};
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceId {
    Gateway,
    Policy,
    Sandbox,
    Audit,
    Data,
}

impl ServiceId {
    #[inline(always)]
    pub const fn default_port(self) -> u16 {
        match self {
            Self::Gateway => 8420,
            Self::Policy  => 8421,
            Self::Sandbox => 8422,
            Self::Audit   => 8423,
            Self::Data    => 8424,
        }
    }

    #[inline(always)]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gateway => "ow-gateway",
            Self::Policy  => "ow-policy",
            Self::Sandbox => "ow-sandbox",
            Self::Audit   => "ow-audit",
            Self::Data    => "ow-data",
        }
    }

    pub const ALL: [ServiceId; 5] = [
        Self::Gateway,
        Self::Policy,
        Self::Sandbox,
        Self::Audit,
        Self::Data,
    ];
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub service: &'static str,
    pub status: &'static str,
    pub port: u16,
}

#[inline]
pub fn health_routes(id: ServiceId) -> Router {
    let resp = HealthResponse {
        service: id.name(),
        status: "ok",
        port: id.default_port(),
    };
    Router::new().route("/healthz", get(move || {
        let r = resp.clone();
        async move {
            let body = sonic_rs::to_string(&r).unwrap_or_default();
            (axum::http::StatusCode::OK, [("content-type", "application/json")], body).into_response()
        }
    }))
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub id: ServiceId,
    pub port: u16,
}

impl ServiceConfig {
    pub fn from_env(id: ServiceId) -> Self {
        let env_key = format!("{}_PORT", id.name().to_uppercase().replace('-', "_"));
        let port = std::env::var(&env_key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| id.default_port());
        Self { id, port }
    }

    #[inline]
    pub fn addr(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.port))
    }
}

pub struct HealthChecker {
    targets: Vec<(ServiceId, u16)>,
}

impl HealthChecker {
    pub fn new(targets: Vec<(ServiceId, u16)>) -> Self {
        Self { targets }
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(2))
                .build()
                .unwrap_or_default();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                for &(id, port) in &self.targets {
                    let url = format!("http://127.0.0.1:{port}/healthz");
                    match client.get(&url).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            tracing::debug!("{} :{port} healthy", id.name());
                        }
                        Ok(resp) => {
                            tracing::warn!("{} :{port} unhealthy ({})", id.name(), resp.status());
                        }
                        Err(e) => {
                            tracing::warn!("{} :{port} unreachable: {e}", id.name());
                        }
                    }
                }
            }
        })
    }
}
