//! Checks the real `knx-server` exit status when its final journal flush fails or succeeds.
//!
//! The project file is deleted under the running server (a portable fault
//! that root cannot bypass), so the background write of an acknowledged
//! edit fails for real.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

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

fn start(name: &str) -> Server {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "knx-server-journal-{name}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
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
    // Dropped on a failed start too: the guard below reaps the child.
    let server = Server { child, port, dir };
    assert!(
        line.starts_with("knx-server listening on http://127.0.0.1:"),
        "unexpected first line: {line:?}"
    );
    server
}

/// One request with `Connection: close`; answers status and parsed body.
fn http(server: &Server, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let mut stream = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let status = raw[9..12].parse().unwrap();
    let payload = raw.split_once("\r\n\r\n").map_or("", |(_, b)| b);
    (status, serde_json::from_str(payload).unwrap_or(Value::Null))
}

/// A saved project under the data dir with one acknowledged, unjournaled-or-journaled edit.
/// Answers the project file path.
fn saved_project_then_edit(server: &Server, delete_file_first: bool) -> PathBuf {
    let (status, tree) = http(
        server,
        "POST",
        "/api/project/new",
        Some(json!({"discardChanges": true})),
    );
    assert_eq!(status, 200, "{tree}");
    let id = tree["installations"][0]["id"].as_u64().unwrap();
    let (status, body) = http(
        server,
        "POST",
        "/api/project/save-as",
        Some(json!({"path": "journal-exit.knxdb"})),
    );
    assert_eq!(status, 200, "{body}");
    let file = server.dir.join("data").join("journal-exit.knxdb");
    assert!(file.is_file(), "{} missing", file.display());
    if delete_file_first {
        std::fs::remove_file(&file).unwrap();
    }
    let (status, body) = http(
        server,
        "PATCH",
        &format!("/api/installations/{id}"),
        Some(json!({"name": "Acknowledged edit"})),
    );
    assert_eq!(status, 200, "the edit is acknowledged from memory: {body}");
    file
}

fn signal_term(server: &Server) {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(server.child.id().to_string())
        .status()
        .unwrap();
    assert!(status.success());
}

fn wait_for_exit(server: &mut Server, limit: Duration) -> ExitStatus {
    let started = Instant::now();
    while started.elapsed() < limit {
        if let Some(status) = server.child.try_wait().unwrap() {
            return status;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("knx-server did not exit within {limit:?}");
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

fn stored_name(file: &std::path::Path) -> String {
    let conn = knx_store::open_existing_and_migrate(file).unwrap();
    knx_store::project_history::load_editor(&conn)
        .unwrap()
        .unwrap()
        .working
        .project
        .installations[0]
        .name
        .clone()
}

fn wait_for_failed_journal(server: &Server) {
    let started = Instant::now();
    loop {
        let (_, status) = http(server, "GET", "/api/project/journal", None);
        if status["state"] == "failed" {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "journal never failed: {status}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

const PROMPT_EXIT: Duration = Duration::from_secs(10);

#[test]
fn a_drained_stop_with_a_healthy_journal_flushes_the_edit_and_exits_zero() {
    let mut server = start("healthy");
    let file = saved_project_then_edit(&server, false);
    signal_term(&server);
    let status = wait_for_exit(&mut server, PROMPT_EXIT);
    let stderr = stderr_of(&mut server);
    assert!(status.success(), "exit status {status}: {stderr}");
    assert!(stderr.contains("knx-server: stopped"), "{stderr}");
    assert!(!stderr.contains("could not be written"), "{stderr}");
    assert_eq!(stored_name(&file), "Acknowledged edit");
}

#[test]
fn a_drained_stop_with_a_failed_journal_reports_it_and_exits_nonzero() {
    let mut server = start("failed");
    let file = saved_project_then_edit(&server, true);
    wait_for_failed_journal(&server);
    signal_term(&server);
    let status = wait_for_exit(&mut server, PROMPT_EXIT);
    let stderr = stderr_of(&mut server);
    assert!(
        stderr.contains("knx-server: the last edits could not be written to the project file:"),
        "{stderr}"
    );
    assert!(stderr.contains("knx-server: stopped"), "{stderr}");
    assert!(
        !stderr.contains("cut off"),
        "the stop was meant to drain: {stderr}"
    );
    assert_eq!(status.code(), Some(1), "exit status {status}: {stderr}");
    assert!(
        !file.exists(),
        "the failed write must not recreate the file"
    );
}

/// Starts a request that never finishes, so the stop needs its grace and is forced.
fn stalled_connection(server: &Server) -> TcpStream {
    let mut stalled = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    stalled
        .write_all(b"GET /healthz HTTP/1.1\r\nHost: loc")
        .unwrap();
    std::thread::sleep(Duration::from_millis(200));
    stalled
}

#[test]
fn a_forced_stop_with_a_failed_journal_exits_nonzero() {
    let mut server = start("forced-failed");
    let file = saved_project_then_edit(&server, true);
    wait_for_failed_journal(&server);
    let _stalled = stalled_connection(&server);
    signal_term(&server);
    let status = wait_for_exit(&mut server, Duration::from_secs(15));
    let stderr = stderr_of(&mut server);
    assert!(
        stderr.contains("are cut off"),
        "the stop was not forced: {stderr}"
    );
    assert!(
        stderr.contains("knx-server: the last edits could not be written to the project file:"),
        "{stderr}"
    );
    assert_eq!(status.code(), Some(1), "exit status {status}: {stderr}");
    assert!(!file.exists());
}

#[test]
fn a_forced_stop_with_a_healthy_journal_still_exits_zero() {
    let mut server = start("forced-healthy");
    let file = saved_project_then_edit(&server, false);
    let _stalled = stalled_connection(&server);
    signal_term(&server);
    let status = wait_for_exit(&mut server, Duration::from_secs(15));
    let stderr = stderr_of(&mut server);
    assert!(
        stderr.contains("are cut off"),
        "the stop was not forced: {stderr}"
    );
    assert!(status.success(), "exit status {status}: {stderr}");
    assert_eq!(stored_name(&file), "Acknowledged edit");
}
