//! Opt-in real process restart after regular server settlement, using only loopback HTTP.

use super::*;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Process {
    child: Child,
    port: u16,
    stdout: PathBuf,
    stderr: PathBuf,
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Process {
    fn start(binary: &Path, root: &Path, attempt: usize) -> Self {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let stdout = root.join(format!("process-{attempt}.stdout"));
        let stderr = root.join(format!("process-{attempt}.stderr"));
        let mut command = Command::new(binary);
        command
            .env("KNX_PORT", port.to_string())
            .env("KNX_DATA_DIR", root.join("data"))
            .env("XDG_DATA_HOME", root.join("xdg"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("KNX_TLS", "off")
            .env_remove("KNX_AUTH_PASSWORD")
            .env_remove("KNX_AUTH_PASSWORD_HASH")
            .env_remove("KNX_STATIC_DIR")
            .env_remove("KNX_TUNNEL_ROUTE_BACK")
            .stdout(std::fs::File::create(&stdout).unwrap())
            .stderr(Stdio::from(std::fs::File::create(&stderr).unwrap()));
        let child = command.spawn().unwrap();
        let mut process = Self {
            child,
            port,
            stdout,
            stderr,
        };
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let ready = std::fs::read_to_string(&process.stdout).unwrap();
            if ready.contains("knx-server listening on http://127.0.0.1:") {
                break;
            }
            assert!(
                process.child.try_wait().unwrap().is_none(),
                "server exited: {}",
                std::fs::read_to_string(&process.stderr).unwrap()
            );
            assert!(Instant::now() < deadline, "server startup timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
        process
    }
    fn http(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        let body = body.map(|b| b.to_string()).unwrap_or_default();
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        write!(stream,"{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        assert!(headers.starts_with("HTTP/1.1 200"), "{response}");
        serde_json::from_str(body).unwrap()
    }
    fn stop(&mut self) {
        assert!(Command::new("kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status()
            .unwrap()
            .success());
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(exit) = self.child.try_wait().unwrap() {
                assert!(exit.success(), "{exit}");
                println!("process {} direct exit {exit}", self.child.id());
                break;
            }
            assert!(Instant::now() < deadline, "server did not shut down");
            std::thread::sleep(Duration::from_millis(10));
        }
        println!("{}", std::fs::read_to_string(&self.stdout).unwrap());
        println!("{}", std::fs::read_to_string(&self.stderr).unwrap());
    }
}

#[test]
#[ignore = "requires KNXBENCH_TEST_SERVER_BINARY; launches loopback-only real server processes"]
fn regular_settlement_survives_two_real_server_processes_and_get_detail() {
    let binary = PathBuf::from(
        std::env::var_os("KNXBENCH_TEST_SERVER_BINARY").expect("explicit server binary"),
    );
    assert!(binary.is_file());
    println!("EXECUTED_BINARY={}", binary.display());
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    let root = f._dir.path();
    let data = root.join("data");
    std::fs::create_dir(&data).unwrap();
    let native = data.join("settled.knxdb");
    domain::save_project_as(&f.state, &native).unwrap();
    // Genuine server callback, source proof and atomic journal/baseline write.
    full(&f, false);
    edit_link(&f.state);
    let expected = serde_json::to_value(detail(&f.state)).unwrap();
    assert_eq!(expected["differing_objects"], 1);
    let live = f.live.clone();
    drop(f.state);
    let product = root.join("xdg/knx/products.sqlite");
    std::fs::create_dir_all(product.parent().unwrap()).unwrap();
    std::fs::copy(live, product).unwrap();
    let mut incarnations = Vec::new();
    let mut pids = Vec::new();
    for attempt in 0..2 {
        let mut process = Process::start(&binary, root, attempt);
        pids.push(process.child.id());
        let tree = process.http(
            "POST",
            "/api/project/open",
            Some(json!({"path":"settled.knxdb"})),
        );
        incarnations.push(tree["server_incarnation"].clone());
        let detail = process.http("GET", "/api/device/1", None);
        assert_eq!(
            detail["last_download"], expected,
            "process restart must not replay settlement or baseline writes"
        );
        println!(
            "restart attempt {attempt}: pid={}, incarnation={}, comparison={}",
            process.child.id(),
            tree["server_incarnation"],
            detail["last_download"]
        );
        process.stop();
    }
    assert_ne!(pids[0], pids[1]);
    assert_ne!(incarnations[0], incarnations[1]);
}
