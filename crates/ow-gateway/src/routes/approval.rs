use axum::{extract::{Path, Query, State}, response::Response, routing::{get, post}, Router};
use crate::registry::ServiceRegistry;
use ow_types::*;
use std::sync::Arc;
use crate::error::*;
use crate::extract::*;

pub fn routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .route("/approvals", get(list))
        .route("/approvals/{id}/decide", post(decide))
}

async fn list(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Query(f): Query<ApprovalListFilter>) -> Result<Response, AppError> {
    r.guardian.check("approval.list", &sonic_rs::json!({}), &c).await?;
    Ok(json_ok(r.approval.list(f).await?))
}

async fn decide(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Json(mut body): Json<ApprovalDecision>) -> Result<Response, AppError> {
    r.guardian.check("approval.decide", &sonic_rs::json!({}), &c).await?;
    body.approval_id = id;
    r.approval.decide(body).await?;
    Ok(json_ok(sonic_rs::json!({})))
}
