use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ow_types::{ApiError, ApiResponse};

pub struct AppError(pub ApiError);

impl From<ApiError> for AppError {
    #[inline(always)]
    fn from(e: ApiError) -> Self { Self(e) }
}

impl IntoResponse for AppError {
    #[inline]
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = sonic_rs::to_string(&ApiResponse::<()>::err(self.0)).unwrap_or_default();
        (status, [("content-type", "application/json")], body).into_response()
    }
}

#[inline]
pub fn json_ok<T: serde::Serialize>(data: T) -> Response {
    let body = sonic_rs::to_string(&ApiResponse::ok(data)).unwrap_or_default();
    (StatusCode::OK, [("content-type", "application/json")], body).into_response()
}

#[inline]
pub fn json_created<T: serde::Serialize>(data: T) -> Response {
    let body = sonic_rs::to_string(&ApiResponse::ok(data)).unwrap_or_default();
    (StatusCode::CREATED, [("content-type", "application/json")], body).into_response()
}
