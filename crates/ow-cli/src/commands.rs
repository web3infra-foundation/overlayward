use clap::{Args, Subcommand};

#[derive(Args)]
pub struct ServeArgs {
    #[arg(long, default_value = "8420")]
    pub rest_port: u16,
    #[arg(long, default_value = "8425")]
    pub grpc_port: u16,
    #[arg(long, default_value = "8426")]
    pub mcp_port: u16,
}

#[derive(Args)]
pub struct CreateArgs {
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long, default_value = "2")]
    pub cpu: u32,
    #[arg(long, default_value = "4GB")]
    pub memory: String,
    #[arg(long, default_value = "20GB")]
    pub disk: String,
    #[arg(long, default_value = "ubuntu:24.04")]
    pub image: String,
    #[arg(long)]
    pub gpu: Option<String>,
    #[arg(long, value_parser = parse_label)]
    pub label: Vec<(String, String)>,
}

fn parse_label(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.into(), v.into()))
        .ok_or_else(|| "format: KEY=VALUE".into())
}

impl CreateArgs {
    pub fn to_body(&self) -> serde_json::Value {
        let labels: std::collections::HashMap<&str, &str> = self
            .label
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        serde_json::json!({ "name": self.name, "cpu": self.cpu, "memory": self.memory, "disk": self.disk, "image": self.image, "labels": labels })
    }
}

#[derive(Args)]
pub struct DestroyArgs {
    pub sandbox_id: String,
    #[arg(long)]
    pub keep_snapshots: bool,
    #[arg(long, default_value = "true")]
    pub keep_audit_logs: bool,
    #[arg(long, short)]
    pub yes: bool,
}

#[derive(Args)]
pub struct ListArgs {
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub label: Vec<String>,
    #[arg(long)]
    pub all: bool,
}

#[derive(Args)]
pub struct ShellArgs {
    pub sandbox_id: String,
    #[arg(long, default_value = "/bin/bash")]
    pub shell: String,
}

#[derive(Args)]
pub struct ExecArgs {
    pub sandbox_id: String,
    #[arg(long)]
    pub workdir: Option<String>,
    #[arg(long)]
    pub timeout: Option<String>,
    #[arg(long, value_parser = parse_label)]
    pub env: Vec<(String, String)>,
    #[arg(last = true, required = true)]
    pub command: Vec<String>,
}

#[derive(Subcommand)]
pub enum SnapshotCmd {
    Save(SnapSaveArgs),
    List {
        sandbox_id: String,
    },
    Restore {
        sandbox_id: String,
        snapshot_id: String,
    },
    Delete {
        sandbox_id: String,
        snapshot_id: String,
    },
    Diff(SnapDiffArgs),
}

#[derive(Args)]
pub struct SnapSaveArgs {
    pub sandbox_id: String,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Args)]
pub struct SnapDiffArgs {
    pub sandbox_id: String,
    pub from: String,
    pub to: String,
}

#[derive(Subcommand)]
pub enum NetworkCmd {
    Get {
        sandbox_id: String,
    },
    Allow(NetAllowArgs),
    Deny {
        sandbox_id: String,
        #[arg(long)]
        rule: String,
    },
}

#[derive(Args)]
pub struct NetAllowArgs {
    pub sandbox_id: String,
    #[arg(long)]
    pub domain: Option<String>,
    #[arg(long)]
    pub cidr: Option<String>,
    #[arg(long, use_value_delimiter = true)]
    pub ports: Vec<u16>,
    #[arg(long)]
    pub reason: Option<String>,
}

#[derive(Subcommand)]
pub enum FileCmd {
    Read {
        sandbox_id: String,
        path: String,
    },
    Write {
        sandbox_id: String,
        path: String,
        source: String,
    },
    List(FileListArgs),
    Upload(FileUploadArgs),
    Download {
        sandbox_id: String,
        guest_path: String,
        #[arg(default_value = ".")]
        local_path: String,
    },
}

#[derive(Args)]
pub struct FileListArgs {
    pub sandbox_id: String,
    pub path: String,
    #[arg(long)]
    pub recursive: bool,
}

#[derive(Args)]
pub struct FileUploadArgs {
    pub sandbox_id: String,
    pub local_path: String,
    pub guest_path: String,
}

#[derive(Subcommand)]
pub enum VolumeCmd {
    Mount(VolMountArgs),
    Unmount { sandbox_id: String, path: String },
    List { sandbox_id: String },
}

#[derive(Args)]
pub struct VolMountArgs {
    pub sandbox_id: String,
    #[arg(long)]
    pub host: String,
    #[arg(long)]
    pub guest: String,
    #[arg(long, default_value = "ro")]
    pub mode: String,
}

#[derive(Subcommand)]
pub enum AuditCmd {
    Query(AuditQueryArgs),
    Detail {
        sandbox_id: String,
        event_id: String,
    },
    Replay(AuditReplayArgs),
}

#[derive(Args)]
pub struct AuditQueryArgs {
    pub sandbox_id: String,
    #[arg(long)]
    pub level: Option<String>,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long, default_value = "50")]
    pub limit: u32,
}

#[derive(Args)]
pub struct AuditReplayArgs {
    pub sandbox_id: String,
    #[arg(long)]
    pub from: String,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long, default_value = "1.0")]
    pub speed: f64,
}

#[derive(Subcommand)]
pub enum ResourceCmd {
    Usage { sandbox_id: String },
    Resize(ResizeArgs),
}

#[derive(Args)]
pub struct ResizeArgs {
    pub sandbox_id: String,
    #[arg(long)]
    pub cpu: Option<u32>,
    #[arg(long)]
    pub memory: Option<String>,
    #[arg(long)]
    pub disk: Option<String>,
}

#[derive(Subcommand)]
pub enum InterCmd {
    Connect(InterConnArgs),
    Disconnect {
        sandbox_a: String,
        sandbox_b: String,
    },
    Send {
        from: String,
        to: String,
        message: String,
    },
}

#[derive(Args)]
pub struct InterConnArgs {
    pub sandbox_a: String,
    pub sandbox_b: String,
    #[arg(long, default_value = "message")]
    pub mode: String,
}

#[derive(Subcommand)]
pub enum ApprovalCmd {
    List {
        #[arg(long)]
        status: Option<String>,
    },
    Decide(ApprovalDecideArgs),
}

#[derive(Args)]
pub struct ApprovalDecideArgs {
    pub approval_id: String,
    #[arg(long, group = "decision")]
    pub approve: bool,
    #[arg(long, group = "decision")]
    pub deny: bool,
    #[arg(long)]
    pub reason: Option<String>,
    #[arg(long)]
    pub permanent: bool,
}
