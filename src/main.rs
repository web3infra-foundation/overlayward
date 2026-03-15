use clap::Parser;
use ow_cli::{Cli, Commands};
use ow_gateway::{GatewayService, MockTokenResolver, ServiceRegistry, InMemoryStore, MockBackend, MockGuardian};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Serve(args) => run_serve(args.rest_port, args.h3_port, args.mcp_port).await,
        Commands::McpServer => run_mcp().await,
        _ => {
            let code = ow_cli::run(cli).await;
            std::process::exit(code);
        }
    }
}

async fn run_serve(rest_port: u16, h3_port: u16, mcp_port: u16) {
    // Start the 4 lightweight services first (policy, sandbox, audit, data)
    let sandbox = tokio::spawn(async { ow_sandbox::SandboxService::new().run().await.expect("ow-sandbox failed"); });
    let policy  = tokio::spawn(async { ow_policy::PolicyService::new().run().await.expect("ow-policy failed"); });
    let audit   = tokio::spawn(async { ow_audit::AuditService::new().run().await.expect("ow-audit failed"); });
    let data    = tokio::spawn(async { ow_data::DataService::new().run().await.expect("ow-data failed"); });

    // Gateway uses explicit ports from CLI args
    let registry = build_registry();
    let resolver: Arc<dyn ow_gateway::TokenResolver> = Arc::new(MockTokenResolver);
    let gateway = tokio::spawn(async move {
        GatewayService::new(registry, resolver)
            .with_ports(rest_port, h3_port, mcp_port)
            .run()
            .await
            .expect("ow-gateway failed");
    });

    tracing::info!("Overlayward serve-all started — 5 services running");

    tokio::select! {
        r = sandbox => { if let Err(e) = r { tracing::error!("ow-sandbox: {e}"); } }
        r = policy  => { if let Err(e) = r { tracing::error!("ow-policy: {e}"); } }
        r = audit   => { if let Err(e) = r { tracing::error!("ow-audit: {e}"); } }
        r = data    => { if let Err(e) = r { tracing::error!("ow-data: {e}"); } }
        r = gateway => { if let Err(e) = r { tracing::error!("ow-gateway: {e}"); } }
    }
}

async fn run_mcp() {
    let registry = build_registry();
    ow_gateway::run_mcp_stdio(registry).await.expect("MCP server failed");
}

fn build_registry() -> Arc<ServiceRegistry> {
    let store = InMemoryStore::new();
    let backend = Arc::new(MockBackend::new(store));
    Arc::new(ServiceRegistry {
        guardian: Arc::new(MockGuardian),
        sandbox: backend.clone(),
        snapshot: backend.clone(),
        network: backend.clone(),
        exec: backend.clone(),
        file: backend.clone(),
        volume: backend.clone(),
        audit: backend.clone(),
        resource: backend.clone(),
        inter: backend.clone(),
        approval: backend.clone(),
        event: backend,
    })
}
