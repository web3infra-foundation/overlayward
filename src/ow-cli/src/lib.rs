pub mod commands;
pub mod client;
pub mod output;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "overlayward", version, about = "Overlayward Sandbox Manager CLI")]
pub struct Cli {
    #[arg(long, env = "OVERLAYWARD_ENDPOINT", default_value = "http://localhost:8420")]
    pub endpoint: String,
    #[arg(long, env = "OVERLAYWARD_TOKEN", default_value = "")]
    pub token: String,
    #[arg(long, default_value = "text", env = "OVERLAYWARD_OUTPUT")]
    pub output: OutputFormat,
    #[arg(long)]
    pub quiet: bool,
    #[arg(long)]
    pub verbose: bool,
    /// Direct mode: connect to ow-sandbox on :8422 without auth
    #[arg(long, env = "OVERLAYWARD_DIRECT")]
    pub direct: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum OutputFormat { Text, Json }

#[derive(Subcommand)]
pub enum Commands {
    Serve(commands::ServeArgs),
    McpServer,
    Create(commands::CreateArgs),
    Start { sandbox_id: String },
    Stop { sandbox_id: String, #[arg(long)] force: bool },
    Pause { sandbox_id: String },
    Resume { sandbox_id: String },
    Destroy(commands::DestroyArgs),
    List(commands::ListArgs),
    Info { sandbox_id: String },
    Shell(commands::ShellArgs),
    Exec(commands::ExecArgs),
    #[command(subcommand)] Snapshot(commands::SnapshotCmd),
    #[command(subcommand)] Network(commands::NetworkCmd),
    #[command(subcommand)] File(commands::FileCmd),
    #[command(subcommand)] Volume(commands::VolumeCmd),
    #[command(subcommand)] Audit(commands::AuditCmd),
    #[command(subcommand)] Resource(commands::ResourceCmd),
    #[command(subcommand)] Inter(commands::InterCmd),
    #[command(subcommand)] Approval(commands::ApprovalCmd),
}

pub async fn run(cli: Cli) -> i32 {
    let (endpoint, token) = if cli.direct {
        ("http://localhost:8422".to_owned(), String::new())
    } else {
        (cli.endpoint, cli.token)
    };

    // In direct mode, reject commands that require full deployment
    if cli.direct {
        if let Some(msg) = direct_unsupported(&cli.command) {
            eprintln!("Error: {msg}");
            return 1;
        }
    }

    let c = client::HttpClient::new(&endpoint, &token);
    let fmt = cli.output;
    match execute(cli.command, &c, fmt).await {
        Ok(()) => 0,
        Err(e) => { eprintln!("Error: {e}"); exit_code_from_error(&e) }
    }
}

const DIRECT_ERR: &str = "\u{6b64}\u{547d}\u{4ee4}\u{9700}\u{8981}\u{5b8c}\u{6574}\u{90e8}\u{7f72}\u{6a21}\u{5f0f} (overlayward serve)";

fn direct_unsupported(cmd: &Commands) -> Option<&'static str> {
    match cmd {
        Commands::Audit(_)
        | Commands::Approval(_)
        | Commands::Network(_)
        | Commands::Volume(_)
        | Commands::Inter(_)
        | Commands::Shell(_) => Some(DIRECT_ERR),
        _ => None,
    }
}

async fn execute(cmd: Commands, c: &client::HttpClient, fmt: OutputFormat) -> Result<(), String> {
    use commands::*;
    match cmd {
        Commands::Serve(_) | Commands::McpServer => { unreachable!("handled in main.rs") }
        Commands::Create(a) => { let r: serde_json::Value = c.post("/sandboxes", &a.to_body()).await?; output::print(fmt, &r); }
        Commands::Start { sandbox_id } => { c.post_empty(&format!("/sandboxes/{sandbox_id}/start")).await?; output::msg(fmt, "started"); }
        Commands::Stop { sandbox_id, force } => { c.post(&format!("/sandboxes/{sandbox_id}/stop"), &serde_json::json!({"force": force})).await.map(|_: serde_json::Value| ())?; output::msg(fmt, "stopped"); }
        Commands::Pause { sandbox_id } => { c.post_empty(&format!("/sandboxes/{sandbox_id}/pause")).await?; output::msg(fmt, "paused"); }
        Commands::Resume { sandbox_id } => { c.post_empty(&format!("/sandboxes/{sandbox_id}/resume")).await?; output::msg(fmt, "resumed"); }
        Commands::Destroy(a) => { c.delete(&format!("/sandboxes/{}?keep_audit_logs={}", a.sandbox_id, a.keep_audit_logs)).await?; output::msg(fmt, "destroyed"); }
        Commands::List(a) => { let q = a.status.as_deref().map(|s| format!("?status={s}")).unwrap_or_default(); let r: serde_json::Value = c.get(&format!("/sandboxes{q}")).await?; output::print(fmt, &r); }
        Commands::Info { sandbox_id } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}")).await?; output::print(fmt, &r); }
        Commands::Shell(_a) => { eprintln!("Interactive shell requires WebSocket (use REST API directly)"); }
        Commands::Exec(a) => { let r: serde_json::Value = c.post(&format!("/sandboxes/{}/exec", a.sandbox_id), &serde_json::json!({"command": a.command.join(" "), "workdir": a.workdir, "timeout": a.timeout})).await?; output::print(fmt, &r); }
        Commands::Snapshot(sub) => match sub {
            SnapshotCmd::Save(a) => { let r: serde_json::Value = c.post(&format!("/sandboxes/{}/snapshots", a.sandbox_id), &serde_json::json!({"name": a.name, "description": a.description})).await?; output::print(fmt, &r); }
            SnapshotCmd::List { sandbox_id } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}/snapshots")).await?; output::print(fmt, &r); }
            SnapshotCmd::Restore { sandbox_id, snapshot_id } => { c.post_empty(&format!("/sandboxes/{sandbox_id}/snapshots/{snapshot_id}/restore")).await?; output::msg(fmt, "restored"); }
            SnapshotCmd::Delete { sandbox_id, snapshot_id } => { c.delete(&format!("/sandboxes/{sandbox_id}/snapshots/{snapshot_id}")).await?; output::msg(fmt, "deleted"); }
            SnapshotCmd::Diff(a) => { let r: serde_json::Value = c.get(&format!("/sandboxes/{}/snapshots/diff?from={}&to={}", a.sandbox_id, a.from, a.to)).await?; output::print(fmt, &r); }
        },
        Commands::Network(sub) => match sub {
            NetworkCmd::Get { sandbox_id } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}/network")).await?; output::print(fmt, &r); }
            NetworkCmd::Allow(a) => { let r: serde_json::Value = c.post(&format!("/sandboxes/{}/network/rules", a.sandbox_id), &serde_json::json!({"domain": a.domain, "cidr": a.cidr, "ports": a.ports, "reason": a.reason})).await?; output::print(fmt, &r); }
            NetworkCmd::Deny { sandbox_id, rule } => { c.delete(&format!("/sandboxes/{sandbox_id}/network/rules/{rule}")).await?; output::msg(fmt, "rule removed"); }
        },
        Commands::File(sub) => match sub {
            FileCmd::Read { sandbox_id, path } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}/files?path={path}")).await?; output::print(fmt, &r); }
            FileCmd::Write { sandbox_id, path, source } => { let content = std::fs::read_to_string(&source).map_err(|e| e.to_string())?; let _: serde_json::Value = c.put(&format!("/sandboxes/{sandbox_id}/files"), &serde_json::json!({"path": path, "content": content})).await?; output::msg(fmt, "written"); }
            FileCmd::List(a) => { let r: serde_json::Value = c.get(&format!("/sandboxes/{}/files/list?path={}&recursive={}", a.sandbox_id, a.path, a.recursive)).await?; output::print(fmt, &r); }
            FileCmd::Upload(a) => { eprintln!("Upload requires multipart (use REST API directly for {}", a.guest_path); }
            FileCmd::Download { sandbox_id, guest_path, .. } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}/files/download?path={guest_path}")).await?; output::print(fmt, &r); }
        },
        Commands::Volume(sub) => match sub {
            VolumeCmd::Mount(a) => { let _: serde_json::Value = c.post(&format!("/sandboxes/{}/volumes", a.sandbox_id), &serde_json::json!({"host_path": a.host, "guest_path": a.guest, "mode": a.mode})).await?; output::msg(fmt, "mounted"); }
            VolumeCmd::Unmount { sandbox_id, path } => { let _: serde_json::Value = c.delete_body(&format!("/sandboxes/{sandbox_id}/volumes"), &serde_json::json!({"guest_path": path})).await?; output::msg(fmt, "unmounted"); }
            VolumeCmd::List { sandbox_id } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}/volumes")).await?; output::print(fmt, &r); }
        },
        Commands::Audit(sub) => match sub {
            AuditCmd::Query(a) => { let r: serde_json::Value = c.get(&format!("/sandboxes/{}/audit?level={}&limit={}", a.sandbox_id, a.level.as_deref().unwrap_or(""), a.limit)).await?; output::print(fmt, &r); }
            AuditCmd::Detail { sandbox_id, event_id } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}/audit/{event_id}")).await?; output::print(fmt, &r); }
            AuditCmd::Replay(a) => { let r: serde_json::Value = c.post(&format!("/sandboxes/{}/audit/replay", a.sandbox_id), &serde_json::json!({"from": a.from, "to": a.to, "speed": a.speed})).await?; output::print(fmt, &r); }
        },
        Commands::Resource(sub) => match sub {
            ResourceCmd::Usage { sandbox_id } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}/resources")).await?; output::print(fmt, &r); }
            ResourceCmd::Resize(a) => { let _: serde_json::Value = c.patch(&format!("/sandboxes/{}/resources", a.sandbox_id), &serde_json::json!({"cpu": a.cpu, "memory": a.memory, "disk": a.disk})).await?; output::msg(fmt, "resized"); }
        },
        Commands::Inter(sub) => match sub {
            InterCmd::Connect(a) => { let _: serde_json::Value = c.post("/inter/connections", &serde_json::json!({"sandbox_a": a.sandbox_a, "sandbox_b": a.sandbox_b, "mode": a.mode})).await?; output::msg(fmt, "connected"); }
            InterCmd::Disconnect { sandbox_a, sandbox_b } => { let _: serde_json::Value = c.delete_body("/inter/connections", &serde_json::json!({"sandbox_a": sandbox_a, "sandbox_b": sandbox_b})).await?; output::msg(fmt, "disconnected"); }
            InterCmd::Send { from, to, message } => { let _: serde_json::Value = c.post("/inter/messages", &serde_json::json!({"from_sandbox": from, "to_sandbox": to, "message": message})).await?; output::msg(fmt, "sent"); }
        },
        Commands::Approval(sub) => match sub {
            ApprovalCmd::List { status } => { let q = status.map(|s| format!("?status={s}")).unwrap_or_default(); let r: serde_json::Value = c.get(&format!("/approvals{q}")).await?; output::print(fmt, &r); }
            ApprovalCmd::Decide(a) => { let _: serde_json::Value = c.post(&format!("/approvals/{}/decide", a.approval_id), &serde_json::json!({"decision": if a.approve { "approve" } else { "deny" }, "reason": a.reason, "permanent": a.permanent})).await?; output::msg(fmt, "decided"); }
        },
    }
    Ok(())
}

fn exit_code_from_error(msg: &str) -> i32 {
    if msg.contains("GUARDIAN") { 3 }
    else if msg.contains("APPROVAL") { 4 }
    else if msg.contains("RESOURCE") { 5 }
    else if msg.contains("PERMISSION") || msg.contains("403") { 6 }
    else if msg.contains("NOT_FOUND") || msg.contains("404") { 7 }
    else if msg.contains("connection") { 8 }
    else { 1 }
}
