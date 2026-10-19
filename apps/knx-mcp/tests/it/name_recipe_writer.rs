//! The opt-in literal naming writer over the real stdio binary (ADR-0116).
//!
//! Every write needs the launch flag for that alias, an exact preview token
//! and a human `accept` of one form elicitation; anything else leaves the
//! file byte-identical.

use crate::common;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use knx_store::project_history as history;
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

    /// The next JSON message of any kind (response or server request).
    fn next(&self) -> Value {
        let line = self.lines.recv_timeout(WAIT).expect("a message in time");
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("non-JSON stdout ({e}): {line}"))
    }

    /// The response to request `id`; a server request in between fails.
    fn response(&self, id: u64) -> Value {
        let message = self.next();
        assert_eq!(message["id"], id, "expected response {id}, got {message}");
        assert!(
            message.get("method").is_none(),
            "unexpected server request: {message}"
        );
        message
    }

    fn init(&mut self, elicitation: bool) -> Value {
        let capabilities = if elicitation {
            json!({ "elicitation": { "form": {} } })
        } else {
            json!({})
        };
        self.send(json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": capabilities,
                "clientInfo": { "name": "knx-mcp-writer-test", "version": "0" },
            },
        }));
        let init = self.response(1);
        self.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        init
    }

    fn request_call(&mut self, id: u64, tool: &str, arguments: Value) {
        self.send(json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": { "name": tool, "arguments": arguments },
        }));
    }

    fn call(&mut self, id: u64, tool: &str, arguments: Value) -> Value {
        self.request_call(id, tool, arguments);
        self.response(id)
    }

    /// Waits for the server's elicitation request and answers it.
    fn answer_elicitation(&mut self, result: Value) -> Value {
        let request = self.next();
        assert_eq!(request["method"], "elicitation/create", "{request}");
        self.send(json!({ "jsonrpc": "2.0", "id": request["id"], "result": result }));
        request
    }

    fn close(mut self) {
        drop(self.child.stdin.take());
        assert!(self.child.wait().unwrap().success());
    }
}

fn is_error(response: &Value) -> bool {
    response.get("error").is_some() || response["result"]["isError"] == true
}

fn structured(response: &Value) -> &Value {
    &response["result"]["structuredContent"]["result"]
}

fn flag(alias: &str, path: &Path) -> String {
    format!("{alias}={}", path.display())
}

/// Working names, undo depth and generation from the native editor history,
/// plus the saved-root names, read without migrating or writing.
fn native(path: &Path) -> (BTreeMap<u32, String>, usize, i64, BTreeMap<u32, String>) {
    let store = knx_store::open_existing_read_only(path).unwrap();
    let names = |project: &knx_core::Project| {
        project
            .installations
            .iter()
            .flat_map(|i| &i.group_addresses)
            .map(|ga| (ga.id.0, ga.name.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    let saved = names(&knx_store::load_project(&store.conn).unwrap());
    match history::load_editor(&store.conn).unwrap() {
        Some(editor) => (
            names(&editor.working.project),
            editor.undo.len(),
            editor.generation,
            saved,
        ),
        None => (saved.clone(), 0, 0, saved),
    }
}

fn intent(project: &str, ids: &[u32], prefix: &str) -> Value {
    json!({ "project": project, "groupAddresses": ids, "prefix": prefix, "suffix": "" })
}

fn preview(session: &mut Session, id: u64, project: &str, ids: &[u32], prefix: &str) -> Value {
    let response = session.call(id, "preview_name_recipe", intent(project, ids, prefix));
    assert!(!is_error(&response), "{response}");
    structured(&response).clone()
}

fn apply_args(project: &str, ids: &[u32], prefix: &str, token: &Value) -> Value {
    let mut args = intent(project, ids, prefix);
    args["confirmationToken"] = token.clone();
    args
}

fn approve(value: bool) -> Value {
    json!({ "action": "accept", "content": { "approve": value } })
}

#[test]
fn without_the_launch_flag_the_writer_is_absent_and_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = common::save(dir.path(), "home.knxdb", &common::project("Home"));
    let before = std::fs::read(&path).unwrap();
    let mut session = Session::start(&["--project", &flag("home", &path), "--no-product-db"]);
    session.init(true);
    session.send(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));
    let list = session.response(2);
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 9, "{names:?}");
    assert!(!names.iter().any(|n| n.contains("name_recipe")));
    let refused = session.call(
        3,
        "apply_name_recipe",
        apply_args("home", &[1], "X ", &json!("0".repeat(64))),
    );
    assert!(is_error(&refused), "{refused}");
    session.close();
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn the_launch_flag_lists_two_writer_tools_with_honest_annotations() {
    let dir = tempfile::tempdir().unwrap();
    let path = common::save(dir.path(), "home.knxdb", &common::project("Home"));
    let mut session = Session::start(&[
        "--project",
        &flag("home", &path),
        "--no-product-db",
        "--allow-name-recipe",
        "home",
    ]);
    let init = session.init(true);
    let instructions = init["result"]["instructions"].as_str().unwrap();
    assert!(
        instructions.contains("apply_name_recipe") && instructions.contains("home"),
        "{instructions}"
    );
    session.send(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));
    let list = session.response(2);
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 11);
    let find = |name: &str| {
        tools
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("{name}"))
    };
    let preview = &find("preview_name_recipe")["annotations"];
    assert_eq!(
        (
            preview["readOnlyHint"].clone(),
            preview["idempotentHint"].clone()
        ),
        (json!(true), json!(true))
    );
    let apply = &find("apply_name_recipe")["annotations"];
    assert_eq!(apply["readOnlyHint"], false);
    assert_eq!(apply["destructiveHint"], false);
    assert_eq!(apply["idempotentHint"], false);
    assert_eq!(apply["openWorldHint"], false);
    for tool in tools {
        let properties = tool["inputSchema"]["properties"].as_object().unwrap();
        assert!(!properties.contains_key("path"), "{tool}");
    }
    session.close();
}

