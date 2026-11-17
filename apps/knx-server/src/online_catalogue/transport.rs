//! One-connection-per-request HTTPS/1.1 client over rustls with a request spacing gate.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, Request};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tokio_rustls::TlsConnector;

use super::source::BaseUrl;

/// The catalogue host advertises roughly two requests per second per address; stay under it.
pub const MIN_REQUEST_GAP: Duration = Duration::from_millis(1000);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const HEADER_TIMEOUT: Duration = Duration::from_secs(60);
/// A body that sends nothing for this long has stalled.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// A narrow home connection still has to finish an index within this.
pub const INDEX_TOTAL_TIMEOUT: Duration = Duration::from_secs(900);
/// Small JSON replies (login, error bodies) are bounded tightly.
pub const SMALL_BODY_BYTES: usize = 16 * 1024;

/// A transport-level failure, named for the user-facing explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    Connect,
    Tls,
    Timeout,
    Protocol,
    BodyLimit,
}

impl TransportError {
    pub fn describe(self) -> &'static str {
        match self {
            Self::Connect => "connection failed",
            Self::Tls => "TLS handshake or certificate check failed",
            Self::Timeout => "the server did not answer in time",
            Self::Protocol => "unexpected HTTP response",
            Self::BodyLimit => "the response is larger than allowed",
        }
    }
}

/// The reply head plus a body still to be read.
pub struct Reply {
    _connection: ConnectionTask,
    pub status: u16,
    pub headers: HeaderMap,
    body: Incoming,
}

/// Dropping a reply (including cancellation) also tears down its one-shot connection.
struct ConnectionTask(tokio::task::JoinHandle<()>);

impl Drop for ConnectionTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl Reply {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// The next body chunk, or `None` at the end; each wait is bounded by `idle`.
    pub async fn chunk(&mut self, idle: Duration) -> Result<Option<Bytes>, TransportError> {
        tokio::time::timeout(idle, async {
            loop {
                match self.body.frame().await {
                    None => return Ok(None),
                    Some(Err(_)) => return Err(TransportError::Protocol),
                    Some(Ok(frame)) => {
                        if let Ok(data) = frame.into_data() {
                            if !data.is_empty() {
                                return Ok(Some(data));
                            }
                        }
                    }
                }
            }
        })
        .await
        .map_err(|_| TransportError::Timeout)?
    }

    /// Reads the whole body up to `cap` bytes within `total`.
    pub async fn read_bounded(
        mut self,
        cap: usize,
        total: Duration,
    ) -> Result<Vec<u8>, TransportError> {
        let deadline = Instant::now() + total;
        let mut out = Vec::new();
        loop {
            let wait = deadline
                .checked_duration_since(Instant::now())
                .ok_or(TransportError::Timeout)?
                .min(IDLE_TIMEOUT);
            match self.chunk(wait).await? {
                None => return Ok(out),
                Some(data) => {
                    if out.len() + data.len() > cap {
                        return Err(TransportError::BodyLimit);
                    }
                    out.extend_from_slice(&data);
                }
            }
        }
    }
}

/// An HTTPS client bound to a fixed trust store.
pub struct HttpsClient {
    tls: TlsConnector,
    last: Mutex<Option<Instant>>,
    gap: Duration,
}

impl HttpsClient {
    /// The production client: Mozilla's roots, independent of the host's certificate store.
    pub fn production() -> Self {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        Self::with_roots(roots, MIN_REQUEST_GAP)
    }

    pub fn with_roots(roots: RootCertStore, gap: Duration) -> Self {
        let config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .expect("ring supports the default protocol versions")
                .with_root_certificates(roots)
                .with_no_client_auth();
        Self {
            tls: TlsConnector::from(Arc::new(config)),
            last: Mutex::new(None),
            gap,
        }
    }

