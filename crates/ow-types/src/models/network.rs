use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    #[serde(default = "default_deny")]
    pub default_action: Box<str>,
    #[serde(default)]
    pub rules: Vec<NetworkRule>,
}

#[inline(always)]
fn default_deny() -> Box<str> { "deny".into() }

impl Default for NetworkPolicy {
    #[inline]
    fn default() -> Self {
        Self { default_action: default_deny(), rules: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRule {
    pub rule_id: Box<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<Box<str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cidr: Option<Box<str>>,
    #[serde(default, skip_serializing_if = "SmallVec::is_empty")]
    pub ports: SmallVec<[u16; 4]>,
    #[serde(default = "default_tcp")]
    pub protocol: Box<str>,
}

#[inline(always)]
fn default_tcp() -> Box<str> { "tcp".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicyConfig {
    #[serde(default = "default_deny", rename = "default")]
    pub default_action: Box<str>,
    #[serde(default)]
    pub allow: Vec<NetworkRuleSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRuleSpec {
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub cidr: Option<String>,
    #[serde(default)]
    pub ports: SmallVec<[u16; 4]>,
    #[serde(default = "default_tcp")]
    pub protocol: Box<str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddNetworkRuleRequest {
    #[serde(default)]
    pub sandbox_id: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub cidr: Option<String>,
    #[serde(default)]
    pub ports: SmallVec<[u16; 4]>,
    #[serde(default = "default_tcp")]
    pub protocol: Box<str>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AddRuleResult {
    Allowed { rule_id: Box<str> },
    ApprovalRequired {
        approval_id: Box<str>,
        status: Box<str>,
        timeout: Box<str>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDefaultNetworkRequest {
    pub sandbox_id: String,
    pub default: String,
}
