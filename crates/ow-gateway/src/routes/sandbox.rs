use axum::{extract::{Path, Query, State}, response::Response, routing::{get, post}, Router};
use crate::registry::ServiceRegistry;
use ow_types::*;
use std::sync::Arc;
use crate::error::*;
use crate::extract::*;

pub fn routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .route("/sandboxes", post(create).get(list))
        .route("/sandboxes/{id}", get(info).delete(destroy))
        .route("/sandboxes/{id}/start", post(start))
        .route("/sandboxes/{id}/pause", post(pause))
        .route("/sandboxes/{id}/resume", post(resume))
        .route("/sandboxes/{id}/stop", post(stop))
}

async fn create(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Json(req): Json<CreateSandboxRequest>) -> Result<Response, AppError> {
    r.guardian.check("sandbox.create", &sonic_rs::json!({}), &c).await?;
    Ok(json_created(r.sandbox.create(req).await?))
}

async fn list(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Query(f): Query<ListFilter>) -> Result<Response, AppError> {
    r.guardian.check("sandbox.list", &sonic_rs::json!({}), &c).await?;
    Ok(json_ok(r.sandbox.list(f).await?))
}

async fn info(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>) -> Result<Response, AppError> {
    r.guardian.check("sandbox.info", &sonic_rs::json!({}), &c).await?;
    Ok(json_ok(r.sandbox.info(&id).await?))
}

async fn start(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>) -> Result<Response, AppError> {
    r.guardian.check("sandbox.start", &sonic_rs::json!({}), &c).await?;
    r.sandbox.start(&id).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn pause(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>) -> Result<Response, AppError> {
    r.guardian.check("sandbox.pause", &sonic_rs::json!({}), &c).await?;
    r.sandbox.pause(&id).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn resume(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>) -> Result<Response, AppError> {
    r.guardian.check("sandbox.resume", &sonic_rs::json!({}), &c).await?;
    r.sandbox.resume(&id).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn stop(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, body: Option<axum::Json<StopBody>>) -> Result<Response, AppError> {
    r.guardian.check("sandbox.stop", &sonic_rs::json!({}), &c).await?;
    let force = body.map(|b| b.force).unwrap_or(false);
    r.sandbox.stop(&id, force).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn destroy(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Query(opts): Query<DestroyOptions>) -> Result<Response, AppError> {
    r.guardian.check("sandbox.destroy", &sonic_rs::json!({}), &c).await?;
    r.sandbox.destroy(&id, opts).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

#[derive(serde::Deserialize)]
struct StopBody {
    #[serde(default)]
    force: bool,
}
