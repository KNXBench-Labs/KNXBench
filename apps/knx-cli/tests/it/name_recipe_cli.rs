//! Exercises the actual offline naming CLI with private preview and durable undo.

use knx_core::{
    CommandStack, CompletionStatus, GroupAddress, GroupAddressEntry, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use knx_store::project_history::{self as history, NativeSnapshot};
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, NativeSnapshot) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recipe.knxdb");
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Kept installation".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    for (address, name) in [("1/1/1", " Küche 🛠 "), ("1/1/2", "Second")] {
        let id = project.ids.next_group_address_id().unwrap();
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id,
                source: SourceRef {
                    path: "synthetic.xml".into(),
                    ets_id: format!("GA-{}", id.0),
                },
                name: name.into(),
                address: GroupAddress::parse(address, project.info.group_address_style).unwrap(),
                central: true,
                unfiltered: true,
                range: None,
                declared_dpt: Default::default(),
            });
    }
    let snapshot = NativeSnapshot {
        project,
        opaque: vec![],
        manufacturer_refs: vec![],
    };
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &snapshot.project).unwrap();
    drop(conn);
    (dir, path, snapshot)
}
fn command(path: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_knx"));
    c.arg("name-recipe").arg(path).args([
        "--group-address",
        "2",
        "--group-address",
        "1",
        "--prefix",
        "EG ",
        "--suffix",
        " /literal",
    ]);
    c
}
fn report(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn run(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn wrong_target_intent_and_replayed_confirmation_never_mutate_native_files() {
    let (dir, path, _) = fixture();
    let args = vec![
        "name-recipe".into(),
        path.display().to_string(),
        "--group-address".into(),
        "1".into(),
        "--prefix".into(),
        "EG ".into(),
    ];
    let preview = run(&args);
    assert!(preview.status.success());
    let token: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    let token = token["confirmationToken"].as_str().unwrap().to_string();
    let clone = dir.path().join("other.knxdb");
    std::fs::copy(&path, &clone).unwrap();
    let original = std::fs::read(&path).unwrap();
    let clone_bytes = std::fs::read(&clone).unwrap();
    let mut wrong_target = args.clone();
    wrong_target[1] = clone.display().to_string();
    wrong_target.extend(["--confirm".into(), token.clone()]);
    assert!(!run(&wrong_target).status.success());
    let mut wrong_intent = args.clone();
    wrong_intent[5] = "other ".into();
    wrong_intent.extend(["--confirm".into(), token.clone()]);
    assert!(!run(&wrong_intent).status.success());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(std::fs::read(&clone).unwrap(), clone_bytes);
    let mut apply = args;
    apply.extend(["--confirm".into(), token]);
    assert!(run(&apply).status.success());
    let applied = std::fs::read(&path).unwrap();
    assert!(!run(&apply).status.success());
    assert_eq!(std::fs::read(&path).unwrap(), applied);
}
#[test]
fn malformed_duplicate_missing_or_invalid_arguments_never_write() {
    let (dir, path, _) = fixture();
    let bytes = std::fs::read(&path).unwrap();
    for tail in [
        vec![],
        vec!["--group-address", "+1"],
        vec!["--group-address", " 1"],
        vec!["--group-address", "1.0"],
        vec!["--group-address", "4294967296"],
        vec!["--group-address", "1", "--group-address", "1"],
        vec!["--group-address", "1", "--group-address", "99"],
        vec!["--group-address", "1", "--prefix", "\n"],
        vec!["--group-address", "1", "--prefix", "a", "--prefix", "b"],
        vec!["--group-address", "1", "--confirm", "bad"],
        vec!["--group-address", "1", "--unknown", "a"],
        vec!["--group-address"],
    ] {
        let mut args = vec!["name-recipe".into(), path.display().to_string()];
        args.extend(tail.into_iter().map(String::from));
        assert!(!run(&args).status.success());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    let missing = dir.path().join("must-not-create.knxdb");
    assert!(!run(&[
        "name-recipe".into(),
        missing.display().to_string(),
        "--group-address".into(),
        "1".into()
    ])
    .status
    .success());
    assert!(!missing.exists());
}
#[test]
fn confirmed_no_op_keeps_native_bytes_and_redo_unchanged() {
    let (_dir, path, mut snapshot) = fixture();
    let mut stack = knx_core::CommandStack::new();
    stack
        .do_command(
            &mut snapshot.project,
            knx_core::Command::RenameGroupAddress {
                id: knx_core::GroupAddressId(2),
                name: "pending".into(),
            },
        )
        .unwrap();
    stack.undo(&mut snapshot.project).unwrap();
    knx_app::project_history::save_as(&path, &snapshot, &stack).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let mut args = vec![
        "name-recipe".into(),
        path.display().to_string(),
        "--group-address".into(),
        "1".into(),
    ];
    let preview = run(&args);
    assert!(preview.status.success());
    let preview: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(preview["preview"]["changed"], 0);
    args.extend([
        "--confirm".into(),
        preview["confirmationToken"].as_str().unwrap().to_string(),
    ]);
    let applied = run(&args);
    assert!(applied.status.success());
    let applied: serde_json::Value = serde_json::from_slice(&applied.stdout).unwrap();
    assert_eq!(applied["changed"], false);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let read = knx_store::open_existing_read_only(&path).unwrap();
    let editor = history::load_editor(&read.conn).unwrap().unwrap();
    assert_eq!(editor.undo.len(), 0);
    assert_eq!(editor.redo.len(), 1);
}

#[test]
fn old_or_foreign_native_input_is_neither_migrated_nor_given_sidecars() {
    for bytes in [
        include_bytes!(
            "../../../../crates/knx-store/tests/fixtures/native13-before-notes-saved.sqlite"
        )
        .as_slice(),
        b"not a native store".as_slice(),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unsupported.knxdb");
        std::fs::write(&path, bytes).unwrap();
        assert!(!run(&[
            "name-recipe".into(),
            path.display().to_string(),
            "--group-address".into(),
            "1".into()
        ])
        .status
        .success());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
#[test]
fn dash_and_template_like_affixes_remain_literal_data() {
    let (_dir, path, _) = fixture();
    let args = vec![
        "name-recipe".into(),
        path.display().to_string(),
        "--group-address".into(),
        "1".into(),
        "--prefix".into(),
        "--${room}<tag> ".into(),
        "--suffix".into(),
        " {name}".into(),
    ];
    let preview = run(&args);
    let preview = report(preview);
    assert_eq!(
        preview["preview"]["rows"][0]["after"],
        "--${room}<tag>  Küche 🛠  {name}"
    );
}

#[test]
fn literal_recipe_preview_is_private_and_confirmed_batch_is_one_undo_step() {
    let (_dir, path, before) = fixture();
    let bytes = std::fs::read(&path).unwrap();
    let preview = report(command(&path).output().unwrap());
    assert_eq!(preview["applied"], false);
    assert_eq!(preview["preview"]["rows"][0]["id"], 2);
    assert_eq!(preview["preview"]["rows"][1]["before"], " Küche 🛠 ");
    assert_eq!(
        preview["preview"]["rows"][1]["after"],
        "EG  Küche 🛠  /literal"
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let token = preview["confirmationToken"].as_str().unwrap();
    let applied = report(command(&path).args(["--confirm", token]).output().unwrap());
    assert_eq!(applied["applied"], true);
    assert_eq!(applied["changed"], true);
    assert_eq!(applied["persistence"], "native-working-history");
    let conn = knx_store::open_existing_read_only(&path).unwrap().conn;
    assert_eq!(
        NativeSnapshot::read(&conn).unwrap(),
        before,
        "working history is not implicit root Save"
    );
    let editor = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(editor.undo.len(), 1);
    assert!(editor.redo.is_empty());
    assert_eq!(
        editor.working.project.installations[0].group_addresses[0].name,
        "EG  Küche 🛠  /literal"
    );
    let mut restored = editor.working.clone();
    for (actual, original) in restored.project.installations[0]
        .group_addresses
        .iter_mut()
        .zip(&before.project.installations[0].group_addresses)
    {
        actual.name = original.name.clone();
    }
    assert_eq!(restored, before, "only names may change");
    let mut state = editor.working;
    let mut stack = CommandStack::from_snapshot_states(editor.undo, editor.redo);
    stack.undo(&mut state.project).unwrap();
    assert_eq!(state, before);
    stack.redo(&mut state.project).unwrap();
    assert_eq!(
        state.project.installations[0].group_addresses[1].name,
        "EG Second /literal"
    );
}
