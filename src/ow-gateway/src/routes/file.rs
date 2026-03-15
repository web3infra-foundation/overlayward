use axum::{extract::{Multipart, Path, Query, State}, response::{IntoResponse, Response}, routing::{get, post}, Router};
use crate::registry::ServiceRegistry;
use ow_types::*;
use std::sync::Arc;
use crate::error::*;
use crate::extract::*;

pub fn routes() -> Router<Arc<ServiceRegistry>> {
    Router::new()
        .route("/sandboxes/{id}/files", get(read).put(write))
        .route("/sandboxes/{id}/files/list", get(list))
        .route("/sandboxes/{id}/files/upload", post(upload))
        .route("/sandboxes/{id}/files/download", get(download))
}

async fn read(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Query(q): Query<FileReadQuery>) -> Result<Response, AppError> {
    r.guardian.check("file.read", &sonic_rs::json!({}), &c).await?;
    Ok(json_ok(r.file.read(&id, &q.path, q.offset, q.limit).await?))
}

async fn write(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Json(body): Json<FileWriteRequest>) -> Result<Response, AppError> {
    r.guardian.check("file.write", &sonic_rs::json!({}), &c).await?;
    r.file.write(&id, &body.path, body.content.as_bytes(), body.mode.as_deref()).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn list(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Query(q): Query<FileListQuery>) -> Result<Response, AppError> {
    r.guardian.check("file.list", &sonic_rs::json!({}), &c).await?;
    Ok(json_ok(r.file.list(&id, &q.path, q.recursive.unwrap_or(false)).await?))
}

async fn upload(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, mut multipart: Multipart) -> Result<Response, AppError> {
    r.guardian.check("file.upload", &sonic_rs::json!({}), &c).await?;
    let mut dest = String::new();
    let mut data = bytes::Bytes::new();
    while let Some(field) = multipart.next_field().await.map_err(|e| ApiError::invalid_argument(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "dest" => dest = field.text().await.unwrap_or_default(),
            "file" => data = field.bytes().await.unwrap_or_default(),
            _ => {}
        }
    }
    if dest.is_empty() { return Err(ApiError::invalid_argument("missing 'dest' field").into()); }
    r.file.upload(&id, &dest, data).await?;
    Ok(json_ok(sonic_rs::json!({})))
}

async fn download(State(r): State<Arc<ServiceRegistry>>, Caller(c): Caller, Path(id): Path<String>, Query(q): Query<FileDownloadQuery>) -> Result<Response, AppError> {
    r.guardian.check("file.download", &sonic_rs::json!({}), &c).await?;
    let data = r.file.download(&id, &q.path).await?;
    Ok((axum::http::StatusCode::OK, [("content-type", "application/octet-stream")], data).into_response())
}

#[derive(serde::Deserialize)]
struct FileReadQuery { path: String, #[serde(default)] offset: Option<u64>, #[serde(default)] limit: Option<u64> }
#[derive(serde::Deserialize)]
struct FileListQuery { path: String, #[serde(default)] recursive: Option<bool> }
#[derive(serde::Deserialize)]
struct FileDownloadQuery { path: String }
