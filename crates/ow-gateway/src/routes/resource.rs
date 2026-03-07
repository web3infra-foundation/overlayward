use axum::{extract::{Path, State}, response::Response, routing::get, Router};
use crate::registry::ServiceRegistry;
use ow_types::*;
use std::sync::Arc;
use crate::error::*;
use crate::extract::*;

pub fn routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .route("/sandboxes/{id}/resources", get(usage).patch(resize))
}

async fn usage(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>) -> Result<Response, AppError> {
    r.guardian.check("resource.usage", &sonic_rs::json!({}), &c).await?;
    Ok(json_ok(r.resource.usage(&id).await?))
}

async fn resize(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Json(mut body): Json<ResourceResizeRequest>) -> Result<Response, AppError> {
    r.guardian.check("resource.resize", &sonic_rs::json!({}), &c).await?;
    body.sandbox_id = id;
    r.resource.resize(body).await?;
    Ok(json_ok(sonic_rs::json!({})))
}
