//! HTTP regression tests for the opt-in saved-project section of the diagnostic bundle.

use super::{call, json_of, state};
use axum::http::StatusCode;
use knx_store::StoredOpaqueEntry;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const BCU_KEY: &str = "2718281828";
const TOOL_KEY: &str = "SNTLtoolKeyProjectBundle01";
const TUNNEL_PASSWORD: &str = "SNTLtunnelPwProjectBundle02";
const SECRETS: [&str; 3] = [BCU_KEY, TOOL_KEY, TUNNEL_PASSWORD];
const FREE_ACCESS_KEY: &str = "4294967295";

const SAVED_NAME: &str = "Saved root project, edited";
const WORKING_NAME: &str = "Unsaved working project name";
const SAVED_DEVICE: &str = "Kitchen actuator";
const WORKING_DEVICE: &str = "Unsaved working actuator";

const MEMBER: &str = "project/project.knxdb";
const SAVED_ROOT_NOTE: &str =
    "the last saved state of the project; unsaved changes are not included";
const UNAVAILABLE_PREFIX: &str = "the selected diagnostic source is unavailable";

fn topology() -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21">
  <Project Id="P-0001"><Installations>
    <Installation InstallationId="0" Name="Main" BCUKey="{FREE_ACCESS_KEY}" DiagnosticWitness="safe neighbour">
      <Topology><Area Id="P-0001-0_A-1" Address="1">
        <Line Id="P-0001-0_L-2" Address="1">
          <Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0">
            <BusAccess Name="Synthetic interface" Edi="synthetic"
                Parameter="Target=203.0.113.250;Port=3671"/>
            <DeviceInstance Id="P-0001-0_DI-1" Name="{SAVED_DEVICE}" Address="1"
                ProductRefId="M-TEST_H-1_P-1" Hardware2ProgramRefId="M-TEST_H-1_HP-1">
              <Security SequenceNumber="7"/>
            </DeviceInstance>
          </Segment>
        </Line>
      </Area></Topology>
      <GroupAddresses><GroupRanges>
        <GroupRange Id="P-0001-0_GR-1" RangeStart="1" RangeEnd="100" Name="Lights">
          <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="Kitchen light"/>
        </GroupRange>
      </GroupRanges></GroupAddresses>
    </Installation>
  </Installations></Project>
</KNX>"#
    )
}

