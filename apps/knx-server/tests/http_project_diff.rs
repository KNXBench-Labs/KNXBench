//! HTTP tests for `POST /api/project/diff` (T14 —
//! `docs/superpowers/specs/2026-09-10-project-diff-design.md`). Compares
//! the server's live, possibly edited, in-memory project (`left`) against
//! a `.knxdb` file at the given `path` (`right`) — never a re-read of
//! `store_path` (design spec §7). Builds small in-memory projects
//! directly, same pattern as `http_documentation_export.rs`; the
//! comparison-file fixture is written with `knx_store::open_and_migrate`/
//! `knx_store::save_project`, exactly as `save_project_as_impl` does
//! internally.
//!
//! CT-6 added raw `.knxproj` comparison inputs. Those fixtures are small
//! synthetic archives built here (no corpus), after the ZIP builders in
//! `crates/knx-etsproj/tests/malformed_input.rs`.

use std::io::{Cursor, Write};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

/// A project named "Test Villa" with one installation and one device
/// (`DeviceId(1)`, unassigned to any line — irrelevant to this route)
/// whose description is `description`. The one field this suite varies
/// between `left`/`right`.
fn project_with_device_description(description: &str) -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.name = "Test Villa".into();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source("d1"),
        name: "Stray Device".into(),
        description: Some(description.to_string()),
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}

fn state_with_project(project: Project) -> knx_server::AppState {
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

/// Writes `project` to a fresh `.knxdb` file at `path` — the comparison
/// fixture ("right"), independent of the server's live state.
fn write_knxdb_fixture(path: &std::path::Path, project: &Project) {
    let conn = knx_store::open_and_migrate(path).unwrap();
    knx_store::save_project(&conn, project).unwrap();
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

async fn diff_project(app: &axum::Router, path: &std::path::Path) -> axum::response::Response {
    call(
        app,
        "POST",
        "/api/project/diff",
        Some(json!({ "path": path.to_string_lossy() })),
    )
    .await
}

#[tokio::test]
async fn a_live_project_identical_to_the_comparison_file_produces_an_empty_diff() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("compare.knxdb");
    write_knxdb_fixture(&db_path, &project_with_device_description("Same"));

    let state = Arc::new(state_with_project(project_with_device_description("Same")));
    let app = knx_server::app(state, None);

    let response = diff_project(&app, &db_path).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;

    assert_eq!(body["infoChanges"].as_array().unwrap().len(), 0, "{body}");
    let installations = body["installations"].as_array().unwrap();
    assert_eq!(installations.len(), 1, "{body}");
    let installation = &installations[0];

    for table_name in [
        "areas",
        "lines",
        "groupRanges",
        "groupAddresses",
        "buildings",
    ] {
        let table = &installation[table_name];
        assert_eq!(
            table["added"].as_array().unwrap().len(),
            0,
            "{table_name}: {body}"
        );
        assert_eq!(
            table["removed"].as_array().unwrap().len(),
            0,
            "{table_name}: {body}"
        );
        assert_eq!(
            table["changed"].as_array().unwrap().len(),
            0,
            "{table_name}: {body}"
        );
        assert_eq!(
            table["ambiguous"].as_array().unwrap().len(),
            0,
            "{table_name}: {body}"
        );
    }
    let devices = &installation["devices"];
    assert_eq!(devices["added"].as_array().unwrap().len(), 0, "{body}");
    assert_eq!(devices["removed"].as_array().unwrap().len(), 0, "{body}");
    assert_eq!(devices["changed"].as_array().unwrap().len(), 0, "{body}");
    assert_eq!(devices["ambiguous"].as_array().unwrap().len(), 0, "{body}");
}

#[tokio::test]
async fn a_changed_device_description_is_named_in_the_response() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("compare.knxdb");
    write_knxdb_fixture(
        &db_path,
        &project_with_device_description("Original description"),
    );

    let state = Arc::new(state_with_project(project_with_device_description(
        "Changed description",
    )));
    let app = knx_server::app(state, None);

    let response = diff_project(&app, &db_path).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;

    let text = body.to_string();
    assert!(
        text.contains("Changed description"),
        "response must name the changed value: {body}"
    );

    let installations = body["installations"].as_array().unwrap();
    assert_eq!(installations.len(), 1, "{body}");
    let changed = installations[0]["devices"]["changed"].as_array().unwrap();
    assert_eq!(changed.len(), 1, "{body}");
    assert_eq!(
        changed[0]["changedFields"],
        json!(["description"]),
        "{body}"
    );
    assert_eq!(
        changed[0]["fieldChanges"],
        json!([{
            "field": "description",
            "left": "Changed description",
            "right": "Original description"
        }]),
        "{body}"
    );
    assert_eq!(
        changed[0]["left"]["description"], "Changed description",
        "{body}"
    );
    assert_eq!(
        changed[0]["right"]["description"], "Original description",
        "{body}"
    );
}

