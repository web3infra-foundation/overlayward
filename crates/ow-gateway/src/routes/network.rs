use axum::{extract::{Path, State}, response::Response, routing::{delete, get, post, put}, Router};
use crate::registry::ServiceRegistry;
use ow_types::*;
use std::sync::Arc;
use crate::error::*;
use crate::extract::*;

pub fn routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .route("/sandboxes/{id}/network", get(get_policy))
        .route("/sandboxes/{id}/network/rules", post(allow_rule))
        .route("/sandboxes/{id}/network/rules/{rule_id}", delete(deny_rule))
        .route("/sandboxes/{id}/network/default", put(set_default))
}

async fn get_policy(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>) -> Result<Response, AppError> {
    r.guardian.check("network.get", &sonic_rs::json!({}), &c).await?;
    Ok(json_ok(r.network.get(&id).await?))
}

async fn allow_rule(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Json(mut body): Json<AddNetworkRuleRequest>) -> Result<Response, AppError> {
    r.guardian.check("network.allow", &sonic_rs::json!({}), &c).await?;
    body.sandbox_id = id;
    let result = r.network.allow(body).await?;
    match &result {
        AddRuleResult::Allowed { .. } => Ok(json_ok(result)),
        AddRuleResult::ApprovalRequired { .. } => {
            use axum::response::IntoResponse;
            let body_str = sonic_rs::to_string(&ApiResponse::<()>::err(
                ApiError::new(codes::APPROVAL_REQUIRED, "此操作需要人类审批").with_detail(sonic_rs::to_value(&result).unwrap_or_default()),
            )).unwrap_or_default();
            Ok((axum::http::StatusCode::UNPROCESSABLE_ENTITY, [("content-type","application/json")], body_str).into_response())
        }
    }
}

async fn deny_rule(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path((id, rule_id)): Path<(String, String)>) -> Result<Response, AppError> {
    r.guardian.check("network.deny", &sonic_rs::json!({}), &c).await?;
    r.network.deny(&id, &rule_id).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn set_default(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Json(body): Json<SetDefaultBody>) -> Result<Response, AppError> {
    r.guardian.check("network.set_default", &sonic_rs::json!({}), &c).await?;
    r.network.set_default(&id, &body.default).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

#[derive(serde::Deserialize)]
struct SetDefaultBody {
    default: String,
}