/// A native project file saved twice (so it has history) beside unsaved working state.
struct Fixture {
    state: Arc<crate::AppState>,
    path: PathBuf,
    saved: knx_core::Project,
    saved_opaque: Vec<StoredOpaqueEntry>,
    _dir: tempfile::TempDir,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let input = work.join("source.knxproj");
    let info = br#"<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001">
<ProjectInformation Name="Synthetic project bundle" GroupAddressStyle="ThreeLevel"/>
</Project></KNX>"#;
    let topology = topology();
    std::fs::write(
        &input,
        knx_testsupport::zip_with_entries(&[
            ("P-0001.signature", b"synthetic signature"),
            ("P-0001/0.xml", topology.as_bytes()),
            ("P-0001/Project.xml", info),
        ]),
    )
    .unwrap();
    let staged = knx_store::open_and_migrate_in_memory().unwrap();
    let imported = knx_app::import_ets_project(&input, &staged).unwrap();
    let mut opaque = knx_store::load_opaque(&staged).unwrap();
    knx_app::import_evidence::capture_primary(&mut opaque, &imported.report).unwrap();
    let refs = knx_store::load_manufacturer_refs(&staged).unwrap();
    let path = work.join("project.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project_with_passthrough(&conn, &imported.project, &opaque, &refs).unwrap();
    let mut saved = imported.project.clone();
    saved.info.name = SAVED_NAME.into();
    knx_store::save_project_with_passthrough(&conn, &saved, &opaque, &refs).unwrap();
    drop(conn);

    // The working copy diverges from the saved root and was never saved.
    let mut working = saved.clone();
    working.info.name = WORKING_NAME.into();
    let ids: Vec<_> = working.devices.iter().map(|device| device.id).collect();
    assert!(!ids.is_empty(), "fixture needs a device");
    for id in ids {
        working.devices.get_mut(id).unwrap().name = WORKING_DEVICE.into();
    }
    let state = state(dir.path(), true);
    *state.project.lock().unwrap() = Some(working);
    *state.opaque.lock().unwrap() = opaque.clone();
    *state.store_path.lock().unwrap() = Some(path.clone());
    Fixture {
        state,
        path,
        saved,
        saved_opaque: opaque,
        _dir: dir,
    }
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

fn project_query() -> Value {
    json!({ "problemType": "projectImport", "sections": ["project"] })
}

/// Previews `query` and returns the preview and the query carrying its own review.
async fn reviewed(state: &Arc<crate::AppState>, query: &Value) -> (Value, Value) {
    let (status, body, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let preview = json_of(&body);
    let mut with_review = query.clone();
    with_review["review"] = preview["review"].clone();
    (preview, with_review)
}

async fn section_text(
    state: &Arc<crate::AppState>,
    query: &Value,
    key: &str,
) -> (StatusCode, Value) {
    let mut query = query.clone();
    query["key"] = json!(key);
    let (status, body, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(query),
    )
    .await;
    (status, json_of(&body))
}

async fn export(state: &Arc<crate::AppState>, query: &Value) -> (StatusCode, Vec<u8>) {
    let mut query = query.clone();
    query["consent"] = json!(true);
    let (status, body, _) = call(state, "POST", "/api/diagnostics/bundle", Some(query)).await;
    (status, body)
}

fn members(zip: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).unwrap();
    (0..archive.len())
        .map(|index| {
            let mut file = archive.by_index(index).unwrap();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            (file.name().to_string(), bytes)
        })
        .collect()
}

fn offer<'a>(preview: &'a Value, kind: &str) -> &'a Value {
    preview["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|offer| offer["kind"] == kind)
        .unwrap()
}

fn project_preview(preview: &Value) -> Option<&Value> {
    preview["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["kind"] == "project")
}

fn manifest_project(archive: &BTreeMap<String, Vec<u8>>) -> Value {
    let manifest: Value = serde_json::from_slice(&archive["manifest.json"]).unwrap();
    manifest["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["kind"] == "project")
        .cloned()
        .expect("manifest lists the project section")
}

/// The unpacked inner project file, reopened through the native store.
struct Reopened {
    project: knx_core::Project,
    opaque: Vec<StoredOpaqueEntry>,
    versions: usize,
    editor: bool,
}

fn reopen(bytes: &[u8]) -> Reopened {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inner.knxdb");
    std::fs::write(&path, bytes).unwrap();
    let store = knx_store::open_existing_read_only(&path).unwrap();
    Reopened {
        project: knx_store::load_project(&store.conn).unwrap(),
        opaque: knx_store::load_opaque(&store.conn).unwrap(),
        versions: knx_store::project_history::list_versions(&store.conn)
            .unwrap()
            .len(),
        editor: knx_store::project_history::load_editor(&store.conn)
            .unwrap()
            .is_some(),
    }
}

fn history_state(path: &Path) -> (usize, bool) {
    let store = knx_store::open_existing_read_only(path).unwrap();
    (
        knx_store::project_history::list_versions(&store.conn)
            .unwrap()
            .len(),
        knx_store::project_history::load_editor(&store.conn)
            .unwrap()
            .is_some(),
    )
}

#[tokio::test]
async fn saved_project_is_available_only_through_explicit_preview() {
    let f = fixture();
    let (preview, _) = reviewed(&f.state, &project_query()).await;
    let project = offer(&preview, "project");
    assert_eq!(project["offer"], "offered");
    assert_eq!(project["selected"], true, "{project}");
    assert!(project["unavailable"].is_null(), "{project}");
    let section = project_preview(&preview).expect("explicit selection previews the project");
    assert_eq!(section["files"], json!(["project.knxdb"]));
    assert!(section["bytes"].as_u64().unwrap() > 0);
    assert_eq!(section["entries"], 1);
}

#[tokio::test]
async fn explicit_project_export_carries_a_reopenable_secret_free_project() {
    let f = fixture();
    let (_, query) = reviewed(&f.state, &project_query()).await;
    let (status, zip) = export(&f.state, &query).await;
    assert_eq!(status, StatusCode::OK);
    let archive = members(&zip);
    let inner = archive
        .get(MEMBER)
        .unwrap_or_else(|| panic!("no {MEMBER} in {:?}", archive.keys()));

    let manifest = manifest_project(&archive);
    let file = &manifest["files"][0];
    assert_eq!(file["path"], MEMBER);
    assert_eq!(file["bytes"], inner.len());
    assert_eq!(file["sha256"], sha(inner));
    let notes: Vec<&str> = manifest["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|note| note.as_str().unwrap())
        .collect();
    assert!(notes.contains(&SAVED_ROOT_NOTE), "{notes:?}");
    assert!(
        notes
            .iter()
            .any(|note| note.starts_with("left out: project history:")),
        "history omission stays visible: {notes:?}"
    );
    assert!(notes.iter().all(|note| !note.starts_with("removed ")));
    assert_eq!(
        manifest["markers"],
        json!({
            "addresses": true, "names": true, "networkAddresses": true, "parameterValues": true
        }),
        "{manifest}"
    );

    let reopened = reopen(inner);
    // Saved root values, never the unsaved working values.
    assert_eq!(reopened.project, f.saved, "typed model identity");
    assert_eq!(reopened.project.info.name, SAVED_NAME);
    assert!(reopened
        .project
        .devices
        .iter()
        .all(|device| device.name == SAVED_DEVICE));
    assert_eq!(reopened.versions, 0);
    assert!(!reopened.editor);
    // Retained opaque XML keeps non-secret neighbours but none of the secret values.
    let retained: Vec<u8> = reopened
        .opaque
        .iter()
        .flat_map(|row| row.bytes.clone())
        .collect();
    for needle in [
        "safe neighbour",
        "Target=203.0.113.250",
        "Port=3671",
        SAVED_DEVICE,
    ] {
        assert!(contains(&retained, needle), "retained data lost {needle}");
    }
    assert_eq!(reopened.opaque.len(), f.saved_opaque.len());
    for row in &reopened.opaque {
        assert_eq!(
            row.sha256,
            knx_store::project_history::image_hash(&row.bytes)
        );
    }
    // Every archive member, the raw inner file and each decoded retained row.
    for (name, bytes) in &archive {
        for secret in SECRETS {
            assert!(!contains(bytes, secret), "{secret} leaked into {name}");
        }
        for text in [WORKING_NAME, WORKING_DEVICE] {
            assert!(!contains(bytes, text), "unsaved {text} leaked into {name}");
        }
    }
    for row in &reopened.opaque {
        for secret in SECRETS {
            assert!(
                !contains(&row.bytes, secret),
                "{secret} in {}",
                row.source_path
            );
        }
    }
}

#[tokio::test]
async fn project_import_producer_preview_text_review_zip_reopen() {
    let f = fixture();
    let source_before = std::fs::read(&f.path).unwrap();
    let query = json!({
        "problemType": "projectImport",
        "sections": ["project", "importReport"],
    });
    let (preview, reviewed) = reviewed(&f.state, &query).await;
    for kind in ["project", "importReport"] {
        let offer = offer(&preview, kind);
        assert_eq!(offer["selected"], true, "{offer}");
        assert_eq!(offer["unavailable"], Value::Null, "{offer}");
        let section = preview["sections"]
            .as_array()
            .unwrap()
            .iter()
            .find(|section| section["kind"] == kind)
            .unwrap();
        let (status, text) =
            section_text(&f.state, &reviewed, section["key"].as_str().unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!text["text"].as_str().unwrap().is_empty());
    }

    let (status, zip) = export(&f.state, &reviewed).await;
    assert_eq!(status, StatusCode::OK);
    let archive = members(&zip);
    let project = archive.get(MEMBER).expect("admitted native project");
    let report = archive
        .get("import/import-report.json")
        .expect("admitted import report");
    let _: Value = serde_json::from_slice(report).unwrap();
    let reopened = reopen(project);
    assert_eq!(reopened.project, f.saved);
    assert_eq!(reopened.opaque, f.saved_opaque);
    assert_eq!(std::fs::read(&f.path).unwrap(), source_before);
}

#[tokio::test]
async fn section_text_and_notes_describe_the_saved_root_without_secrets() {
    let f = fixture();
    let (preview, query) = reviewed(&f.state, &project_query()).await;
    let notes = project_preview(&preview).expect("project previewed")["notes"].to_string();
    assert!(notes.contains(SAVED_ROOT_NOTE), "{notes}");
    assert!(notes.contains("project history"), "{notes}");
    let (status, view) = section_text(&f.state, &query, "project").await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert!(view["text"]
        .as_str()
        .unwrap()
        .starts_with("===== project/project.knxdb ====="));
    let everything = format!("{preview}{view}");
    for secret in SECRETS {
        assert!(!everything.contains(secret), "{secret} in preview/section");
    }
}

#[tokio::test]
async fn defaults_leave_the_project_unchecked_and_the_default_zip_has_no_knxdb() {
    let f = fixture();
    let query = json!({ "problemType": "projectImport" });
    let (preview, query) = reviewed(&f.state, &query).await;
    let project = offer(&preview, "project");
    assert_eq!(project["offer"], "offered");
    assert_eq!(project["selected"], false, "{project}");
    assert!(project_preview(&preview).is_none());
    let (status, zip) = export(&f.state, &query).await;
    assert_eq!(status, StatusCode::OK);
    let archive = members(&zip);
    assert!(
        archive
            .keys()
            .all(|name| !name.ends_with(".knxdb") && !name.starts_with("project/")),
        "{:?}",
        archive.keys()
    );
    let manifest: Value = serde_json::from_slice(&archive["manifest.json"]).unwrap();
    assert!(manifest["sections"]
        .as_array()
        .unwrap()
        .iter()
        .all(|section| section["kind"] != "project"));
    for (name, bytes) in &archive {
        for secret in SECRETS {
            assert!(!contains(bytes, secret), "{secret} in {name}");
        }
    }
}

#[tokio::test]
async fn export_leaves_source_bytes_history_and_working_state_untouched() {
    let f = fixture();
    let before = std::fs::read(&f.path).unwrap();
    let history = history_state(&f.path);
    assert!(history.0 > 0 || history.1, "fixture must carry history");
    let working = f.state.project.lock().unwrap().clone();
    let opaque = f.state.opaque.lock().unwrap().clone();
    let (_, query) = reviewed(&f.state, &project_query()).await;
    assert_eq!(
        section_text(&f.state, &query, "project").await.0,
        StatusCode::OK
    );
    assert_eq!(export(&f.state, &query).await.0, StatusCode::OK);
    let after = std::fs::read(&f.path).unwrap();
    assert_eq!(sha(&after), sha(&before));
    assert_eq!(after.len(), before.len());
    assert_eq!(history_state(&f.path), history);
    assert_eq!(*f.state.project.lock().unwrap(), working);
    assert_eq!(*f.state.opaque.lock().unwrap(), opaque);
    assert_eq!(
        f.state.project.lock().unwrap().as_ref().unwrap().info.name,
        WORKING_NAME
    );
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar = f.path.clone().into_os_string();
        sidecar.push(suffix);
        assert!(
            !Path::new(&sidecar).exists(),
            "export created a {suffix} file"
        );
    }
}

#[tokio::test]
async fn repeated_preview_section_and_export_of_an_unchanged_source_are_stable() {
    let f = fixture();
    let (first, query) = reviewed(&f.state, &project_query()).await;
    let (second, same_query) = reviewed(&f.state, &project_query()).await;
    assert_eq!(first["review"], second["review"]);
    assert_eq!(project_preview(&first), project_preview(&second));
    assert_eq!(query["review"], same_query["review"]);
    let (_, text_one) = section_text(&f.state, &query, "project").await;
    let (_, text_two) = section_text(&f.state, &same_query, "project").await;
    assert_eq!(text_one, text_two);
    let (status, zip_one) = export(&f.state, &query).await;
    assert_eq!(status, StatusCode::OK);
    let (status, zip_two) = export(&f.state, &same_query).await;
    assert_eq!(status, StatusCode::OK);
    let (one, two) = (members(&zip_one), members(&zip_two));
    assert_eq!(
        one.keys().collect::<Vec<_>>(),
        two.keys().collect::<Vec<_>>()
    );
    assert_eq!(one[MEMBER], two[MEMBER]);
    assert_eq!(manifest_project(&one), manifest_project(&two));
}

#[tokio::test]
async fn a_project_review_is_bound_to_its_selection_consent_and_saved_bytes() {
    let f = fixture();
    let (_, query) = reviewed(&f.state, &project_query()).await;

    // No review, no consent.
    let mut no_review = project_query();
    no_review["consent"] = json!(true);
    let (status, body, _) =
        call(&f.state, "POST", "/api/diagnostics/bundle", Some(no_review)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("bundlePreviewRequired"));
    let (status, body, _) = call(
        &f.state,
        "POST",
        "/api/diagnostics/bundle",
        Some(query.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("bundleConsentRequired"));
    let mut text_without_review = project_query();
    text_without_review["key"] = json!("project");
    let (status, body, _) = call(
        &f.state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(text_without_review),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("bundlePreviewRequired"));

    // A review for another selection does not authorize the project.
    let (_, default_query) = reviewed(&f.state, &json!({ "problemType": "projectImport" })).await;
    let mut borrowed = project_query();
    borrowed["review"] = default_query["review"].clone();
    let (status, body) = export(&f.state, &borrowed).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert!(String::from_utf8_lossy(&body).contains("bundlePreviewChanged"));
    assert_eq!(
        section_text(&f.state, &borrowed, "project").await.0,
        StatusCode::CONFLICT
    );

    // Saving the project again changes the reviewed bytes.
    let mut edited = f.saved.clone();
    edited.info.name = "Saved again after the preview".into();
    {
        let conn = knx_store::open_and_migrate(&f.path).unwrap();
        let refs = Vec::new();
        knx_store::save_project_with_passthrough(&conn, &edited, &f.saved_opaque, &refs).unwrap();
    }
    let (status, _) = export(&f.state, &query).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        section_text(&f.state, &query, "project").await.0,
        StatusCode::CONFLICT
    );
}

/// Asserts the project is visibly unavailable over HTTP, with a sanitized reason.
async fn assert_refused(f: &Fixture, _reason: &str) {
    let (preview, query) = reviewed(&f.state, &project_query()).await;
    let project = offer(&preview, "project");
    assert_eq!(project["selected"], false, "{project}");
    let message = project["unavailable"].as_str().expect("visible refusal");
    assert_eq!(message, UNAVAILABLE_PREFIX);
    assert!(project_preview(&preview).is_none());
    let everything = format!("{preview}");
    for secret in SECRETS {
        assert!(!everything.contains(secret));
    }
    let root = f.path.parent().unwrap().to_string_lossy().into_owned();
    assert!(
        !message.contains(&root),
        "refusal names a local path: {message}"
    );
    let (status, view) = section_text(&f.state, &query, "project").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{view}");
    assert!(view.to_string().contains("bundleSectionUnknown"));
    let (status, zip) = export(&f.state, &query).await;
    assert_eq!(status, StatusCode::OK);
    let archive = members(&zip);
    assert!(
        archive
            .keys()
            .all(|name| !name.starts_with("project/") && !name.ends_with(".knxdb")),
        "no raw file fallback: {:?}",
        archive.keys()
    );
    for (name, bytes) in &archive {
        for secret in SECRETS {
            assert!(!contains(bytes, secret), "{secret} in {name}");
        }
    }
}

#[tokio::test]
async fn an_unsaved_project_is_visibly_unavailable() {
    let f = fixture();
    *f.state.store_path.lock().unwrap() = None;
    assert_refused(&f, "has not been saved").await;
}

#[tokio::test]
async fn a_malformed_project_file_is_visibly_unavailable_and_unchanged() {
    let f = fixture();
    let garbage = format!("not a database {BCU_KEY}").into_bytes();
    std::fs::write(&f.path, &garbage).unwrap();
    assert_refused(&f, UNAVAILABLE_PREFIX).await;
    assert_eq!(std::fs::read(&f.path).unwrap(), garbage);
}

#[tokio::test]
async fn a_missing_project_file_is_visibly_unavailable() {
    let f = fixture();
    std::fs::remove_file(&f.path).unwrap();
    assert_refused(&f, UNAVAILABLE_PREFIX).await;
    assert!(!f.path.exists(), "the refusal must not create the file");
}

#[tokio::test]
async fn a_wal_mode_project_file_is_visibly_unavailable_and_unchanged() {
    let f = fixture();
    {
        let conn = rusqlite::Connection::open(&f.path).unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
    }
    let mut sidecar = f.path.clone().into_os_string();
    for suffix in ["-wal", "-shm"] {
        sidecar.push(suffix);
        let _ = std::fs::remove_file(Path::new(&sidecar));
        sidecar = f.path.clone().into_os_string();
    }
    let before = std::fs::read(&f.path).unwrap();
    assert!(before[18..20].contains(&2), "fixture must be WAL-mode");
    assert_refused(&f, "WAL").await;
    assert_eq!(std::fs::read(&f.path).unwrap(), before);
}

#[tokio::test]
async fn each_journal_sidecar_makes_the_project_visibly_unavailable_and_stays_unchanged() {
    for suffix in ["-wal", "-shm", "-journal"] {
        let f = fixture();
        let mut sidecar = f.path.clone().into_os_string();
        sidecar.push(suffix);
        let sidecar = PathBuf::from(sidecar);
        let content = format!("synthetic sidecar {suffix}").into_bytes();
        std::fs::write(&sidecar, &content).unwrap();
        let root = std::fs::read(&f.path).unwrap();
        assert_refused(&f, "sidecar").await;
        assert_eq!(std::fs::read(&f.path).unwrap(), root, "{suffix} root");
        assert_eq!(
            std::fs::read(&sidecar).unwrap(),
            content,
            "{suffix} sidecar"
        );
    }
}
