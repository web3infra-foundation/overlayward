/// Temporary HTTP/3 client test — validates the QUIC endpoint is working.
/// Run: cargo run --bin h3-test
use std::sync::Arc;
use bytes::Buf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("crypto provider");

    // Client TLS config — skip cert verification for self-signed
    let mut tls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureVerifier))
        .with_no_client_auth();
    tls_config.alpn_protocols = vec![b"h3".to_vec()];

    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?,
    ));

    let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);

    println!("[1/3] Connecting to localhost:8425 via QUIC...");
    let conn = endpoint
        .connect("127.0.0.1:8425".parse()?, "localhost")?
        .await?;
    println!("[1/3] ✓ QUIC connection established");

    println!("[2/3] Opening HTTP/3 session...");
    let (mut driver, mut send_request) = h3::client::new(h3_quinn::Connection::new(conn)).await?;

    // Drive the connection in background
    tokio::spawn(async move {
        let _ = std::future::poll_fn(|cx| driver.poll_close(cx)).await;
    });

    // Request 1: GET /healthz
    println!("[2/3] ✓ HTTP/3 session ready");
    println!("[3/3] Sending GET /healthz...");

    let req = http::Request::builder()
        .method("GET")
        .uri("https://localhost/healthz")
        .body(())?;

    let mut stream = send_request.send_request(req).await?;
    stream.finish().await?;

    let resp = stream.recv_response().await?;
    println!("  Status: {}", resp.status());

    let mut body = Vec::new();
    while let Some(chunk) = stream.recv_data().await? {
        body.extend_from_slice(&chunk.chunk());
    }
    let body_str = String::from_utf8_lossy(&body);
    println!("  Body: {body_str}");

    if resp.status() == 200 && body_str.contains("ow-gateway") {
        println!("\n✓ HTTP/3 healthz — PASS");
    } else {
        println!("\n✗ HTTP/3 healthz — FAIL");
    }

    // Request 2: GET /api/v1/sandboxes (with auth)
    println!("\n[bonus] Sending GET /api/v1/sandboxes with auth...");
    let req2 = http::Request::builder()
        .method("GET")
        .uri("https://localhost/api/v1/sandboxes")
        .header("authorization", "Bearer ow-agent-token")
        .body(())?;

    let mut stream2 = send_request.send_request(req2).await?;
    stream2.finish().await?;

    let resp2 = stream2.recv_response().await?;
    println!("  Status: {}", resp2.status());

    let mut body2 = Vec::new();
    while let Some(chunk) = stream2.recv_data().await? {
        body2.extend_from_slice(&chunk.chunk());
    }
    let body2_str = String::from_utf8_lossy(&body2);
    println!("  Body: {body2_str}");

    if resp2.status() == 200 && body2_str.contains("\"ok\":true") {
        println!("\n✓ HTTP/3 auth API — PASS");
    } else {
        println!("\n✗ HTTP/3 auth API — FAIL");
    }

    // Request 3: No auth (should 401)
    println!("\n[bonus] Sending GET /api/v1/sandboxes WITHOUT auth...");
    let req3 = http::Request::builder()
        .method("GET")
        .uri("https://localhost/api/v1/sandboxes")
        .body(())?;

    let mut stream3 = send_request.send_request(req3).await?;
    stream3.finish().await?;

    let resp3 = stream3.recv_response().await?;
    println!("  Status: {}", resp3.status());

    let mut body3 = Vec::new();
    while let Some(chunk) = stream3.recv_data().await? {
        body3.extend_from_slice(&chunk.chunk());
    }
    println!("  Body: {}", String::from_utf8_lossy(&body3));

    if resp3.status() == 401 {
        println!("\n✓ HTTP/3 no-auth rejection — PASS");
    } else {
        println!("\n✗ HTTP/3 no-auth rejection — FAIL");
    }

    println!("\n=== All HTTP/3 tests done ===");
    Ok(())
}

/// Dangerous: skip certificate verification (for self-signed dev certs)
#[derive(Debug)]
struct InsecureVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self, _: &rustls::pki_types::CertificateDer<'_>, _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>, _: &[u8], _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self, _: &[u8], _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self, _: &[u8], _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
