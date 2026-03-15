use axum::{extract::{Path, State}, response::Response, routing::post, Router};
use crate::registry::ServiceRegistry;
use ow_types::*;
use std::sync::Arc;
use crate::error::*;
use crate::extract::*;

pub fn routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .route("/sandboxes/{id}/volumes", post(mount).get(list).delete(unmount))
}

async fn mount(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Json(mut body): Json<VolumeMountRequest>) -> Result<Response, AppError> {
    r.guardian.check("volume.mount", &sonic_rs::json!({}), &c).await?;
    body.sandbox_id = id;
    r.volume.mount(body).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn unmount(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Json(body): Json<UnmountBody>) -> Result<Response, AppError> {
    r.guardian.check("volume.unmount", &sonic_rs::json!({}), &c).await?;
    r.volume.unmount(&id, &body.guest_path).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn list(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>) -> Result<Response, AppError> {
    r.guardian.check("volume.list", &sonic_rs::json!({}), &c).await?;
    Ok(json_ok(r.volume.list(&id).await?))
}

#[derive(serde::Deserialize)]
struct UnmountBody { guest_path: String }
