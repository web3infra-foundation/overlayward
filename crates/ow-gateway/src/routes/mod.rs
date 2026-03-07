pub mod sandbox;
pub mod snapshot;
pub mod network;
pub mod exec;
pub mod file;
pub mod volume;
pub mod audit;
pub mod resource;
pub mod inter;
pub mod approval;
pub mod events;

use axum::Router;
use crate::registry::ServiceRegistry;
use std::sync::Arc;

pub fn api_routes(reg: Arc<ServiceRegistry>) -> Router {
    Router::new()
        .nest("/api/v1", v1_routes())
        .with_state(reg)
}

fn v1_routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .merge(sandbox::routes())
        .merge(snapshot::routes())
        .merge(network::routes())
        .merge(exec::routes())
        .merge(file::routes())
        .merge(volume::routes())
        .merge(audit::routes())
        .merge(resource::routes())
        .merge(inter::routes())
        .merge(approval::routes())
        .merge(events::routes())
}