#[test]
fn a_confirmed_apply_writes_one_undoable_working_change_and_keeps_the_saved_root() {
    let dir = tempfile::tempdir().unwrap();
    let path = common::save(dir.path(), "home.knxdb", &common::project("Home"));
    let (_, _, _, saved_before) = native(&path);
    let mut session = Session::start(&[
        "--project",
        &flag("home", &path),
        "--no-product-db",
        "--allow-name-recipe",
        "home",
    ]);
    session.init(true);
    let plan = preview(&mut session, 2, "home", &[1, 2], "EG ");
    assert_eq!(plan["preview"]["changed"], 2, "{plan}");
    assert_eq!(
        plan["preview"]["rows"][0]["after"],
        "EG Kitchen light switch"
    );
    assert_eq!(plan["preview"]["rows"][1]["before"], "Living room blinds");
    let token = plan["confirmationToken"].clone();

    session.request_call(
        3,
        "apply_name_recipe",
        apply_args("home", &[1, 2], "EG ", &token),
    );
    let request = session.answer_elicitation(approve(true));
    let message = request["params"]["message"].as_str().unwrap();
    assert!(
        message.contains("home") && message.contains('2'),
        "{message}"
    );
    assert_eq!(
        request["params"]["requestedSchema"]["properties"]["approve"]["type"], "boolean",
        "{request}"
    );
    let applied = session.response(3);
    assert!(!is_error(&applied), "{applied}");
    let result = structured(&applied);
    assert_eq!(result["applied"], true, "{result}");
    assert_eq!(result["persistence"], "native-working-history");
    assert_eq!(result["savedRootUnchanged"], true);
    session.close();

    let (working, undo, generation, saved) = native(&path);
    assert_eq!(working[&1], "EG Kitchen light switch");
    assert_eq!(working[&2], "EG Living room blinds");
    assert_eq!(working[&3], "Unused spare");
    assert_eq!(undo, 1, "exactly one undoable batch");
    assert!(generation > 0);
    assert_eq!(saved, saved_before, "the saved root is untouched");

    // Reopen: a fresh server previews from the persisted working state.
    let mut again = Session::start(&[
        "--project",
        &flag("home", &path),
        "--no-product-db",
        "--allow-name-recipe",
        "home",
    ]);
    again.init(true);
    let reopened = preview(&mut again, 2, "home", &[1], "EG ");
    assert_eq!(
        reopened["preview"]["rows"][0]["before"],
        "EG Kitchen light switch"
    );
    again.close();
}

