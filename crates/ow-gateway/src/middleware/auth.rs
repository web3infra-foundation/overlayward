use axum::{
    extract::{Extension, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use crate::auth::TokenResolver;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthState {
    pub resolver: Arc<dyn TokenResolver>,
}

pub async fn auth_middleware(
    Extension(state): Extension<AuthState>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match token {
        Some(t) => match state.resolver.resolve(t) {
            Some(identity) => {
                req.extensions_mut().insert(identity);
                next.run(req).await
            }
            None => err_response(StatusCode::UNAUTHORIZED, "invalid token"),
        },
        None => err_response(StatusCode::UNAUTHORIZED, "missing Authorization header"),
    }
}

#[inline]
fn err_response(status: StatusCode, msg: &str) -> Response {
    let body = sonic_rs::to_string(&ow_types::ApiResponse::<()>::err(
        ow_types::ApiError::new("UNAUTHORIZED", msg),
    )).unwrap_or_default();
    (status, [("content-type", "application/json")], body).into_response()
}
