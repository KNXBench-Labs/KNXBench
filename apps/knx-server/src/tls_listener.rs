//! A TLS listener for `axum::serve` that redirects plain HTTP on the same port.
//!
//! See ADR-0088.
//! Accepting and handshaking are decoupled on purpose. A TLS handshake is
//! a conversation with the client, and a listener that held it inside
//! `accept()` would let one client that connects and says nothing stop
//! every other client from being accepted at all. So a background task
//! accepts TCP connections and hands each to its own task, which has
//! [`HANDSHAKE_TIMEOUT`] to become a TLS stream; only finished streams
//! reach `axum` through a channel.
//!
//! A connection is told apart by its first byte. Every TLS connection
//! opens with a handshake record, content type 22 (RFC 8446 §5.1, the
//! same value in TLS 1.2: RFC 5246 §6.2.1); an HTTP request line opens
//! with a method name in ASCII letters. Anything that is not 22 gets an
//! HTTP answer pointing at `https://`, and is then closed.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use rustls::ServerConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;

/// How long a connection has to finish its TLS handshake, or to send the
/// plain-HTTP request head it is being redirected for.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// TLS record content type `handshake` (RFC 8446 §5.1).
const TLS_HANDSHAKE_RECORD: u8 = 22;
/// The most of a plain-HTTP request head read before answering; the
/// request line and `Host` are all the redirect needs.
const MAX_PLAINTEXT_HEAD: usize = 8 * 1024;
/// Finished TLS streams waiting for `axum` to take them.
const ACCEPTED_BACKLOG: usize = 64;

/// TLS connections, ready for `axum::serve`.
pub struct TlsListener {
    accepted: mpsc::Receiver<(TlsStream<TcpStream>, SocketAddr)>,
    local_addr: SocketAddr,
    accept_task: tokio::task::JoinHandle<()>,
}

impl TlsListener {
    pub fn new(tcp: TcpListener, config: Arc<ServerConfig>) -> io::Result<Self> {
        let local_addr = tcp.local_addr()?;
        let (sender, accepted) = mpsc::channel(ACCEPTED_BACKLOG);
        let accept_task = tokio::spawn(accept_loop(
            tcp,
            TlsAcceptor::from(config),
            sender,
            local_addr.port(),
        ));
        Ok(TlsListener {
            accepted,
            local_addr,
            accept_task,
        })
    }
}

impl Drop for TlsListener {
    fn drop(&mut self) {
        self.accept_task.abort();
    }
}

impl axum::serve::Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.accepted.recv().await {
            Some(connection) => connection,
            // The accept task only ends when this listener is dropped, and a
            // dropped listener is not being polled; wait forever rather than
            // invent a connection.
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        Ok(self.local_addr)
    }
}

async fn accept_loop(
    tcp: TcpListener,
    acceptor: TlsAcceptor,
    accepted: mpsc::Sender<(TlsStream<TcpStream>, SocketAddr)>,
    port: u16,
) {
    loop {
        let (stream, peer) = match tcp.accept().await {
            Ok(connection) => connection,
            Err(error) => {
                // Same policy as axum's own TCP listener: a connection that
                // died before it was accepted is the client's business; an
                // error of the listener itself (out of file descriptors) is
                // reported and retried after a pause instead of spinning.
                if !is_connection_error(&error) {
                    eprintln!("knx-server: accept failed: {error}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                continue;
            }
        };
        let acceptor = acceptor.clone();
        let accepted = accepted.clone();
        tokio::spawn(async move {
            // A failed or abandoned handshake is ordinary — a browser
            // rejecting a self-signed certificate closes the connection
            // mid-handshake on purpose — so it is dropped without a log
            // line per attempt.
            if let Ok(Ok(Some(tls))) =
                tokio::time::timeout(HANDSHAKE_TIMEOUT, admit(stream, acceptor, port)).await
            {
                let _ = accepted.send((tls, peer)).await;
            }
        });
    }
}

fn is_connection_error(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    )
}

/// The TLS stream, or `None` for a connection that was answered in plain
/// HTTP or closed before saying anything.
async fn admit(
    stream: TcpStream,
    acceptor: TlsAcceptor,
    port: u16,
) -> io::Result<Option<TlsStream<TcpStream>>> {
    let mut first = [0u8; 1];
    if stream.peek(&mut first).await? == 0 {
        return Ok(None);
    }
    if first[0] == TLS_HANDSHAKE_RECORD {
        return acceptor.accept(stream).await.map(Some);
    }
    answer_plaintext(stream, port).await?;
    Ok(None)
}

async fn answer_plaintext(mut stream: TcpStream, port: u16) -> io::Result<()> {
    let mut head = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    while head.len() < MAX_PLAINTEXT_HEAD && !has_head_end(&head) {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        head.extend_from_slice(&chunk[..read]);
    }
    stream
        .write_all(plaintext_response(&head, port).as_bytes())
        .await?;
    stream.shutdown().await
}

fn has_head_end(head: &[u8]) -> bool {
    head.windows(4).any(|w| w == b"\r\n\r\n") || head.windows(2).any(|w| w == b"\n\n")
}

/// The whole HTTP/1.1 response for a plain request that reached the TLS
/// port: `307` to the same host, port and path under `https://` when the
/// request names a usable `Host`, otherwise `400` saying what to do.
///
/// `307` rather than `301`/`308`: a permanent redirect is cached by
/// browsers indefinitely, and would keep sending them to `https://` after
/// the deployer switched TLS off again. Both the host and the path are
/// echoed only after checking every byte, so nothing a client sends can
/// add a header or a line to the response.
pub(crate) fn plaintext_response(head: &[u8], port: u16) -> String {
    let text = String::from_utf8_lossy(head);
    let mut lines = text.split('\n').map(|line| line.trim_end_matches('\r'));
    let target = lines
        .next()
        .and_then(|request_line| request_line.split(' ').nth(1))
        .filter(|target| is_origin_form(target))
        .unwrap_or("/");
    let host = lines
        .take_while(|line| !line.is_empty())
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("host")
                .then(|| value.trim())
        })
        .filter(|host| is_host(host));
    match host {
        Some(host) => {
            let authority = if host_has_port(host) {
                host.to_string()
            } else {
                format!("{host}:{port}")
            };
            let location = format!("https://{authority}{target}");
            let body = format!("This server speaks HTTPS. Continue at {location}\n");
            format!(
                "HTTP/1.1 307 Temporary Redirect\r\nLocation: {location}\r\n\
                 Content-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{body}",
                body.len()
            )
        }
        None => {
            let body =
                format!("This server speaks HTTPS. Open https://<this server>:{port}/ instead.\n");
            format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain; charset=utf-8\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
        }
    }
}

