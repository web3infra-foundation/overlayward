use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecRequest {
    #[serde(default)]
    pub sandbox_id: String,
    pub command: String,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub timeout: Option<String>,
    #[serde(default)]
    pub stdin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellRequest {
    pub sandbox_id: String,
    #[serde(default = "default_shell")]
    pub shell: Box<str>,
}

#[inline(always)]
fn default_shell() -> Box<str> { "/bin/bash".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ShellMessage {
    #[serde(rename = "stdin")]
    Stdin { data: String },
    #[serde(rename = "stdout")]
    Stdout { data: String },
    #[serde(rename = "resize")]
    Resize { cols: u32, rows: u32 },
}
