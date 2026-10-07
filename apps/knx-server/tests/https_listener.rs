//! The TLS listener against real sockets and a real rustls client (ADR-0088).
//!
//! A trusted handshake, the plain-HTTP redirect on the same port, and the
//! property the accept/handshake split exists for: a client that connects
//! and says nothing does not stop anyone else.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

struct Server {
    addr: SocketAddr,
    trusted: CertificateDer<'static>,
    _data: tempfile::TempDir,
}

/// A server the way `main.rs` starts one with a password set and nothing
/// else: `KNX_TLS` unset, so a self-signed certificate under the data dir.
async fn start() -> Server {
    let data = tempfile::tempdir().unwrap();
    let decision = knx_server::resolve_tls(knx_server::TlsInputs {
        switch: knx_server::TlsSwitch::Auto,
        cert: None,
        key: None,
        extra_names: None,
        hostname: None,
        auth_required: true,
        data_dir: data.path(),
    })
    .unwrap();
    let prepared = knx_server::prepare_tls(&decision.plan, time::OffsetDateTime::now_utc())
        .unwrap()
        .expect("a password-protected server speaks HTTPS by default");
    let trusted =
        CertificateDer::from_pem_file(data.path().join(knx_server::TLS_DIR_NAME).join("cert.pem"))
            .unwrap();

    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let listener = knx_server::TlsListener::new(tcp, prepared.config).unwrap();
    let addr = axum::serve::Listener::local_addr(&listener).unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Server {
        addr,
        trusted,
        _data: data,
    }
}

fn client_trusting(cert: Option<&CertificateDer<'static>>) -> tokio_rustls::TlsConnector {
    let mut roots = rustls::RootCertStore::empty();
    if let Some(cert) = cert {
        roots.add(cert.clone()).unwrap();
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    tokio_rustls::TlsConnector::from(Arc::new(config))
}

/// One `GET` over HTTPS, returning the whole response as text.
async fn https_get(server: &Server, name: &str, path: &str) -> std::io::Result<String> {
    let tcp = TcpStream::connect(server.addr).await?;
    let mut tls = client_trusting(Some(&server.trusted))
        .connect(ServerName::try_from(name.to_string()).unwrap(), tcp)
        .await?;
    tls.write_all(
        format!("GET {path} HTTP/1.1\r\nHost: {name}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .await?;
    let mut response = String::new();
    tls.read_to_string(&mut response).await?;
    Ok(response)
}

#[tokio::test]
async fn a_client_trusting_the_generated_certificate_is_served_over_https() {
    let server = start().await;
    for name in ["localhost", "127.0.0.1"] {
        let response = https_get(&server, name, "/healthz").await.unwrap();
        assert!(response.starts_with("HTTP/1.1 200"), "{name}: {response}");
    }
}

#[tokio::test]
async fn a_name_the_certificate_does_not_cover_is_refused_by_the_client() {
    let server = start().await;
    let error = https_get(&server, "knx.example", "/healthz")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("not valid for name"), "{error}");
}

#[tokio::test]
async fn plain_http_on_the_same_port_is_redirected_to_https() {
    let server = start().await;
    let mut tcp = TcpStream::connect(server.addr).await.unwrap();
    let host = format!("127.0.0.1:{}", server.addr.port());
    tcp.write_all(format!("GET /api/version HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let mut response = String::new();
    tcp.read_to_string(&mut response).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 307 "), "{response}");
    assert!(
        response.contains(&format!("Location: https://{host}/api/version\r\n")),
        "{response}"
    );
}

#[tokio::test]
async fn a_silent_client_does_not_hold_up_anyone_else() {
    let server = start().await;
    // Connected, and then nothing: no ClientHello, no request line.
    let _silent = TcpStream::connect(server.addr).await.unwrap();
    let _also_silent = TcpStream::connect(server.addr).await.unwrap();
    let served = tokio::time::timeout(
        Duration::from_secs(3),
        https_get(&server, "localhost", "/healthz"),
    )
    .await
    .expect("served well before the silent clients' handshake timeout")
    .unwrap();
    assert!(served.starts_with("HTTP/1.1 200"), "{served}");
    assert!(knx_server::HANDSHAKE_TIMEOUT > Duration::from_secs(3));
}

#[tokio::test]
async fn a_client_that_rejects_the_certificate_does_not_break_the_listener() {
    let server = start().await;
    let tcp = TcpStream::connect(server.addr).await.unwrap();
    let refused = client_trusting(None)
        .connect(ServerName::try_from("localhost").unwrap(), tcp)
        .await;
    assert!(refused.is_err(), "an empty trust store must not accept it");
    let response = https_get(&server, "localhost", "/healthz").await.unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
}
