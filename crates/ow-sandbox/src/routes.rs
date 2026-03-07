use axum::{extract::{Multipart, Path, Query, State, WebSocketUpgrade, ws::{Message, WebSocket}}, response::{IntoResponse, Response}, routing::{delete, get, post}, Router};
use ow_types::*;
use std::sync::Arc;
use crate::mock::SandboxBackend;

type Reg = Arc<SandboxBackend>;

pub fn sandbox_routes(backend: Reg) -> Router {
    Router::new()
        // sandbox CRUD
        .route("/api/v1/sandboxes", post(create).get(list))
        .route("/api/v1/sandboxes/{id}", get(info).delete(destroy))
        .route("/api/v1/sandboxes/{id}/start", post(start))
        .route("/api/v1/sandboxes/{id}/pause", post(pause))
        .route("/api/v1/sandboxes/{id}/resume", post(resume))
        .route("/api/v1/sandboxes/{id}/stop", post(stop))
        // exec
        .route("/api/v1/sandboxes/{id}/exec", post(exec_run))
        .route("/api/v1/sandboxes/{id}/shell", get(shell))
        // files
        .route("/api/v1/sandboxes/{id}/files", get(file_read).put(file_write))
        .route("/api/v1/sandboxes/{id}/files/list", get(file_list))
        .route("/api/v1/sandboxes/{id}/files/upload", post(file_upload))
        .route("/api/v1/sandboxes/{id}/files/download", get(file_download))
        // snapshots
        .route("/api/v1/sandboxes/{id}/snapshots", post(snap_save).get(snap_list))
        .route("/api/v1/sandboxes/{id}/snapshots/{snap_id}/restore", post(snap_restore))
        .route("/api/v1/sandboxes/{id}/snapshots/{snap_id}", delete(snap_delete))
        .route("/api/v1/sandboxes/{id}/snapshots/diff", get(snap_diff))
        // resource
        .route("/api/v1/sandboxes/{id}/resources", get(resource_usage).patch(resource_resize))
        .with_state(backend)
}

#[inline(always)]
fn json_ok<T: serde::Serialize>(data: T) -> Response {
    let body = sonic_rs::to_string(&ApiResponse::ok(data)).unwrap_or_default();
    (axum::http::StatusCode::OK, [("content-type", "application/json")], body).into_response()
}

#[inline(always)]
fn json_created<T: serde::Serialize>(data: T) -> Response {
    let body = sonic_rs::to_string(&ApiResponse::ok(data)).unwrap_or_default();
    (axum::http::StatusCode::CREATED, [("content-type", "application/json")], body).into_response()
}

#[inline(always)]
fn json_err(e: ApiError) -> Response {
    let status = axum::http::StatusCode::from_u16(e.http_status()).unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    let body = sonic_rs::to_string(&ApiResponse::<()>::err(e)).unwrap_or_default();
    (status, [("content-type", "application/json")], body).into_response()
}

// --- Sandbox CRUD ---

async fn create(State(r): State<Reg>, axum::Json(req): axum::Json<CreateSandboxRequest>) -> Response {
    use crate::mock::SandboxManager;
    match r.create(req).await {
        Ok(sb) => json_created(sb),
        Err(e) => json_err(e),
    }
}

async fn list(State(r): State<Reg>, Query(f): Query<ListFilter>) -> Response {
    use crate::mock::SandboxManager;
    match r.list(f).await {
        Ok(v) => json_ok(v),
        Err(e) => json_err(e),
    }
}

async fn info(State(r): State<Reg>, Path(id): Path<String>) -> Response {
    use crate::mock::SandboxManager;
    match r.info(&id).await {
        Ok(v) => json_ok(v),
        Err(e) => json_err(e),
    }
}

async fn start(State(r): State<Reg>, Path(id): Path<String>) -> Response {
    use crate::mock::SandboxManager;
    match r.start(&id).await { Ok(()) => json_ok(sonic_rs::json!({})), Err(e) => json_err(e) }
}

async fn pause(State(r): State<Reg>, Path(id): Path<String>) -> Response {
    use crate::mock::SandboxManager;
    match r.pause(&id).await { Ok(()) => json_ok(sonic_rs::json!({})), Err(e) => json_err(e) }
}

async fn resume(State(r): State<Reg>, Path(id): Path<String>) -> Response {
    use crate::mock::SandboxManager;
    match r.resume(&id).await { Ok(()) => json_ok(sonic_rs::json!({})), Err(e) => json_err(e) }
}

async fn stop(State(r): State<Reg>, Path(id): Path<String>, body: Option<axum::Json<StopBody>>) -> Response {
    use crate::mock::SandboxManager;
    let force = body.map(|b| b.force).unwrap_or(false);
    match r.stop(&id, force).await { Ok(()) => json_ok(sonic_rs::json!({})), Err(e) => json_err(e) }
}

async fn destroy(State(r): State<Reg>, Path(id): Path<String>, Query(opts): Query<DestroyOptions>) -> Response {
    use crate::mock::SandboxManager;
    match r.destroy(&id, opts).await { Ok(()) => json_ok(sonic_rs::json!({})), Err(e) => json_err(e) }
}

