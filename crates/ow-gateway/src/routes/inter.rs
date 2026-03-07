use axum::{extract::State, response::Response, routing::post, Router};
use crate::registry::ServiceRegistry;
use ow_types::*;
use std::sync::Arc;
use crate::error::*;
use crate::extract::*;

pub fn routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .route("/inter/connections", post(connect).delete(disconnect))
        .route("/inter/messages", post(send_msg))
}

async fn connect(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Json(body): Json<InterConnectRequest>) -> Result<Response, AppError> {
    r.guardian.check("inter.connect", &sonic_rs::json!({}), &c).await?;
    r.inter.connect(body).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn send_msg(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Json(body): Json<InterMessage>) -> Result<Response, AppError> {
    r.guardian.check("inter.send", &sonic_rs::json!({}), &c).await?;
    r.inter.send(body).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn disconnect(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Json(body): Json<InterDisconnectRequest>) -> Result<Response, AppError> {
    r.guardian.check("inter.disconnect", &sonic_rs::json!({}), &c).await?;
    r.inter.disconnect(&body.sandbox_a, &body.sandbox_b).await?;
    Ok(json_ok(sonic_rs::json!({})))
}