#[tokio::test]
async fn a_comparison_path_that_does_not_exist_is_a_400_and_creates_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let missing_path = dir.path().join("does-not-exist.knxdb");
    assert!(!missing_path.exists());

    let state = Arc::new(state_with_project(project_with_device_description("Same")));
    let app = knx_server::app(state, None);

    let response = diff_project(&app, &missing_path).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // The existence-check-gotcha regression: `knx_store::open_and_migrate`
    // creates an empty SQLite file if handed a path that doesn't exist —
    // `diff_project_impl` must reject the path before ever calling it.
    assert!(
        !missing_path.exists(),
        "a typo'd comparison path must not silently create a .knxdb file"
    );
}

#[tokio::test]
async fn calling_it_with_no_project_open_is_a_400_not_a_500() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("compare.knxdb");
    write_knxdb_fixture(&db_path, &project_with_device_description("Same"));

    let response = diff_project(&app, &db_path).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------
// CT-6: raw `.knxproj` comparison inputs (KNOWN_LIMITATIONS §57).
// ---------------------------------------------------------------------

/// One installation, one line, one device linked to one group address —
/// the same minimal document `crates/knx-etsproj/tests/malformed_input.rs`
/// imports cleanly.
const MINIMAL_INSTALLATION: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="P-0001">
    <Installations>
      <Installation InstallationId="0" Name="" DefaultLine="P-0001-0_L-2" CompletionStatus="Undefined">
        <Topology>
          <Area Id="P-0001-0_A-1" Name="A" Address="1" CompletionStatus="Undefined">
            <Line Id="P-0001-0_L-2" Name="L" Address="1" MediumTypeRefId="MT-0" CompletionStatus="Accepted">
              <DeviceInstance Id="P-0001-0_DI-1" Name="D" ProductRefId="M-0001_H-1_P-1"
                              Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1"
                              LastModified="2023-07-14T11:55:33" CompletionStatus="FinishedDesign"
                              IndividualAddressLoaded="1" ApplicationProgramLoaded="1"
                              ParametersLoaded="1" CommunicationPartLoaded="1"
                              MediumConfigLoaded="1" IsCommunicationObjectVisibilityCalculated="1"
                              Broken="0">
                <ComObjectInstanceRefs>
                  <ComObjectInstanceRef RefId="M-0001_A-1_O-0_R-1" DatapointType="" IsActive="1">
                    <Connectors><Send GroupAddressRefId="P-0001-0_GA-1" /></Connectors>
                  </ComObjectInstanceRef>
                </ComObjectInstanceRefs>
              </DeviceInstance>
            </Line>
          </Area>
        </Topology>
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" Name="Licht" RangeStart="1" RangeEnd="255">
              <GroupRange Id="P-0001-0_GR-2" Name="An/Aus" RangeStart="1" RangeEnd="127">
                <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />
              </GroupRange>
            </GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