#[test]
fn decline_cancel_disapproval_and_a_missing_capability_never_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = common::save(dir.path(), "home.knxdb", &common::project("Home"));
    let before = std::fs::read(&path).unwrap();
    let args = [
        "--project",
        &flag("home", &path),
        "--no-product-db",
        "--allow-name-recipe",
        "home",
    ];
    let mut session = Session::start(&args);
    session.init(true);
    let token = preview(&mut session, 2, "home", &[1], "EG ")["confirmationToken"].clone();
    let answers = [
        json!({ "action": "decline" }),
        json!({ "action": "cancel" }),
        approve(false),
    ];
    for (offset, answer) in answers.into_iter().enumerate() {
        let id = 10 + offset as u64;
        session.request_call(
            id,
            "apply_name_recipe",
            apply_args("home", &[1], "EG ", &token),
        );
        session.answer_elicitation(answer);
        let refused = session.response(id);
        assert!(is_error(&refused), "{refused}");
    }
    session.close();
    assert_eq!(std::fs::read(&path).unwrap(), before);

    // A client that cannot show a form is refused before any prompt.
    let mut blind = Session::start(&args);
    blind.init(false);
    let token = preview(&mut blind, 2, "home", &[1], "EG ")["confirmationToken"].clone();
    let refused = blind.call(
        3,
        "apply_name_recipe",
        apply_args("home", &[1], "EG ", &token),
    );
    assert!(is_error(&refused), "{refused}");
    let text = refused.to_string();
    assert!(text.contains("confirm"), "{text}");
    blind.close();
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn stale_tokens_and_changes_during_confirmation_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = common::save(dir.path(), "home.knxdb", &common::project("Home"));
    let before = std::fs::read(&path).unwrap();
    let args = [
        "--project",
        &flag("home", &path),
        "--no-product-db",
        "--allow-name-recipe",
        "home",
    ];
    let mut session = Session::start(&args);
    session.init(true);

    // A token for another intent is stale: no prompt, no write.
    let token = preview(&mut session, 2, "home", &[1], "EG ")["confirmationToken"].clone();
    let refused = session.call(
        3,
        "apply_name_recipe",
        apply_args("home", &[2], "EG ", &token),
    );
    assert!(is_error(&refused), "{refused}");
    assert_eq!(std::fs::read(&path).unwrap(), before);

    // Another writer lands while the human is still deciding.
    session.request_call(
        4,
        "apply_name_recipe",
        apply_args("home", &[1], "EG ", &token),
    );
    let request = session.next();
    assert_eq!(request["method"], "elicitation/create", "{request}");
    let mut other = Session::start(&args);
    other.init(true);
    let other_token = preview(&mut other, 2, "home", &[3], "OG ")["confirmationToken"].clone();
    other.request_call(
        3,
        "apply_name_recipe",
        apply_args("home", &[3], "OG ", &other_token),
    );
    other.answer_elicitation(approve(true));
    assert!(!is_error(&other.response(3)));
    other.close();
    session.send(json!({ "jsonrpc": "2.0", "id": request["id"], "result": approve(true) }));
    let conflict = session.response(4);
    assert!(is_error(&conflict), "{conflict}");
    session.close();

    let (working, undo, _, _) = native(&path);
    assert_eq!(
        working[&1], "Kitchen light switch",
        "the stale apply wrote nothing"
    );
    assert_eq!(working[&3], "OG Unused spare");
    assert_eq!(undo, 1);
}

#[test]
fn only_enabled_aliases_accept_previews_or_writes() {
    let dir = tempfile::tempdir().unwrap();
    let home = common::save(dir.path(), "home.knxdb", &common::project("Home"));
    let office = common::save(dir.path(), "office.knxdb", &common::project("Office"));
    let before = std::fs::read(&office).unwrap();
    let mut session = Session::start(&[
        "--project",
        &flag("home", &home),
        "--project",
        &flag("office", &office),
        "--no-product-db",
        "--allow-name-recipe",
        "home",
    ]);
    session.init(true);
    let preview = session.call(2, "preview_name_recipe", intent("office", &[1], "X "));
    assert!(is_error(&preview), "{preview}");
    let apply = session.call(
        3,
        "apply_name_recipe",
        apply_args("office", &[1], "X ", &json!("0".repeat(64))),
    );
    assert!(is_error(&apply), "{apply}");
    session.close();
    assert_eq!(std::fs::read(&office).unwrap(), before);
}

#[test]
fn invalid_intents_are_refused_without_a_prompt_or_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = common::save(dir.path(), "home.knxdb", &common::project("Home"));
    let before = std::fs::read(&path).unwrap();
    let mut session = Session::start(&[
        "--project",
        &flag("home", &path),
        "--no-product-db",
        "--allow-name-recipe",
        "home",
    ]);
    session.init(true);
    let long = "x".repeat(4097);
    let cases = [
        intent("home", &[], "X "),
        intent("home", &[99], "X "),
        intent("home", &[1, 1], "X "),
        intent("home", &[1], &long),
        json!({ "project": "home", "groupAddresses": [1], "prefix": "X ", "path": "/etc/passwd" }),
    ];
    for (offset, case) in cases.into_iter().enumerate() {
        let response = session.call(10 + offset as u64, "preview_name_recipe", case);
        assert!(is_error(&response), "{response}");
    }
    let malformed = session.call(
        20,
        "apply_name_recipe",
        apply_args("home", &[1], "X ", &json!("NOT-A-TOKEN")),
    );
    assert!(is_error(&malformed), "{malformed}");
    session.close();
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_persistence_failure_after_approval_leaves_the_file_unchanged() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = common::save(dir.path(), "home.knxdb", &common::project("Home"));
    let before = std::fs::read(&path).unwrap();
    let mut session = Session::start(&[
        "--project",
        &flag("home", &path),
        "--no-product-db",
        "--allow-name-recipe",
        "home",
    ]);
    session.init(true);
    let token = preview(&mut session, 2, "home", &[1], "EG ")["confirmationToken"].clone();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
    assert!(
        std::fs::OpenOptions::new().write(true).open(&path).is_err(),
        "precondition: this test must not run with privileges that ignore file modes"
    );
    session.request_call(
        3,
        "apply_name_recipe",
        apply_args("home", &[1], "EG ", &token),
    );
    session.answer_elicitation(approve(true));
    let failed = session.response(3);
    assert!(is_error(&failed), "{failed}");
    session.close();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
