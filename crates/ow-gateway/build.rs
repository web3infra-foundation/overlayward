fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Pre-generated proto code is committed at src/proto/overlayward.v1.rs
    // To regenerate: REGEN_PROTO=1 cargo build -p ow-gateway
    if std::env::var("REGEN_PROTO").is_err() {
        return Ok(());
    }

    auto_detect_protoc();

    let protos = &[
        "../../proto/overlayward/v1/sandbox.proto",
        "../../proto/overlayward/v1/snapshot.proto",
        "../../proto/overlayward/v1/network.proto",
        "../../proto/overlayward/v1/exec.proto",
        "../../proto/overlayward/v1/file.proto",
        "../../proto/overlayward/v1/volume.proto",
        "../../proto/overlayward/v1/audit.proto",
        "../../proto/overlayward/v1/resource.proto",
        "../../proto/overlayward/v1/inter.proto",
        "../../proto/overlayward/v1/approval.proto",
        "../../proto/overlayward/v1/event.proto",
    ];
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto")
        .compile_protos(protos, &["../../proto"])?;
    Ok(())
}

fn auto_detect_protoc() {
    if std::env::var("PROTOC").is_ok() {
        return;
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let candidates = [
        format!("{home}/.local/protoc/bin/protoc.exe"),
        format!("{home}/.local/protoc/bin/protoc"),
        format!("{home}/.local/bin/protoc"),
        "/usr/bin/protoc".into(),
        "/usr/local/bin/protoc".into(),
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            std::env::set_var("PROTOC", path);
            return;
        }
    }
}
