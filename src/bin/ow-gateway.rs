use ow_gateway::{GatewayService, MockTokenResolver, ServiceRegistry, InMemoryStore, MockBackend, MockGuardian};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let store = InMemoryStore::new();
    let backend = Arc::new(MockBackend::new(store));
    let registry = Arc::new(ServiceRegistry {
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
    });
    let resolver: Arc<dyn ow_gateway::TokenResolver> = Arc::new(MockTokenResolver);

    GatewayService::new(registry, resolver)
        .run()
        .await
        .expect("ow-gateway failed");
}
