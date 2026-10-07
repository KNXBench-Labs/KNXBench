//! Checks that the real `knx-server` binary leaves promptly on SIGTERM and SIGINT.
//!
//! In a container the binary is PID 1, which the kernel never stops on a
//! signal it has no handler for; `docker stop` then has to fall back to
//! SIGKILL (exit code 137).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Well below `knx_server::STOP_GRACE`: an idle server must not need it.
const PROMPT_EXIT: Duration = Duration::from_secs(3);

struct Server {
    child: Child,
    port: u16,
    dir: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn scratch_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("knx-server-{name}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Starts the binary on loopback without a password and waits until it
/// listens. The product database goes into the scratch directory, never
/// into the developer's own.
fn start(name: &str) -> Server {
    let dir = scratch_dir(name);
    let port = free_port();
    let mut child = Command::new(env!("CARGO_BIN_EXE_knx-server"))
        .env("KNX_PORT", port.to_string())
        .env("KNX_DATA_DIR", dir.join("data"))
        .env("XDG_DATA_HOME", dir.join("xdg"))
        .env("KNX_TLS", "off")
        .env_remove("KNX_AUTH_PASSWORD")
        .env_remove("KNX_AUTH_PASSWORD_HASH")
        .env_remove("KNX_STATIC_DIR")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start the knx-server binary");
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert!(
        line.starts_with("knx-server listening on http://127.0.0.1:"),
        "unexpected first line: {line:?}"
    );
    Server { child, port, dir }
}

fn send_signal(server: &Server, signal: &str) {
    let status = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(server.child.id().to_string())
        .status()
        .expect("failed to run kill");
    assert!(status.success());
}

/// Waits for the process to exit, at most `limit`.
fn wait_for_exit(server: &mut Server, limit: Duration) -> Option<ExitStatus> {
    let started = Instant::now();
    while started.elapsed() < limit {
        if let Some(status) = server.child.try_wait().unwrap() {
            return Some(status);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

fn stderr_of(server: &mut Server) -> String {
    let mut stderr = String::new();
    server
        .child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    stderr
}

/// One request on a connection that stays open afterwards, the way a
/// browser keeps it for the next request.
fn idle_keep_alive_connection(port: u16) -> TcpStream {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n")
        .unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut head = [0u8; 12];
    stream.read_exact(&mut head).unwrap();
    assert_eq!(&head, b"HTTP/1.1 200");
    stream
}

#[test]
fn sigterm_ends_the_server_promptly_even_with_an_idle_connection_open() {
    let mut server = start("sigterm");
    let _browser = idle_keep_alive_connection(server.port);

    send_signal(&server, "TERM");
    let status =
        wait_for_exit(&mut server, PROMPT_EXIT).expect("knx-server did not exit after SIGTERM");

    assert!(status.success(), "exit status after SIGTERM: {status}");
    let stderr = stderr_of(&mut server);
    assert!(stderr.contains("knx-server: SIGTERM received"), "{stderr}");
    assert!(stderr.contains("knx-server: stopped"), "{stderr}");
    assert!(
        !stderr.contains("cut off"),
        "an idle server was forced: {stderr}"
    );
}

#[test]
fn sigint_ends_the_server_too() {
    let mut server = start("sigint");

    send_signal(&server, "INT");
    let status =
        wait_for_exit(&mut server, PROMPT_EXIT).expect("knx-server did not exit after SIGINT");

    assert!(status.success(), "exit status after SIGINT: {status}");
    let stderr = stderr_of(&mut server);
    assert!(stderr.contains("knx-server: SIGINT received"), "{stderr}");
    assert!(stderr.contains("knx-server: stopped"), "{stderr}");
}

#[test]
fn a_stalled_request_is_cut_off_after_the_grace_and_the_server_still_exits() {
    let mut server = start("stalled");
    // A client that starts a request and never finishes it: the server
    // cannot drain this connection by itself.
    let mut stalled = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    stalled
        .write_all(b"GET /healthz HTTP/1.1\r\nHost: loc")
        .unwrap();
    std::thread::sleep(Duration::from_millis(200));

    let sent = Instant::now();
    send_signal(&server, "TERM");
    let status = wait_for_exit(&mut server, Duration::from_secs(9))
        .expect("knx-server hung past its grace period");
    let took = sent.elapsed();

    assert!(status.success(), "exit status after SIGTERM: {status}");
    assert!(
        took >= Duration::from_secs(4),
        "exited after {took:?}; the stalled request was not given its grace"
    );
    let stderr = stderr_of(&mut server);
    assert!(stderr.contains("are cut off"), "{stderr}");
    assert!(stderr.contains("knx-server: stopped"), "{stderr}");
}

#[test]
fn a_second_signal_cuts_the_grace_short() {
    let mut server = start("second");
    let mut stalled = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    stalled
        .write_all(b"GET /healthz HTTP/1.1\r\nHost: loc")
        .unwrap();
    std::thread::sleep(Duration::from_millis(200));

    send_signal(&server, "TERM");
    std::thread::sleep(Duration::from_millis(300));
    send_signal(&server, "INT");
    let status = wait_for_exit(&mut server, PROMPT_EXIT)
        .expect("a second signal did not end the grace period");

    assert!(status.success(), "exit status: {status}");
    let stderr = stderr_of(&mut server);
    assert!(stderr.contains("second signal"), "{stderr}");
}
