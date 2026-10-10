//! Runs the actual selective-import CLI against seeded disposable native projects.

use serde_json::Value;
use std::process::Command;

fn fixture() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    u8,
    u32,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let source = knx_etsproj::import_knxproj(&path).unwrap();
    let installation = source.project.installations[0].id.0;
    let device = source.project.devices.iter().next().unwrap().id.0;
    let mut target = source.project.clone();
    target.info.name = "Keep existing target".into();
    target.devices = knx_core::Devices::new();
    for i in &mut target.installations {
        i.parameters.clear();
        i.group_addresses.clear();
        i.group_ranges.clear();
        i.buildings.clear();
        i.topology.unassigned.clear();
        for l in &mut i.topology.lines {
            l.devices.clear();
        }
    }
    let store = dir.path().join("target.knxdb");
    let conn = knx_store::open_and_migrate(&store).unwrap();
    knx_store::save_project(&conn, &target).unwrap();
    drop(conn);
    (dir, path, store, installation, device)
}

fn command(
    path: &std::path::Path,
    store: &std::path::Path,
    installation: u8,
    device: u32,
) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_knx"));
    c.arg("import-selection")
        .arg(path)
        .arg("--project")
        .arg(store)
        .arg("--source-installation")
        .arg(installation.to_string())
        .arg("--target-installation")
        .arg(installation.to_string())
        .arg("--device")
        .arg(device.to_string());
    c
}

#[test]
fn preview_does_not_write_and_confirmed_import_preserves_identity_and_undo() {
    let (_dir, path, store, installation, device) = fixture();
    let bytes = std::fs::read(&store).unwrap();
    let preview = command(&path, &store, installation, device)
        .output()
        .unwrap();
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let report: Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(report["preview"]["counts"]["devices"], 1);
    assert_eq!(std::fs::read(&store).unwrap(), bytes);
    let token = report["confirmationToken"].as_str().unwrap();
    let applied = command(&path, &store, installation, device)
        .arg("--confirm")
        .arg(token)
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let conn = knx_store::open_existing_read_only(&store).unwrap().conn;
    let history = knx_store::project_history::load_editor(&conn)
        .unwrap()
        .unwrap();
    assert_eq!(history.working.project.devices.iter().count(), 1);
    assert_eq!(history.working.project.info.name, "Keep existing target");
    assert_eq!(history.undo.len(), 1);
    assert!(history.undo[0].devices.iter().next().is_none());
    assert!(history
        .working
        .opaque
        .iter()
        .any(|e| e.kind == "SelectiveImportArchive"
            && e.bytes == knx_testsupport::minimal_knxproj_bytes()));
}

#[test]
fn wrong_confirmation_unknown_device_and_missing_target_create_nothing() {
    let (dir, path, store, installation, device) = fixture();
    let bytes = std::fs::read(&store).unwrap();
    let refused = command(&path, &store, installation, device)
        .arg("--confirm")
        .arg("wrong-token")
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert_eq!(std::fs::read(&store).unwrap(), bytes);
    let unknown = command(&path, &store, installation, u32::MAX)
        .output()
        .unwrap();
    assert!(!unknown.status.success());
    assert_eq!(std::fs::read(&store).unwrap(), bytes);
    let missing = dir.path().join("missing.knxdb");
    assert!(!command(&path, &missing, installation, device)
        .output()
        .unwrap()
        .status
        .success());
    assert!(!missing.exists());
}

#[test]
fn source_inventory_uses_the_actual_binary_without_a_target_or_product_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["import-selection", "inspect"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let inventory: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        inventory["installations"][0]["devices"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(inventory["sourceHash"].is_string());
}
