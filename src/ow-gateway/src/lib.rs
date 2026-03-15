pub mod traits;
pub mod auth;
pub mod registry;
pub mod mock;
pub mod error;
pub mod extract;
pub mod middleware;
pub mod routes;
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
    pub h3_port: u16,
    pub mcp_port: u16,
}

impl GatewayService {
    pub fn new(registry: Arc<ServiceRegistry>, token_resolver: Arc<dyn TokenResolver>) -> Self {
        Self {
            registry,
            token_resolver,
            rest_port: ServiceId::Gateway.default_port(),
            h3_port: 8425,
            mcp_port: 8426,
        }
    }

    #[inline]
    pub fn with_ports(mut self, rest: u16, h3: u16, mcp: u16) -> Self {
        self.rest_port = rest;
        self.h3_port = h3;
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

        // HTTP/3 over QUIC
        let h3_reg = self.registry.clone();
        let h3_resolver = self.token_resolver.clone();
        let h3_port = self.h3_port;
        let h3 = tokio::spawn(async move {
            run_h3(h3_reg, h3_resolver, h3_port).await.expect("HTTP/3 server failed");
        });

        // MCP HTTP
        let mcp_reg = self.registry.clone();
        let mcp_port = self.mcp_port;
        let mcp = tokio::spawn(async move {
            mcp::run_http(mcp_reg, mcp_port).await.expect("MCP HTTP server failed");
        });

        tracing::info!(
            "Overlayward Gateway started — REST :{rest_port} | H3 :{h3_port} | MCP :{mcp_port}",
            rest_port = self.rest_port, h3_port = self.h3_port, mcp_port = self.mcp_port,
        );

        HealthChecker::new(vec![
            (ServiceId::Policy, ServiceId::Policy.default_port()),
        ]).spawn();

        tokio::select! {
            r = rest => { if let Err(e) = r { tracing::error!("REST: {e}"); } }
            r = h3 => { if let Err(e) = r { tracing::error!("H3: {e}"); } }
            r = mcp => { if let Err(e) = r { tracing::error!("MCP: {e}"); } }
        }
        Ok(())
    }
}

#[inline]
fn build_app(registry: Arc<ServiceRegistry>, resolver: Arc<dyn TokenResolver>) -> axum::Router {
    use axum::{Extension, middleware as axum_mw};
    use tower_http::cors::CorsLayer;
    use tower_http::trace::TraceLayer;

    let auth_state = middleware::AuthState { resolver };
    let api = routes::api_routes(registry)
        .layer(axum_mw::from_fn(middleware::auth_middleware))
        .layer(Extension(auth_state));
    axum::Router::new()
        .merge(ow_service_common::health_routes(ServiceId::Gateway))
        .merge(api)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

async fn run_rest(registry: Arc<ServiceRegistry>, resolver: Arc<dyn TokenResolver>, port: u16) -> std::io::Result<()> {
    let app = build_app(registry, resolver);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("REST API listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}

async fn run_h3(registry: Arc<ServiceRegistry>, resolver: Arc<dyn TokenResolver>, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use h3_quinn::quinn;

    // Crypto provider (ring — matches our rustls feature flags)
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Self-signed cert for dev; replace with file-based certs for production
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(generated.key_pair.serialize_der().into());
    let cert_der = rustls::pki_types::CertificateDer::from(generated.cert);

    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key)?;
    tls_config.alpn_protocols = vec![b"h3".to_vec()];
    tls_config.max_early_data_size = u32::MAX;

    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)?,
    ));

    let transport = Arc::get_mut(&mut server_config.transport).unwrap();
    transport
        .max_concurrent_bidi_streams(256_u32.into())
        .max_concurrent_uni_streams(256_u32.into())
        .max_idle_timeout(Some(std::time::Duration::from_secs(120).try_into()?));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let endpoint = quinn::Endpoint::server(server_config, addr)?;
    tracing::info!("HTTP/3 (QUIC) listening on {addr}");

    let app = build_app(registry, resolver);

    while let Some(incoming) = endpoint.accept().await {
        let app = app.clone();
        tokio::spawn(async move {
            if let Err(e) = h3_handle_conn(incoming, app).await {
                tracing::error!("H3 connection error: {e}");
            }
        });
    }
    Ok(())
}

async fn h3_handle_conn(
    incoming: h3_quinn::quinn::Incoming,
    app: axum::Router,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let conn = incoming.await?;
    let h3_conn = h3::server::builder()
        .build(h3_quinn::Connection::new(conn))
        .await?;
    tokio::pin!(h3_conn);

    loop {
        match h3_conn.accept().await {
            Ok(Some(resolver)) => {
                let app = app.clone();
                tokio::spawn(async move {
                    if let Err(e) = h3_axum::serve_h3_with_axum(app, resolver).await {
                        tracing::error!("H3 request error: {e}");
                    }
                });
            }
            Ok(None) => break,
            Err(e) => {
                if !h3_axum::is_graceful_h3_close(&e) {
                    tracing::error!("H3 error: {e:?}");
                }
                break;
            }
        }
    }
    Ok(())
}

/// Run MCP server over stdio transport.
pub async fn run_mcp_stdio(registry: Arc<ServiceRegistry>) -> Result<(), Box<dyn std::error::Error>> {
    mcp::run_stdio(registry).await
}
