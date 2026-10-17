//! Runs the real `knx adopt-plugin-data` binary against disposable native projects.

use knx_testsupport::{PluginFixture, PLUGIN_FIXTURE_BLOB_PATH, PLUGIN_FIXTURE_RBG_PATH};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture {
    _dir: tempfile::TempDir,
    source: PathBuf,
    store: PathBuf,
}

fn fixture(target: &PluginFixture, source: &PluginFixture) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let target_archive = dir.path().join("target.knxproj");
    std::fs::write(&target_archive, target.knxproj_bytes()).unwrap();
    let source_archive = dir.path().join("source.knxproj");
    std::fs::write(&source_archive, source.knxproj_bytes()).unwrap();
    let store = dir.path().join("target.knxdb");
    let conn = knx_store::open_and_migrate(&store).unwrap();
    let imported = knx_app::import_ets_project(&target_archive, &conn).unwrap();
    knx_store::save_project(&conn, &imported.project).unwrap();
    Fixture {
        _dir: dir,
        source: source_archive,
        store,
    }
}

fn knx(fixture: &Fixture, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .arg("adopt-plugin-data")
        .arg(&fixture.source)
        .arg("--project")
        .arg(&fixture.store)
        .args(extra)
        .output()
        .unwrap()
}

fn json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn stored(store: &Path, path: &str) -> Vec<u8> {
    let conn = knx_store::open_existing_read_only(store).unwrap().conn;
    let editor = knx_store::project_history::load_editor(&conn).unwrap();
    let opaque = match editor {
        Some(editor) => editor.working.opaque,
        None => knx_store::load_opaque(&conn).unwrap(),
    };
    opaque
        .into_iter()
        .find(|e| e.xpath.is_empty() && e.source_path == path)
        .unwrap()
        .bytes
}

fn devices(listing: &Value) -> (String, String) {
    let first = &listing["candidates"][0];
    (
        first["sourceDevice"]["id"].to_string(),
        first["targetDevice"]["id"].to_string(),
    )
}

#[test]
fn listing_names_the_device_pairs_that_could_take_the_bytes_and_writes_nothing() {
    let f = fixture(&PluginFixture::default(), &PluginFixture::edited());
    let before = std::fs::read(&f.store).unwrap();
    let listing = json(&knx(&f, &[]));
    assert_eq!(listing["candidates"].as_array().unwrap().len(), 1);
    assert_eq!(listing["candidates"][0]["refusal"], Value::Null);
    assert_eq!(listing["candidates"][0]["replacements"], 2);
    assert_eq!(std::fs::read(&f.store).unwrap(), before);
}

#[test]
fn preview_writes_nothing_and_a_confirmed_adoption_replaces_exactly_the_plugin_bytes() {
    let f = fixture(&PluginFixture::default(), &PluginFixture::edited());
    let (source_device, target_device) = devices(&json(&knx(&f, &[])));
    let args = [
        "--source-device",
        &source_device,
        "--target-device",
        &target_device,
    ];
    let before = std::fs::read(&f.store).unwrap();
    let preview = json(&knx(&f, &args));
    assert_eq!(preview["applied"], false);
    assert_eq!(
        preview["preview"]["replacements"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        std::fs::read(&f.store).unwrap(),
        before,
        "a preview is read-only"
    );
    let token = preview["confirmationToken"].as_str().unwrap().to_string();

    let applied = json(&knx(
        &f,
        &[args[0], args[1], args[2], args[3], "--confirm", &token],
    ));
    assert_eq!(applied["applied"], true);

    let edited = PluginFixture::edited();
    assert_eq!(stored(&f.store, PLUGIN_FIXTURE_BLOB_PATH), edited.blob);
    assert_eq!(stored(&f.store, PLUGIN_FIXTURE_RBG_PATH), edited.rbg);

    // The previous saved state, with the old bytes, is a restorable version.
    let conn = knx_store::open_existing_read_only(&f.store).unwrap().conn;
    let versions = knx_store::project_history::list_versions(&conn).unwrap();
    assert_eq!(versions.len(), 1, "{versions:?}");
    let old = knx_store::project_history::read_version(&conn, versions[0].id).unwrap();
    let old_blob = old
        .opaque
        .iter()
        .find(|e| e.xpath.is_empty() && e.source_path == PLUGIN_FIXTURE_BLOB_PATH)
        .unwrap();
    assert_eq!(old_blob.bytes, PluginFixture::default().blob);

    // The same preview is now nothing to do.
    let again = knx(&f, &args);
    assert!(!again.status.success());
    assert!(String::from_utf8_lossy(&again.stderr).contains("nothing to adopt"));
}

#[test]
fn wrong_token_incompatible_devices_and_missing_projects_change_nothing() {
    let f = fixture(&PluginFixture::default(), &PluginFixture::edited());
    let (source_device, target_device) = devices(&json(&knx(&f, &[])));
    let before = std::fs::read(&f.store).unwrap();

    let wrong = knx(
        &f,
        &[
            "--source-device",
            &source_device,
            "--target-device",
            &target_device,
            "--confirm",
            "wrong",
        ],
    );
    assert!(!wrong.status.success());
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("preview again"));
    let unknown = knx(
        &f,
        &[
            "--source-device",
            "999999",
            "--target-device",
            &target_device,
        ],
    );
    assert!(!unknown.status.success());
    assert_eq!(std::fs::read(&f.store).unwrap(), before);

    // A device linked to a different group address is listed, with the reason, and refused.
    let linked = fixture(
        &PluginFixture::default(),
        &PluginFixture {
            group_address: 2,
            ..PluginFixture::edited()
        },
    );
    let listing = json(&knx(&linked, &[]));
    assert!(listing["candidates"][0]["refusal"]
        .as_str()
        .unwrap()
        .contains("group addresses"));
    let (s, t) = devices(&listing);
    let refused = knx(&linked, &["--source-device", &s, "--target-device", &t]);
    assert!(!refused.status.success());

    let missing = f.store.with_file_name("missing.knxdb");
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .arg("adopt-plugin-data")
        .arg(&f.source)
        .arg("--project")
        .arg(&missing)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!missing.exists());
}

#[test]
fn argument_mistakes_are_usage_errors() {
    let f = fixture(&PluginFixture::default(), &PluginFixture::edited());
    for bad in [
        vec!["--source-device", "1"],
        vec!["--confirm", "x"],
        vec!["--password", "secret"],
        vec!["--target-device", "x", "--source-device", "1"],
    ] {
        let output = knx(&f, &bad);
        assert!(!output.status.success(), "{bad:?}");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("secret"));
    }
}