const PROJECT_INFO: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" CompletionStatus="Undefined" />
  </Project>
</KNX>"#;

const GROUP_ADDRESS: &str = r#"<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />"#;

/// A `.knxproj`-shaped archive: `installation_xml` as `P-0001/0.xml`, a
/// valid `Project.xml`, and the signature entry the container reader
/// needs to find the project part.
fn knxproj_bytes(installation_xml: &str) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let entries: [(&str, &[u8]); 3] = [
        ("P-0001.signature", b"x"),
        ("P-0001/0.xml", installation_xml.as_bytes()),
        ("P-0001/Project.xml", PROJECT_INFO),
    ];
    for (name, bytes) in entries {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn clean_knxproj() -> Vec<u8> {
    knxproj_bytes(MINIMAL_INSTALLATION)
}

/// An attribute no schema knows: imported, reported as an unknown
/// construct (a warning), never an error.
fn knxproj_with_unknown_attribute() -> Vec<u8> {
    knxproj_bytes(&MINIMAL_INSTALLATION.replace(
        GROUP_ADDRESS,
        r#"<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" FancyNewAttr="1" />"#,
    ))
}

/// Two group addresses sharing one ETS id: an error-level `DuplicateId`.
fn knxproj_with_duplicate_id() -> Vec<u8> {
    knxproj_bytes(&MINIMAL_INSTALLATION.replace(
        GROUP_ADDRESS,
        r#"<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />
           <GroupAddress Id="P-0001-0_GA-1" Address="2" Name="GA2" />"#,
    ))
}

async fn diff_with_kind(
    app: &axum::Router,
    path: &std::path::Path,
    input_kind: &str,
) -> axum::response::Response {
    call(
        app,
        "POST",
        "/api/project/diff",
        Some(json!({ "path": path.to_string_lossy(), "inputKind": input_kind })),
    )
    .await
}

fn severities(body: &Value) -> Vec<String> {
    body["importDiagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["severity"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn a_knxdb_comparison_names_its_input_kind_and_carries_no_import_report() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("compare.knxdb");
    write_knxdb_fixture(&db_path, &project_with_device_description("Same"));
    let app = knx_server::app(
        Arc::new(state_with_project(project_with_device_description("Same"))),
        None,
    );

    let response = diff_project(&app, &db_path).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;

    assert_eq!(body["inputKind"], "knxdb", "{body}");
    assert!(body["importReport"].is_null(), "{body}");
    assert_eq!(body["importDiagnostics"], json!([]), "{body}");
}

#[tokio::test]
async fn a_clean_knxproj_is_compared_and_its_import_report_travels_with_the_diff() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("clean.knxproj");
    std::fs::write(&path, clean_knxproj()).unwrap();
    let app = knx_server::app(
        Arc::new(state_with_project(project_with_device_description("Same"))),
        None,
    );

    let response = diff_with_kind(&app, &path, "knxproj").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;

    assert_eq!(body["inputKind"], "knxproj", "{body}");
    assert_eq!(body["importReport"]["errors"], json!([]), "{body}");
    assert_eq!(
        body["importReport"]["source"]["file_name"], "clean.knxproj",
        "the full report, not a summary: {body}"
    );
    assert!(
        severities(&body).iter().all(|severity| severity == "info"),
        "a clean import has no warning or error: {body}"
    );
    // The live project has one unassigned device the archive lacks, and
    // the archive has an area the live project lacks: a real comparison ran.
    let installation = &body["installations"][0];
    assert_eq!(
        installation["devices"]["removed"].as_array().unwrap().len(),
        1,
        "{body}"
    );
    assert_eq!(
        installation["areas"]["added"].as_array().unwrap().len(),
        1,
        "{body}"
    );
}

#[tokio::test]
async fn a_knxproj_kind_is_detected_from_the_extension_when_not_named() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Detected.KNXPROJ");
    std::fs::write(&path, clean_knxproj()).unwrap();
    let app = knx_server::app(
        Arc::new(state_with_project(project_with_device_description("Same"))),
        None,
    );

    let response = diff_project(&app, &path).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["inputKind"], "knxproj");
}

