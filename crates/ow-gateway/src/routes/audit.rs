use axum::{extract::{Path, Query, State}, response::Response, routing::{get, post}, Router};
use crate::registry::ServiceRegistry;
use ow_types::*;
use std::sync::Arc;
use crate::error::*;
use crate::extract::*;

pub fn routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .route("/sandboxes/{id}/audit", get(query))
        .route("/sandboxes/{id}/audit/{event_id}", get(detail))
        .route("/sandboxes/{id}/audit/replay", post(replay))
}

async fn query(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Query(mut q): Query<AuditQuery>) -> Result<Response, AppError> {
    r.guardian.check("audit.query", &sonic_rs::json!({}), &c).await?;
    q.sandbox_id = id;
    Ok(json_ok(r.audit.query(q).await?))
}

async fn detail(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path((id, event_id)): Path<(String, String)>) -> Result<Response, AppError> {
    r.guardian.check("audit.detail", &sonic_rs::json!({}), &c).await?;
    Ok(json_ok(r.audit.detail(&id, &event_id).await?))
}

async fn replay(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Json(mut body): Json<AuditReplayRequest>) -> Result<Response, AppError> {
    r.guardian.check("audit.replay", &sonic_rs::json!({}), &c).await?;
    body.sandbox_id = id;
    Ok(json_ok(r.audit.replay(body).await?))
}
