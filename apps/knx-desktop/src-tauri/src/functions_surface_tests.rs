//! The desktop window reaches KNX Functions through its own embedded server.
//!
//! ARCHITECTURE rule 3 keeps project operations out of Tauri IPC: the
//! desktop window loads the web UI from the in-process `knx-server` that
//! [`crate::spawn_server`] starts, and its Functions panels talk to
//! `/api/project/functions` there (ADR-0124, MODEL-07). This test starts that
//! exact function on an ephemeral loopback port, as a release build does, and
//! drives the Functions surface over plain HTTP: build a room and an address,
//! create a Function, add a member, see a stale edit refused, undo, and read
//! the result back. No native window, display or bus is involved.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

struct Desktop {
    port: u16,
    server: knx_server::EmbeddedServerHandle,
    _data: tempfile::TempDir,
}

impl Desktop {
    /// Starts the shell's own server the way `run` does in a release build.
    fn start() -> Self {
        let data = tempfile::tempdir().unwrap();
        // `AppState::new`, not `with_user_product_db`: the test must never
        // open the developer's own product database. Functions do not use it.
        let state = Arc::new(knx_server::AppState::new(data.path().join("projects")));
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        // Retain the lifetime owner as the native shell does. Dropping it
        // closes the stop channel and ends HTTP before the first request.
        let server = crate::spawn_server(listener, state, None);
        Self {
            port,
            server,
            _data: data,
        }
    }

    /// One HTTP/1.1 request; answers status and JSON body. A chunked or
    /// unparsable answer fails the test instead of being guessed at.
    fn call(&self, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut stream = loop {
            match TcpStream::connect(("127.0.0.1", self.port)) {
                Ok(stream) => break stream,
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => panic!("embedded server never answered: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let body = body.map(|b| b.to_string()).unwrap_or_default();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        let text = String::from_utf8(raw).unwrap();
        let (head, payload) = text.split_once("\r\n\r\n").expect("an HTTP answer");
        assert!(
            !head
                .to_ascii_lowercase()
                .contains("transfer-encoding: chunked"),
            "{head}"
        );
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        let value = serde_json::from_str(payload).unwrap_or_else(|_| json!(payload));
        (status, value)
    }

    fn ok(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        let (status, value) = self.call(method, path, body);
        assert_eq!(status, 200, "{method} {path}: {value}");
        value
    }
}

impl Drop for Desktop {
    fn drop(&mut self) {
        // Join the actual HTTP and worker shutdown before removing its files,
        // including when a surface assertion unwinds. Never force the drain.
        let result =
            tauri::async_runtime::block_on(self.server.drain_close(std::future::pending()));
        if !std::thread::panicking() {
            assert_eq!(result.http, Ok(knx_server::ServeEnd::Drained));
            assert!(!result.requires_process_exit(), "{result:?}");
        }
    }
}

/// The `id` of the first object anywhere in `tree` whose `name` is `name`.
fn id_named(tree: &Value, name: &str) -> u64 {
    fn walk(value: &Value, name: &str) -> Option<u64> {
        match value {
            Value::Object(map) => {
                if map.get("name").and_then(Value::as_str) == Some(name) {
                    if let Some(id) = map.get("id").and_then(Value::as_u64) {
                        return Some(id);
                    }
                }
                map.values().find_map(|v| walk(v, name))
            }
            Value::Array(items) => items.iter().find_map(|v| walk(v, name)),
            _ => None,
        }
    }
    walk(tree, name).unwrap_or_else(|| panic!("no {name:?} in {tree}"))
}

fn bound(view: &Value, action: Value) -> Value {
    json!({
        "serverIncarnation": view["serverIncarnation"],
        "projectIncarnation": view["projectIncarnation"],
        "snapshotRevision": view["snapshotRevision"],
        "action": action,
    })
}

#[test]
fn the_desktop_server_reads_edits_refuses_stale_and_undoes_functions() {
    let desktop = Desktop::start();
    desktop.ok("POST", "/api/project/new", Some(json!({})));
    desktop.ok(
        "POST",
        "/api/building-parts",
        Some(json!({"name": "Kitchen", "kind": "Room"})),
    );
    let tree = desktop.ok(
        "POST",
        "/api/group-addresses",
        Some(json!({"name": "Light", "address": "1/0/1"})),
    );
    let room = id_named(&tree, "Kitchen");
    let light = id_named(&tree, "Light");

    let empty = desktop.ok("GET", "/api/project/functions", None);
    assert_eq!(empty["functions"], json!([]), "{empty}");

    let created = desktop.ok(
        "POST",
        "/api/project/functions/edit",
        Some(bound(
            &empty,
            json!({"kind": "create", "location": room, "details": {"name": "Ceiling"}}),
        )),
    );
    let function = created["createdId"].clone();
    let with_member = desktop.ok(
        "POST",
        "/api/project/functions/edit",
        Some(bound(
            &created,
            json!({"kind": "addMember", "functionId": function, "groupAddressId": light,
                   "role": "SwitchOnOff"}),
        )),
    );
    let listed = &with_member["functions"][0];
    assert_eq!(listed["name"], "Ceiling", "{with_member}");
    assert_eq!(listed["location"]["name"], "Kitchen", "{with_member}");
    assert_eq!(listed["members"][0]["address"], "1/0/1", "{with_member}");
    assert_eq!(listed["members"][0]["role"], "SwitchOnOff", "{with_member}");

    // An edit prepared against the older read is refused without effect.
    let (status, stale) = desktop.call(
        "POST",
        "/api/project/functions/edit",
        Some(bound(&created, json!({"kind": "delete", "id": function}))),
    );
    assert_eq!(status, 409, "{stale}");
    let after_stale = desktop.ok("GET", "/api/project/functions", None);
    assert_eq!(after_stale["functions"], with_member["functions"]);

    // Each edit is one undoable step of the project's own history.
    desktop.ok("POST", "/api/undo", None);
    let undone = desktop.ok("GET", "/api/project/functions", None);
    assert_eq!(undone["functions"][0]["members"], json!([]), "{undone}");
    desktop.ok("POST", "/api/undo", None);
    let gone = desktop.ok("GET", "/api/project/functions", None);
    assert_eq!(gone["functions"], json!([]), "{gone}");
}
