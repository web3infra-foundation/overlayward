use axum::{extract::rejection::JsonRejection, http::StatusCode, response::{IntoResponse, Response}};
use ow_types::{ApiError, ApiResponse, CallerIdentity};

pub struct Caller(pub CallerIdentity);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for Caller {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, _s: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CallerIdentity>()
            .cloned()
            .map(Caller)
            .ok_or_else(|| {
                let body = sonic_rs::to_string(&ApiResponse::<()>::err(
                    ApiError::new("UNAUTHORIZED", "no caller identity"),
                )).unwrap_or_default();
                (StatusCode::UNAUTHORIZED, [("content-type","application/json")], body).into_response()
            })
    }
}

pub struct Json<T>(pub T);

impl<S, T> axum::extract::FromRequest<S> for Json<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(v)) => Ok(Json(v)),
            Err(rejection) => {
                let msg = match rejection {
                    JsonRejection::JsonDataError(e) => format!("invalid JSON data: {e}"),
                    JsonRejection::JsonSyntaxError(e) => format!("invalid JSON syntax: {e}"),
                    JsonRejection::MissingJsonContentType(e) => format!("missing content-type: {e}"),
                    _ => "invalid request body".into(),
                };
                let body = sonic_rs::to_string(&ApiResponse::<()>::err(
                    ApiError::invalid_argument(msg),
                )).unwrap_or_default();
                Err((StatusCode::BAD_REQUEST, [("content-type","application/json")], body).into_response())
            }
        }
    }
}
