use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterConnection {
    pub sandbox_a: Box<str>,
    pub sandbox_b: Box<str>,
    #[serde(default = "default_message_mode")]
    pub mode: Box<str>,
    #[serde(default = "ret_true")]
    pub bidirectional: bool,
}

#[inline(always)]
fn default_message_mode() -> Box<str> { "message".into() }
#[inline(always)]
const fn ret_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterConnectRequest {
    pub sandbox_a: String,
    pub sandbox_b: String,
    #[serde(default = "default_message_mode")]
    pub mode: Box<str>,
    #[serde(default = "ret_true")]
    pub bidirectional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterDisconnectRequest {
    pub sandbox_a: String,
    pub sandbox_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterMessage {
    pub from_sandbox: String,
    pub to_sandbox: String,
    pub message: String,
}