#[tokio::test]
async fn a_knxproj_with_warnings_is_compared_and_every_warning_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("warnings.knxproj");
    std::fs::write(&path, knxproj_with_unknown_attribute()).unwrap();
    let app = knx_server::app(
        Arc::new(state_with_project(project_with_device_description("Same"))),
        None,
    );

    let response = diff_with_kind(&app, &path, "knxproj").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;

    assert_eq!(
        body["importReport"]["unknown"].as_array().unwrap().len(),
        1,
        "{body}"
    );
    let severities = severities(&body);
    assert!(severities.iter().any(|s| s == "warning"), "{body}");
    assert!(!severities.iter().any(|s| s == "error"), "{body}");
    assert!(
        body.to_string().contains("FancyNewAttr"),
        "the warning names what it found: {body}"
    );
    assert!(body["installations"].is_array(), "{body}");
}

#[tokio::test]
async fn a_knxproj_with_an_error_diagnostic_is_refused_with_its_report() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.knxproj");
    std::fs::write(&path, knxproj_with_duplicate_id()).unwrap();
    let app = knx_server::app(
        Arc::new(state_with_project(project_with_device_description("Same"))),
        None,
    );

    let response = diff_with_kind(&app, &path, "knxproj").await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(response).await;

    // Two since ADR-0073: the repeated `@Id` (validate) and the device's
    // link to it, which is no longer attached to one of the two by guesswork
    // (map, `AmbiguousReference`).
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("2 error diagnostic"),
        "{body}"
    );
    assert_eq!(body["inputKind"], "knxproj", "{body}");
    assert!(severities(&body).iter().any(|s| s == "error"), "{body}");
    assert!(body.to_string().contains("DuplicateId"), "{body}");
    assert!(
        body.get("installations").is_none(),
        "a refused import is not a comparison: {body}"
    );
}

#[tokio::test]
async fn an_unknown_input_kind_is_a_400_naming_the_accepted_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("clean.knxproj");
    std::fs::write(&path, clean_knxproj()).unwrap();
    let app = knx_server::app(
        Arc::new(state_with_project(project_with_device_description("Same"))),
        None,
    );

    let response = diff_with_kind(&app, &path, "ets6").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error = body_json(response).await["error"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        error.contains("knxdb") && error.contains("knxproj"),
        "{error}"
    );
}

#[tokio::test]
async fn an_input_kind_that_contradicts_the_extension_is_a_400() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("compare.knxdb");
    write_knxdb_fixture(&db_path, &project_with_device_description("Same"));
    let app = knx_server::app(
        Arc::new(state_with_project(project_with_device_description("Same"))),
        None,
    );

    let response = diff_with_kind(&app, &db_path, "knxproj").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_unsupported_extension_is_a_400() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.txt");
    std::fs::write(&path, "not a project").unwrap();
    let app = knx_server::app(
        Arc::new(state_with_project(project_with_device_description("Same"))),
        None,
    );

    let response = diff_project(&app, &path).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_uploaded_knxproj_is_compared_by_its_mount_relative_path() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("uploads")).unwrap();
    std::fs::write(dir.path().join("uploads/up.knxproj"), clean_knxproj()).unwrap();
    let mut state = state_with_project(project_with_device_description("Same"));
    state.data_dir = dir.path().to_path_buf();
    let app = knx_server::app(Arc::new(state), None);

    let response = call(
        &app,
        "POST",
        "/api/project/diff",
        Some(json!({ "path": "uploads/up.knxproj", "inputKind": "knxproj" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let escape = call(
        &app,
        "POST",
        "/api/project/diff",
        Some(json!({ "path": "../outside.knxproj" })),
    )
    .await;
    assert_eq!(escape.status(), StatusCode::BAD_REQUEST);
}
