//! A scripted loopback HTTPS server with a throwaway certificate, for the catalogue tests.

use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{RootCertStore, ServerConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use super::source::BaseUrl;
use crate::tls_cert::{generate_self_signed, SubjectName};

/// One scripted answer, consumed in order; the last one repeats when the script runs out.
#[derive(Clone)]
pub struct Script {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    /// Announce this many body bytes instead of the real length (a truncated transfer).
    declared_len: Option<usize>,
    /// Send the headers and prefix, then wait for the client to close the stream.
    hold_body: bool,
    /// A deterministic test-owned gate before response headers are sent.
    release: Option<Arc<tokio::sync::Notify>>,
}

impl Script {
    pub fn reply(status: u16, headers: &[(&str, &str)], body: &[u8]) -> Self {
        Self {
            status,
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: body.to_vec(),
            declared_len: None,
            hold_body: false,
            release: None,
        }
    }

    pub fn gated(mut self) -> (Self, Arc<tokio::sync::Notify>) {
        let release = Arc::new(tokio::sync::Notify::new());
        self.release = Some(Arc::clone(&release));
        (self, release)
    }

    pub fn json(status: u16, value: serde_json::Value) -> Self {
        Self::reply(
            status,
            &[("content-type", "application/json")],
            &serde_json::to_vec(&value).unwrap(),
        )
    }

    pub fn truncated(body: &[u8], declared: usize) -> Self {
        let mut script = Self::reply(200, &[], body);
        script.declared_len = Some(declared);
        script
    }

    pub fn holding_body(prefix: &[u8], declared: usize) -> Self {
        let mut script = Self::truncated(prefix, declared);
        script.hold_body = true;
        script
    }
}

/// What the server saw.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub struct TestServer {
    addr: SocketAddr,
    cert: CertificateDer<'static>,
    requests: Arc<Mutex<Vec<Recorded>>>,
    listener: tokio::task::JoinHandle<()>,
}

impl TestServer {
    pub async fn start(script: Vec<Script>) -> Self {
        let names = [SubjectName::Ip(IpAddr::V4(Ipv4Addr::LOCALHOST))];
        let (cert_pem, key_pem) =
            generate_self_signed(&names, time::OffsetDateTime::now_utc()).unwrap();
        let cert = CertificateDer::from_pem_slice(cert_pem.as_bytes()).unwrap();
        let key = PrivateKeyDer::from_pem_slice(key_pem.as_bytes()).unwrap();
        let mut config =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(vec![cert.clone()], key)
                .unwrap();
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let queue = Arc::new(Mutex::new(VecDeque::from(script)));
        let seen = Arc::clone(&requests);
        let listener = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                let accepted = tokio::select! {
                    accepted = listener.accept() => accepted,
                    _ = connections.join_next(), if !connections.is_empty() => continue,
                };
                let Ok((tcp, _)) = accepted else { return };
                let acceptor = acceptor.clone();
                let queue = Arc::clone(&queue);
                let seen = Arc::clone(&seen);
                connections.spawn(async move {
                    let Ok(mut tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    let Some(request) = read_request(&mut tls).await else {
                        return;
                    };
                    seen.lock().unwrap().push(request);
                    let script = {
                        let mut queue = queue.lock().unwrap();
                        if queue.len() > 1 {
                            queue.pop_front().unwrap()
                        } else {
                            queue
                                .front()
                                .cloned()
                                .unwrap_or_else(|| Script::reply(500, &[], b""))
                        }
                    };
                    if let Some(release) = &script.release {
                        if tokio::time::timeout(
                            std::time::Duration::from_secs(30),
                            release.notified(),
                        )
                        .await
                        .is_err()
                        {
                            return;
                        }
                    }
                    let mut head = format!(
                        "HTTP/1.1 {} X\r\ncontent-length: {}\r\nconnection: close\r\n",
                        script.status,
                        script.declared_len.unwrap_or(script.body.len())
                    );
                    for (k, v) in &script.headers {
                        head.push_str(&format!("{k}: {v}\r\n"));
                    }
                    head.push_str("\r\n");
                    let _ = tls.write_all(head.as_bytes()).await;
                    let _ = tls.write_all(&script.body).await;
                    let _ = tls.flush().await;
                    if script.hold_body {
                        let mut byte = [0u8; 1];
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_secs(10),
                            tls.read(&mut byte),
                        )
                        .await;
                    }
                    let _ = tls.shutdown().await;
                });
            }
        });
        Self {
            addr,
            cert,
            requests,
            listener,
        }
    }

    pub fn base(&self) -> BaseUrl {
        BaseUrl::parse(&self.base_text()).unwrap()
    }

    pub fn base_text(&self) -> String {
        format!("https://127.0.0.1:{}", self.addr.port())
    }

    pub fn roots(&self) -> RootCertStore {
        let mut roots = RootCertStore::empty();
        roots.add(self.cert.clone()).unwrap();
        roots
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // Dropping JoinSet when this task is aborted also cancels every held connection.
        self.listener.abort();
    }
}

async fn read_request<S: AsyncReadExt + Unpin>(stream: &mut S) -> Option<Recorded> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos;
        }
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_string();
    let path = first.next()?.to_string();
    let headers: HashMap<String, String> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let length: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    Some(Recorded {
        method,
        path,
        headers,
        body,
    })
}
