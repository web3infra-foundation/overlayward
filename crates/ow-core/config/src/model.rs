use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContainerConfig {
    pub image: String,
    #[serde(default)]
    pub hostname: Option<String>,
    pub process: ProcessConfig,
    #[serde(default)]
    pub resources: Option<ResourceConfig>,
    #[serde(default)]
    pub network: Option<NetworkConfigDef>,
    #[serde(default)]
    pub security: Option<SecurityConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProcessConfig {
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default = "default_working_dir")]
    pub working_dir: String,
}

fn default_working_dir() -> String {
    "/".to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResourceConfig {
    pub cpu_quota_us: Option<u64>,
    pub cpu_period_us: Option<u64>,
    pub memory_max: Option<u64>,
    pub memory_swap_max: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NetworkConfigDef {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default)]
    pub readonly_rootfs: bool,
}
