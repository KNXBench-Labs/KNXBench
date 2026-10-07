//! The real `knx-mcp` binary over stdio: handshake, tool list, calls, refusals, exit.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

const BIN: &str = env!("CARGO_BIN_EXE_knx-mcp");
const WAIT: Duration = Duration::from_secs(30);

struct Session {
    child: Child,
    lines: Receiver<String>,
}

impl Session {
    fn start(args: &[&str]) -> Self {
        let mut child = Command::new(BIN)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Self { child, lines }
    }

    fn send(&mut self, message: Value) {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    /// The response to request `id`. Every stdout line must be JSON.
    fn response(&self, id: u64) -> Value {
        loop {
            let line = self.lines.recv_timeout(WAIT).expect("a response in time");
            let message: Value = serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("stdout carried a non-JSON line ({e}): {line}"));
            if message["id"] == id {
                return message;
            }
        }
    }

    fn call(&mut self, id: u64, tool: &str, arguments: Value) -> Value {
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": tool, "arguments": arguments },
        }));
        self.response(id)
    }
}

#[test]
fn a_client_lists_eight_read_only_tools_and_calls_them() {
    let dir = tempfile::tempdir().unwrap();
    let path = common::save(dir.path(), "home.knxdb", &common::project("Home"));
    let before = std::fs::read(&path).unwrap();
    let project = format!("home={}", path.display());
    let mut session = Session::start(&["--project", &project, "--no-product-db"]);

    session.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "knx-mcp-test", "version": "0" },
        },
    }));
    let init = session.response(1);
    assert_eq!(init["result"]["serverInfo"]["name"], "knx-mcp", "{init}");
    let instructions = init["result"]["instructions"].as_str().unwrap();
    assert!(instructions.contains("home") && instructions.contains("Nothing can be changed"));
    session.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));

    session.send(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));
    let list = session.response(2);
    let tools = list["result"]["tools"].as_array().unwrap();
    let mut names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "diff_projects",
            "explain_parameter",
            "find_issues",
            "get_device",
            "get_group_address",
            "project_summary",
            "search",
            "validate_ga_csv",
        ]
    );
    for tool in tools {
        let annotations = &tool["annotations"];
        assert_eq!(annotations["readOnlyHint"], true, "{tool}");
        assert_eq!(annotations["destructiveHint"], false, "{tool}");
        assert_eq!(annotations["openWorldHint"], false, "{tool}");
        let properties = tool["inputSchema"]["properties"].as_object().unwrap();
        assert!(
            !properties.contains_key("path"),
            "no tool takes a path: {tool}"
        );
    }

    let summary = session.call(3, "project_summary", json!({ "project": "home" }));
    assert_eq!(summary["result"]["isError"], false, "{summary}");
    assert_eq!(
        summary["result"]["structuredContent"]["result"]["name"],
        "Home"
    );
    let text = summary["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(text).unwrap()["result"]["name"],
        "Home"
    );

    let ambiguous = session.call(
        4,
        "get_device",
        json!({ "project": "home", "device": "1.1.2" }),
    );
    assert_eq!(ambiguous["result"]["isError"], true, "{ambiguous}");

    let smuggled = session.call(
        5,
        "search",
        json!({ "project": "home", "query": "x", "path": "/etc/passwd" }),
    );
    let refused = smuggled.get("error").is_some() || smuggled["result"]["isError"] == true;
    assert!(refused, "an unknown argument must be refused: {smuggled}");

    let missing = session.call(6, "bus_write", json!({}));
    assert!(
        missing.get("error").is_some() || missing["result"]["isError"] == true,
        "{missing}"
    );

    // Closing stdin ends the session cleanly.
    drop(session.child.stdin.take());
    let status = session.child.wait().unwrap();
    assert!(status.success(), "{status:?}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_bad_configuration_fails_at_startup_with_nothing_on_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let missing = format!("home={}", dir.path().join("absent.knxdb").display());
    let output = Command::new(BIN)
        .args(["--project", &missing, "--no-product-db"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("\"home\""), "{stderr}");
    assert!(!dir.path().join("absent.knxdb").exists(), "nothing created");

    let usage = Command::new(BIN).arg("--bus").output().unwrap();
    assert_eq!(usage.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&usage.stderr).contains("usage: knx-mcp"));

    let help = Command::new(BIN).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("--project <alias>=<path>"));
}
