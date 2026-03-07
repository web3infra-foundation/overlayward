use axum::{extract::{Path, State, WebSocketUpgrade, ws::{Message, WebSocket}}, response::Response, routing::post, Router};
use crate::registry::ServiceRegistry;
use ow_types::*;
use std::sync::Arc;
use crate::error::*;
use crate::extract::*;

pub fn routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .route("/sandboxes/{id}/exec", post(run))
        .route("/sandboxes/{id}/shell", axum::routing::get(shell))
}

async fn run(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Json(mut body): Json<ExecRequest>) -> Result<Response, AppError> {
    r.guardian.check("exec.run", &sonic_rs::json!({}), &c).await?;
    body.sandbox_id = id;
    Ok(json_ok(r.exec.run(body).await?))
}

async fn shell(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, ws: WebSocketUpgrade) -> Result<Response, AppError> {
    r.guardian.check("exec.shell", &sonic_rs::json!({}), &c).await?;
    let _ = r.sandbox.info(&id).await?;
    Ok(ws.on_upgrade(move |socket| handle_shell(socket, id)))
}

async fn handle_shell(mut socket: WebSocket, _sandbox_id: String) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(input) => {
                let resp = sonic_rs::json!({ "type": "stdout", "data": format!("(mock) $ {input}") });
                let _ = socket.send(Message::Text(sonic_rs::to_string(&resp).unwrap_or_default().into())).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