#[derive(serde::Deserialize)]
struct StopBody { #[serde(default)] force: bool }

// --- Exec ---

async fn exec_run(State(r): State<Reg>, Path(id): Path<String>, axum::Json(mut body): axum::Json<ExecRequest>) -> Response {
    use crate::mock::ExecManager;
    body.sandbox_id = id;
    match r.run(body).await {
        Ok(v) => json_ok(v),
        Err(e) => json_err(e),
    }
}

async fn shell(State(_r): State<Reg>, Path(id): Path<String>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| handle_shell(socket, id))
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

// --- File ---

async fn file_read(State(r): State<Reg>, Path(id): Path<String>, Query(q): Query<FileReadQuery>) -> Response {
    use crate::mock::FileManager;
    match r.read(&id, &q.path, q.offset, q.limit).await {
        Ok(v) => json_ok(v),
        Err(e) => json_err(e),
    }
}

async fn file_write(State(r): State<Reg>, Path(id): Path<String>, axum::Json(body): axum::Json<FileWriteRequest>) -> Response {
    use crate::mock::FileManager;
    match r.write(&id, &body.path, body.content.as_bytes(), body.mode.as_deref()).await {
        Ok(()) => json_ok(sonic_rs::json!({})),
        Err(e) => json_err(e),
    }
}

async fn file_list(State(r): State<Reg>, Path(id): Path<String>, Query(q): Query<FileListQuery>) -> Response {
    use crate::mock::FileManager;
    match r.list(&id, &q.path, q.recursive.unwrap_or(false)).await {
        Ok(v) => json_ok(v),
        Err(e) => json_err(e),
    }
}

async fn file_upload(State(r): State<Reg>, Path(id): Path<String>, mut multipart: Multipart) -> Response {
    use crate::mock::FileManager;
    let mut dest = String::new();
    let mut data = bytes::Bytes::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "dest" => dest = field.text().await.unwrap_or_default(),
            "file" => data = field.bytes().await.unwrap_or_default(),
            _ => {}
        }
    }
    if dest.is_empty() { return json_err(ApiError::invalid_argument("missing 'dest' field")); }
    match r.upload(&id, &dest, data).await {
        Ok(()) => json_ok(sonic_rs::json!({})),
        Err(e) => json_err(e),
    }
}

async fn file_download(State(r): State<Reg>, Path(id): Path<String>, Query(q): Query<FileDownloadQuery>) -> Response {
    use crate::mock::FileManager;
    match r.download(&id, &q.path).await {
        Ok(data) => (axum::http::StatusCode::OK, [("content-type", "application/octet-stream")], data).into_response(),
        Err(e) => json_err(e),
    }
}

#[derive(serde::Deserialize)]
struct FileReadQuery { path: String, #[serde(default)] offset: Option<u64>, #[serde(default)] limit: Option<u64> }
#[derive(serde::Deserialize)]
struct FileListQuery { path: String, #[serde(default)] recursive: Option<bool> }
#[derive(serde::Deserialize)]
struct FileDownloadQuery { path: String }

// --- Snapshot ---

async fn snap_save(State(r): State<Reg>, Path(id): Path<String>, axum::Json(body): axum::Json<SnapSaveBody>) -> Response {
    use crate::mock::SnapshotManager;
    match r.save(&id, body.name.as_deref(), body.description.as_deref()).await {
        Ok(v) => json_created(v),
        Err(e) => json_err(e),
    }
}

async fn snap_list(State(r): State<Reg>, Path(id): Path<String>) -> Response {
    use crate::mock::SnapshotManager;
    match r.list(&id).await {
        Ok(v) => json_ok(v),
        Err(e) => json_err(e),
    }
}

async fn snap_restore(State(r): State<Reg>, Path((id, snap_id)): Path<(String, String)>) -> Response {
    use crate::mock::SnapshotManager;
    match r.restore(&id, &snap_id).await { Ok(()) => json_ok(sonic_rs::json!({})), Err(e) => json_err(e) }
}

async fn snap_delete(State(r): State<Reg>, Path((id, snap_id)): Path<(String, String)>) -> Response {
    use crate::mock::SnapshotManager;
    match r.delete(&id, &snap_id).await { Ok(()) => json_ok(sonic_rs::json!({})), Err(e) => json_err(e) }
}

async fn snap_diff(State(r): State<Reg>, Path(id): Path<String>, Query(q): Query<DiffQuery>) -> Response {
    use crate::mock::SnapshotManager;
    match r.diff(&id, &q.from, &q.to).await {
        Ok(v) => json_ok(v),
        Err(e) => json_err(e),
    }
}

#[derive(serde::Deserialize)]
struct SnapSaveBody { #[serde(default)] name: Option<String>, #[serde(default)] description: Option<String> }
#[derive(serde::Deserialize)]
struct DiffQuery { from: String, to: String }

// --- Resource ---

async fn resource_usage(State(r): State<Reg>, Path(id): Path<String>) -> Response {
    use crate::mock::ResourceManager;
    match r.usage(&id).await {
        Ok(v) => json_ok(v),
        Err(e) => json_err(e),
    }
}

async fn resource_resize(State(r): State<Reg>, Path(id): Path<String>, axum::Json(mut body): axum::Json<ResourceResizeRequest>) -> Response {
    use crate::mock::ResourceManager;
    body.sandbox_id = id;
    match r.resize(body).await { Ok(()) => json_ok(sonic_rs::json!({})), Err(e) => json_err(e) }
}
