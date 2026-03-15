use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_error_codes {
    ($($variant:ident = $code:literal => (http: $http:literal, grpc: $grpc:literal));* $(;)?) => {
        pub mod codes {
            $(pub const $variant: &str = $code;)*
        }

        static HTTP_STATUS_MAP: phf::Map<&'static str, u16> = phf::phf_map! {
            $($code => $http),*
        };

        static GRPC_CODE_MAP: phf::Map<&'static str, i32> = phf::phf_map! {
            $($code => $grpc),*
        };

        #[inline(always)]
        pub fn error_to_http_status(code: &str) -> u16 {
            HTTP_STATUS_MAP.get(code).copied().unwrap_or(500)
        }

        #[inline(always)]
        pub fn error_to_grpc_code(code: &str) -> i32 {
            GRPC_CODE_MAP.get(code).copied().unwrap_or(13)
        }
    };
}

define_error_codes! {
    GUARDIAN_NETWORK_VIOLATION   = "GUARDIAN_NETWORK_VIOLATION"   => (http: 403, grpc: 7);
    GUARDIAN_FILESYSTEM_VIOLATION= "GUARDIAN_FILESYSTEM_VIOLATION"=> (http: 403, grpc: 7);
    GUARDIAN_CONFIG_VIOLATION    = "GUARDIAN_CONFIG_VIOLATION"    => (http: 403, grpc: 7);
    GUARDIAN_PERMISSION_DENIED   = "GUARDIAN_PERMISSION_DENIED"   => (http: 403, grpc: 7);
    NOT_FOUND_SANDBOX           = "NOT_FOUND_SANDBOX"            => (http: 404, grpc: 5);
    NOT_FOUND_SNAPSHOT          = "NOT_FOUND_SNAPSHOT"            => (http: 404, grpc: 5);
    NOT_FOUND_RULE              = "NOT_FOUND_RULE"               => (http: 404, grpc: 5);
    NOT_FOUND_APPROVAL          = "NOT_FOUND_APPROVAL"           => (http: 404, grpc: 5);
    NOT_FOUND_EVENT             = "NOT_FOUND_EVENT"              => (http: 404, grpc: 5);
    NOT_FOUND_CONNECTION        = "NOT_FOUND_CONNECTION"         => (http: 404, grpc: 5);
    INVALID_ARGUMENT            = "INVALID_ARGUMENT"             => (http: 400, grpc: 3);
    INVALID_STATUS_TRANSITION   = "INVALID_STATUS_TRANSITION"    => (http: 409, grpc: 9);
    RESOURCE_EXHAUSTED          = "RESOURCE_EXHAUSTED"           => (http: 503, grpc: 8);
    RESOURCE_QUOTA_EXCEEDED     = "RESOURCE_QUOTA_EXCEEDED"      => (http: 503, grpc: 8);
    INTERNAL_ERROR              = "INTERNAL_ERROR"               => (http: 500, grpc: 13);
    APPROVAL_REQUIRED           = "APPROVAL_REQUIRED"            => (http: 422, grpc: 9);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: Box<str>,
    pub message: Box<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<sonic_rs::Value>,
}

impl ApiError {
    #[inline]
    pub fn new(code: &str, message: impl Into<Box<str>>) -> Self {
        Self { code: code.into(), message: message.into(), detail: None }
    }

    #[inline]
    pub fn with_detail(mut self, detail: sonic_rs::Value) -> Self {
        self.detail = Some(detail);
        self
    }

    #[inline(always)]
    pub fn http_status(&self) -> u16 {
        error_to_http_status(&self.code)
    }

    #[inline(always)]
    pub fn grpc_code(&self) -> i32 {
        error_to_grpc_code(&self.code)
    }

    #[inline]
    pub fn not_found(resource: &str, id: &str) -> Self {
        let code = match resource {
            "sandbox" => codes::NOT_FOUND_SANDBOX,
            "snapshot" => codes::NOT_FOUND_SNAPSHOT,
            "rule" => codes::NOT_FOUND_RULE,
            "approval" => codes::NOT_FOUND_APPROVAL,
            "event" => codes::NOT_FOUND_EVENT,
            "connection" => codes::NOT_FOUND_CONNECTION,
            _ => codes::NOT_FOUND_SANDBOX,
        };
        Self::new(code, format!("{resource} '{id}' not found"))
    }

    #[inline]
    pub fn invalid_argument(msg: impl Into<Box<str>>) -> Self {
        Self::new(codes::INVALID_ARGUMENT, msg)
    }

    #[inline]
    pub fn permission_denied(msg: impl Into<Box<str>>) -> Self {
        Self::new(codes::GUARDIAN_PERMISSION_DENIED, msg)
    }

    #[inline]
    pub fn internal(msg: impl Into<Box<str>>) -> Self {
        Self::new(codes::INTERNAL_ERROR, msg)
    }

    #[inline]
    pub fn invalid_transition(from: &str, to: &str) -> Self {
        Self::new(
            codes::INVALID_STATUS_TRANSITION,
            format!("cannot transition from '{from}' to '{to}'"),
        )
    }

    #[inline]
    pub fn approval_required(approval_id: Box<str>, timeout: &str) -> Self {
        Self::new(codes::APPROVAL_REQUIRED, "此操作需要人类审批").with_detail(
            sonic_rs::json!({
                "approval_id": approval_id,
                "status": "pending",
                "timeout": timeout
            }),
        )
    }
}

impl fmt::Display for ApiError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}
