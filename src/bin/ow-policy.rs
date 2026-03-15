#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    ow_policy::PolicyService::new().run().await.expect("ow-policy failed");
}