/// An origin-form request target (RFC 9112 §3.2.1): a path starting with
/// `/`, made only of visible ASCII. Anything else redirects to `/`.
fn is_origin_form(target: &str) -> bool {
    target.starts_with('/')
        && !target.starts_with("//")
        && target.bytes().all(|b| b.is_ascii_graphic())
}

/// A host name, an IPv4 address or a bracketed IPv6 address, optionally
/// with a port: letters, digits, `.`, `-`, `:`, `[` and `]` only.
fn is_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 255
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':' | b'[' | b']'))
}

fn host_has_port(host: &str) -> bool {
    match host.strip_prefix('[') {
        Some(bracketed) => bracketed.contains("]:"),
        None => host.contains(':'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location(response: &str) -> Option<&str> {
        response
            .lines()
            .find_map(|line| line.strip_prefix("Location: "))
    }

    #[test]
    fn a_bookmark_is_sent_to_the_same_place_over_https() {
        let response = plaintext_response(
            b"GET /api/version?x=1 HTTP/1.1\r\nHost: knx.lan:8484\r\nAccept: */*\r\n\r\n",
            8484,
        );
        assert!(response.starts_with("HTTP/1.1 307 "), "{response}");
        assert_eq!(
            location(&response),
            Some("https://knx.lan:8484/api/version?x=1")
        );
        assert!(response.contains("Connection: close"));
    }

    #[test]
    fn a_host_without_a_port_gets_this_servers_port() {
        let response = plaintext_response(b"GET / HTTP/1.1\nhost: 192.168.1.10\n\n", 8484);
        assert_eq!(location(&response), Some("https://192.168.1.10:8484/"));
        let v6 = plaintext_response(b"GET / HTTP/1.1\r\nHost: [fe80::1]\r\n\r\n", 80);
        assert_eq!(location(&v6), Some("https://[fe80::1]:80/"));
        let v6_port = plaintext_response(b"GET / HTTP/1.1\r\nHost: [::1]:9000\r\n\r\n", 80);
        assert_eq!(location(&v6_port), Some("https://[::1]:9000/"));
    }

    #[test]
    fn nothing_a_client_sends_reaches_the_response_unchecked() {
        let injected = plaintext_response(
            b"GET / HTTP/1.1\r\nHost: evil.example\r\x0bSet-Cookie: x=y\r\n\r\n",
            8484,
        );
        assert!(injected.starts_with("HTTP/1.1 400 "), "{injected}");
        assert!(!injected.contains("Set-Cookie"));

        let path_with_space = plaintext_response(b"GET /a%20b\x7fc HTTP/1.1\r\nHost: h\r\n\r\n", 1);
        assert_eq!(location(&path_with_space), Some("https://h:1/"));

        let protocol_relative =
            plaintext_response(b"GET //evil.example/ HTTP/1.1\r\nHost: h\r\n\r\n", 1);
        assert_eq!(location(&protocol_relative), Some("https://h:1/"));
    }

    #[test]
    fn no_usable_host_is_told_what_to_do_instead() {
        for head in [
            &b"GET / HTTP/1.0\r\n\r\n"[..],
            b"",
            b"\x00\x01garbage",
            b"GET / HTTP/1.1\r\nHost: a b\r\n\r\n",
        ] {
            let response = plaintext_response(head, 8484);
            assert!(response.starts_with("HTTP/1.1 400 "), "{response}");
            assert!(response.contains("https://<this server>:8484/"));
        }
    }

    #[test]
    fn the_content_length_matches_the_body() {
        for response in [
            plaintext_response(b"GET / HTTP/1.1\r\nHost: h\r\n\r\n", 1),
            plaintext_response(b"", 1),
        ] {
            let (head, body) = response.split_once("\r\n\r\n").unwrap();
            let declared: usize = head
                .lines()
                .find_map(|l| l.strip_prefix("Content-Length: "))
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(declared, body.len());
        }
    }
}
