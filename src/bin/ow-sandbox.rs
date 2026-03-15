use clap::{Parser, Subcommand, ValueEnum};
use ow_cli::commands::*;
use ow_cli::client::HttpClient;
use ow_cli::output;

#[derive(Parser)]
#[command(name = "ow-sandbox", version, about = "Overlayward Sandbox Engine — minimal deployment")]
struct Cli {
    #[arg(long, env = "OW_SANDBOX_ENDPOINT", default_value = "http://localhost:8422")]
    endpoint: String,
    #[arg(long, default_value = "text")]
    output: Format,
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format { Text, Json }

#[derive(Subcommand)]
enum Cmd {
    /// Start sandbox engine server
    Serve {
        #[arg(long, default_value = "8422")]
        port: u16,
    },
    Create(CreateArgs),
    Start { sandbox_id: String },
    Stop { sandbox_id: String, #[arg(long)] force: bool },
    Pause { sandbox_id: String },
    Resume { sandbox_id: String },
    Destroy(DestroyArgs),
    List(ListArgs),
    Info { sandbox_id: String },
    Exec(ExecArgs),
    #[command(subcommand)]
    Snapshot(SnapshotCmd),
    #[command(subcommand)]
    File(FileCmd),
    #[command(subcommand)]
    Resource(ResourceCmd),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        None | Some(Cmd::Serve { .. }) => {
            let port = match cli.command {
                Some(Cmd::Serve { port }) => port,
                _ => 8422,
            };
            ow_sandbox::SandboxService::new()
                .with_port(port)
                .run()
                .await
                .expect("ow-sandbox failed");
        }
        Some(cmd) => {
            let c = HttpClient::new(&cli.endpoint, "");
            let fmt = match cli.output {
                Format::Text => ow_cli::OutputFormat::Text,
                Format::Json => ow_cli::OutputFormat::Json,
            };
            let code = match run_cmd(cmd, &c, fmt).await {
                Ok(()) => 0,
                Err(e) => { eprintln!("Error: {e}"); 1 }
            };
            std::process::exit(code);
        }
    }
}

async fn run_cmd(cmd: Cmd, c: &HttpClient, fmt: ow_cli::OutputFormat) -> Result<(), String> {
    match cmd {
        Cmd::Serve { .. } => unreachable!(),
        Cmd::Create(a) => { let r: serde_json::Value = c.post("/sandboxes", &a.to_body()).await?; output::print(fmt, &r); }
        Cmd::Start { sandbox_id } => { c.post_empty(&format!("/sandboxes/{sandbox_id}/start")).await?; output::msg(fmt, "started"); }
        Cmd::Stop { sandbox_id, force } => { c.post(&format!("/sandboxes/{sandbox_id}/stop"), &serde_json::json!({"force": force})).await.map(|_: serde_json::Value| ())?; output::msg(fmt, "stopped"); }
        Cmd::Pause { sandbox_id } => { c.post_empty(&format!("/sandboxes/{sandbox_id}/pause")).await?; output::msg(fmt, "paused"); }
        Cmd::Resume { sandbox_id } => { c.post_empty(&format!("/sandboxes/{sandbox_id}/resume")).await?; output::msg(fmt, "resumed"); }
        Cmd::Destroy(a) => { c.delete(&format!("/sandboxes/{}?keep_audit_logs={}", a.sandbox_id, a.keep_audit_logs)).await?; output::msg(fmt, "destroyed"); }
        Cmd::List(a) => { let q = a.status.as_deref().map(|s| format!("?status={s}")).unwrap_or_default(); let r: serde_json::Value = c.get(&format!("/sandboxes{q}")).await?; output::print(fmt, &r); }
        Cmd::Info { sandbox_id } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}")).await?; output::print(fmt, &r); }
        Cmd::Exec(a) => { let r: serde_json::Value = c.post(&format!("/sandboxes/{}/exec", a.sandbox_id), &serde_json::json!({"command": a.command.join(" "), "workdir": a.workdir, "timeout": a.timeout})).await?; output::print(fmt, &r); }
        Cmd::Snapshot(sub) => match sub {
            SnapshotCmd::Save(a) => { let r: serde_json::Value = c.post(&format!("/sandboxes/{}/snapshots", a.sandbox_id), &serde_json::json!({"name": a.name, "description": a.description})).await?; output::print(fmt, &r); }
            SnapshotCmd::List { sandbox_id } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}/snapshots")).await?; output::print(fmt, &r); }
            SnapshotCmd::Restore { sandbox_id, snapshot_id } => { c.post_empty(&format!("/sandboxes/{sandbox_id}/snapshots/{snapshot_id}/restore")).await?; output::msg(fmt, "restored"); }
            SnapshotCmd::Delete { sandbox_id, snapshot_id } => { c.delete(&format!("/sandboxes/{sandbox_id}/snapshots/{snapshot_id}")).await?; output::msg(fmt, "deleted"); }
            SnapshotCmd::Diff(a) => { let r: serde_json::Value = c.get(&format!("/sandboxes/{}/snapshots/diff?from={}&to={}", a.sandbox_id, a.from, a.to)).await?; output::print(fmt, &r); }
        },
        Cmd::File(sub) => match sub {
            FileCmd::Read { sandbox_id, path } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}/files?path={path}")).await?; output::print(fmt, &r); }
            FileCmd::Write { sandbox_id, path, source } => { let content = std::fs::read_to_string(&source).map_err(|e| e.to_string())?; let _: serde_json::Value = c.put(&format!("/sandboxes/{sandbox_id}/files"), &serde_json::json!({"path": path, "content": content})).await?; output::msg(fmt, "written"); }
            FileCmd::List(a) => { let r: serde_json::Value = c.get(&format!("/sandboxes/{}/files/list?path={}&recursive={}", a.sandbox_id, a.path, a.recursive)).await?; output::print(fmt, &r); }
            FileCmd::Upload(a) => { eprintln!("upload not supported in minimal mode for {}", a.guest_path); }
            FileCmd::Download { sandbox_id, guest_path, .. } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}/files/download?path={guest_path}")).await?; output::print(fmt, &r); }
        },
        Cmd::Resource(sub) => match sub {
            ResourceCmd::Usage { sandbox_id } => { let r: serde_json::Value = c.get(&format!("/sandboxes/{sandbox_id}/resources")).await?; output::print(fmt, &r); }
            ResourceCmd::Resize(a) => { let _: serde_json::Value = c.patch(&format!("/sandboxes/{}/resources", a.sandbox_id), &serde_json::json!({"cpu": a.cpu, "memory": a.memory, "disk": a.disk})).await?; output::msg(fmt, "resized"); }
        },
    }
    Ok(())
}
