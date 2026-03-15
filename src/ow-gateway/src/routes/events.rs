use axum::{extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}}, response::Response, Router};
use crate::registry::ServiceRegistry;
use std::sync::Arc;
use crate::error::*;
use crate::extract::*;

pub fn routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .route("/events", axum::routing::get(subscribe))
}

async fn subscribe(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, ws: WebSocketUpgrade) -> Result<Response, AppError> {
    r.guardian.check("events.subscribe", &sonic_rs::json!({}), &c).await?;
    let rx = r.event.subscribe();
    Ok(ws.on_upgrade(move |socket| handle_events(socket, rx)))
}

async fn handle_events(mut socket: WebSocket, mut rx: tokio::sync::broadcast::Receiver<ow_types::Event>) {
    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(e) => {
                        let json = sonic_rs::to_string(&e).unwrap_or_default();
                        if socket.send(Message::Text(json.into())).await.is_err() { break; }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
