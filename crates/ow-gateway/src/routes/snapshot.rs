use axum::{extract::{Path, Query, State}, response::Response, routing::{delete, get, post}, Router};
use crate::registry::ServiceRegistry;
use std::sync::Arc;
use crate::error::*;
use crate::extract::*;

pub fn routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .route("/sandboxes/{id}/snapshots", post(save).get(list))
        .route("/sandboxes/{id}/snapshots/{snap_id}/restore", post(restore))
        .route("/sandboxes/{id}/snapshots/{snap_id}", delete(del))
        .route("/sandboxes/{id}/snapshots/diff", get(diff))
}

async fn save(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Json(body): Json<SnapSaveBody>) -> Result<Response, AppError> {
    r.guardian.check("snapshot.save", &sonic_rs::json!({}), &c).await?;
    Ok(json_created(r.snapshot.save(&id, body.name.as_deref(), body.description.as_deref()).await?))
}

async fn list(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>) -> Result<Response, AppError> {
    r.guardian.check("snapshot.list", &sonic_rs::json!({}), &c).await?;
    Ok(json_ok(r.snapshot.list(&id).await?))
}

async fn restore(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path((id, snap_id)): Path<(String, String)>) -> Result<Response, AppError> {
    r.guardian.check("snapshot.restore", &sonic_rs::json!({}), &c).await?;
    r.snapshot.restore(&id, &snap_id).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn del(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path((id, snap_id)): Path<(String, String)>) -> Result<Response, AppError> {
    r.guardian.check("snapshot.delete", &sonic_rs::json!({}), &c).await?;
    r.snapshot.delete(&id, &snap_id).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn diff(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Query(q): Query<DiffQuery>) -> Result<Response, AppError> {
    r.guardian.check("snapshot.diff", &sonic_rs::json!({}), &c).await?;
    Ok(json_ok(r.snapshot.diff(&id, &q.from, &q.to).await?))
}

#[derive(serde::Deserialize)]
struct SnapSaveBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(serde::Deserialize)]
struct DiffQuery {
    from: String,
    to: String,
}