    /// Sends one request. Redirects are never followed: the caller sees the 3xx reply.
    pub async fn request(
        &self,
        base: &BaseUrl,
        method: Method,
        path: &str,
        headers: &[(HeaderName, HeaderValue)],
        body: Option<Vec<u8>>,
    ) -> Result<Reply, TransportError> {
        self.space().await;
        let host = base.host().trim_start_matches('[').trim_end_matches(']');
        let name = ServerName::try_from(host.to_string()).map_err(|_| TransportError::Tls)?;
        let tcp = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect((host, base.port())))
            .await
            .map_err(|_| TransportError::Timeout)?
            .map_err(|_| TransportError::Connect)?;
        let tls = tokio::time::timeout(CONNECT_TIMEOUT, self.tls.connect(name, tcp))
            .await
            .map_err(|_| TransportError::Timeout)?
            .map_err(|_| TransportError::Tls)?;
        let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(tls))
            .await
            .map_err(|_| TransportError::Protocol)?;
        // Ends by itself when the reply (or the failed send) drops the sender side.
        let connection = ConnectionTask(tokio::spawn(async move {
            let _ = connection.await;
        }));
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header(
                header::HOST,
                HeaderValue::from_str(&base.host_header()).map_err(|_| TransportError::Protocol)?,
            )
            .header(header::CONNECTION, "close")
            .header(
                header::USER_AGENT,
                concat!("KNXBench/", env!("CARGO_PKG_VERSION")),
            );
        for (name, value) in headers {
            builder = builder.header(name, value);
        }
        let request = builder
            .body(Full::new(Bytes::from(body.unwrap_or_default())))
            .map_err(|_| TransportError::Protocol)?;
        let response = tokio::time::timeout(HEADER_TIMEOUT, sender.send_request(request))
            .await
            .map_err(|_| TransportError::Timeout)?
            .map_err(|_| TransportError::Protocol)?;
        let (parts, body) = response.into_parts();
        Ok(Reply {
            _connection: connection,
            status: parts.status.as_u16(),
            headers: parts.headers,
            body,
        })
    }

    /// Waits until at least `gap` has passed since the previous request started.
    async fn space(&self) {
        let mut last = self.last.lock().await;
        if let Some(previous) = *last {
            tokio::time::sleep_until(previous + self.gap).await;
        }
        *last = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::online_catalogue::test_server::{Script, TestServer};

    #[tokio::test]
    async fn a_trusted_server_answers_and_redirects_are_not_followed() {
        let server = TestServer::start(vec![
            Script::reply(200, &[("x-a", "1")], b"hello"),
            Script::reply(302, &[("location", "https://elsewhere.invalid/")], b""),
        ])
        .await;
        let client = HttpsClient::with_roots(server.roots(), Duration::ZERO);
        let base = server.base();
        let reply = client
            .request(&base, Method::GET, "/a", &[], None)
            .await
            .unwrap();
        assert_eq!((reply.status, reply.header("x-a")), (200, Some("1")));
        assert_eq!(
            reply
                .read_bounded(100, Duration::from_secs(5))
                .await
                .unwrap(),
            b"hello"
        );
        let redirect = client
            .request(&base, Method::GET, "/b", &[], None)
            .await
            .unwrap();
        assert_eq!(redirect.status, 302);
        assert_eq!(server.requests().len(), 2);
    }

    #[tokio::test]
    async fn an_untrusted_certificate_is_refused_before_any_request_is_sent() {
        let server = TestServer::start(vec![Script::reply(200, &[], b"never")]).await;
        // An empty trust store trusts nothing, including this server's own certificate.
        let client = HttpsClient::with_roots(RootCertStore::empty(), Duration::ZERO);
        let result = client
            .request(&server.base(), Method::GET, "/a", &[], None)
            .await;
        assert_eq!(result.err(), Some(TransportError::Tls));
        assert!(server.requests().is_empty());
    }

    #[tokio::test]
    async fn bodies_over_the_cap_are_refused_while_reading() {
        let server = TestServer::start(vec![Script::reply(200, &[], &[b'x'; 50])]).await;
        let client = HttpsClient::with_roots(server.roots(), Duration::ZERO);
        let reply = client
            .request(&server.base(), Method::GET, "/a", &[], None)
            .await
            .unwrap();
        assert_eq!(
            reply.read_bounded(49, Duration::from_secs(5)).await.err(),
            Some(TransportError::BodyLimit)
        );
    }

    #[tokio::test]
    async fn requests_are_spaced_by_the_configured_gap() {
        let server = TestServer::start(vec![Script::reply(200, &[], b"")]).await;
        let gap = Duration::from_millis(400);
        let client = HttpsClient::with_roots(server.roots(), gap);
        let base = server.base();
        let start = Instant::now();
        for _ in 0..2 {
            client
                .request(&base, Method::GET, "/a", &[], None)
                .await
                .unwrap();
        }
        assert!(start.elapsed() >= gap);
    }
}
