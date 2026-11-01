//! `button-literal-passive-v1` over the real in-process HTTP parameter routes.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const HARDWARE: &str =
    include_str!("../../../../../crates/knx-productdb/tests/fixtures/passive-button-hardware.xml");
const EN: &str = include_str!(
    "../../../../../crates/knx-productdb/tests/fixtures/passive-button-literal.en.xml"
);
const DE: &str = include_str!(
    "../../../../../crates/knx-productdb/tests/fixtures/passive-button-literal.de.xml"
);
const ONLY: &str =
    include_str!("../../../../../crates/knx-productdb/tests/fixtures/passive-button-only.xml");
const ORACLE: &str =
    include_str!("../../../../../crates/knx-productdb/tests/fixtures/passive-button-literal.json");

const PARAMETERS: &str = "/api/device/1/parameters";
const OUTER: &str = r#"<ParameterBlock Id="PB-OUTER" Text="Fictional outer page">"#;

fn oracle() -> Value {
    serde_json::from_str(ORACLE).unwrap()
}

#[allow(clippy::field_reassign_with_default)]
fn state(program: &str) -> (tempfile::TempDir, Arc<knx_server::AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-FIX/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-FIX/A-FIX.xml", program.as_bytes()).unwrap();
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "device-1".into(),
            ets_id: "device-1".into(),
        },
        name: "Device 1".into(),
        description: None,
        address: None,
        product_ref: "M-FIX_P-1".into(),
        program_ref: "H-FIX_HP-1".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
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
    project.ids = knx_core::project::IdAllocators::from_counts(1, 0, 0, 0, 0, 0, 0, 0, 0);
    let mut state = knx_server::AppState::default();
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    *state.clean_project.lock().unwrap() = Some(project.clone());
    *state.project.lock().unwrap() = Some(project);
    (dir, Arc::new(state))
}

async fn call(
    state: &Arc<knx_server::AppState>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(body) => {
            request = request.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let response = knx_server::app(state.clone(), None)
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn panel_at(state: &Arc<knx_server::AppState>, uri: &str) -> Value {
    let (status, dto) = call(state, "GET", uri, None).await;
    assert_eq!(status, StatusCode::OK, "{dto}");
    dto
}

async fn panel(program: &str) -> Value {
    let (_dir, state) = state(program);
    panel_at(&state, PARAMETERS).await
}

fn device_section(dto: &Value) -> &Value {
    let sections = dto["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 1, "{dto}");
    assert!(sections[0]["scope"].is_null());
    &sections[0]
}

fn passive_ids(dto: &Value) -> Vec<String> {
    dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|section| {
            section["passiveLabels"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .map(|label| label["elementId"].as_str().unwrap().to_owned())
        .collect()
}

fn layout<'a>(section: &'a Value, key: &str) -> Option<&'a Value> {
    section["groupLayouts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|layout| layout["groupKey"] == key)
}

/// `(etsId, writeEtsId, value, group key, cell)`: every field's identity.
fn identities(section: &Value) -> Vec<Value> {
    section["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| {
            json!({ "etsId": field["etsId"], "writeEtsId": field["writeEtsId"], "value": field["value"],
                "groupKey": field["group"]["key"], "cell": field["cell"] })
        })
        .collect()
}

#[tokio::test]
async fn a_mixed_program_returns_literal_passive_labels_and_unchanged_fields() {
    let dto = panel(EN).await;
    let section = device_section(&dto);
    assert_eq!(section["passiveLabels"], oracle()["passiveLabels"]["en"]);
    assert_eq!(
        identities(section),
        *oracle()["unchangedFields"].as_array().unwrap()
    );
    let only = layout(section, "PB-ONLY").expect("the passive-only group registers its grid");
    assert_eq!(only["status"], "grid");
    assert_eq!(only["columns"][0]["width"], "46%");
    assert_eq!(only["columns"][1]["width"], "53%");
    assert_eq!(
        only["labels"],
        json!([]),
        "passive text is no separator caption"
    );
    // The functional Button stays reported as unsupported content; the server
    // keeps this one issue kind and the client words it as "label shown; its
    // action is not supported" wherever it actually shows the label.
    assert_eq!(
        only["issues"],
        json!([{ "kind": "unshownContent", "element": "Button", "reference": "B-ONLY", "cell": "1,2" }])
    );
    let mixed = layout(section, "PB-MIX").unwrap();
    assert_eq!(
        mixed["labels"],
        json!([{ "row": 1, "column": 2, "text": "Fictional separator" }])
    );
    let separators: Vec<&Value> = section["separators"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| &s["elementId"])
        .collect();
    assert_eq!(separators, [&json!("S-MIX")]);
    // No handler data and no fake parameter anywhere in the response.
    let text = dto.to_string();
    for inert in ["fictionalCallback", "ConnectionOriented", "fixture\\\""] {
        assert!(!text.contains(inert), "{inert} leaked");
    }
    assert!(identities(section)
        .iter()
        .all(|field| !field["etsId"].as_str().unwrap().starts_with("B-")));
}

#[tokio::test]
async fn a_sole_button_program_keeps_a_passive_only_section_with_its_grid() {
    let dto = panel(ONLY).await;
    let section = device_section(&dto);
    assert_eq!(section["fields"], json!([]));
    assert_eq!(section["separators"], json!([]));
    assert_eq!(
        section["passiveLabels"],
        json!([oracle()["passiveLabels"]["en"][0]])
    );
    assert_eq!(layout(section, "PB-ONLY").unwrap()["status"], "grid");
}

#[tokio::test]
async fn alternate_sources_keep_their_literal_and_a_product_language_translates_nothing() {
    let dto = panel(DE).await;
    assert_eq!(
        device_section(&dto)["passiveLabels"],
        oracle()["passiveLabels"]["de"]
    );
    let (_dir, state) = state(EN);
    for language in ["de", "de-DE", "en-US"] {
        let dto = panel_at(&state, &format!("{PARAMETERS}?language={language}")).await;
        assert_eq!(
            device_section(&dto)["passiveLabels"],
            oracle()["passiveLabels"]["en"],
            "{language}"
        );
    }
}

#[tokio::test]
async fn activation_decides_presence_without_evaluating_every_branch() {
    let closed = panel(&EN.replace(
        r#"Text="Fictional selector" ParameterType="PT-MODE" Value="1""#,
        r#"Text="Fictional selector" ParameterType="PT-MODE" Value="0""#,
    ))
    .await;
    assert_eq!(passive_ids(&closed), ["B-MIX"]);
    assert!(layout(device_section(&closed), "PB-ONLY").is_none());
    let mut expected = oracle()["unchangedFields"].clone();
    expected[0]["value"] = json!("0");
    assert_eq!(
        identities(device_section(&closed)),
        *expected.as_array().unwrap()
    );

    // A controlling value that is no number activates nothing below the choice.
    let non_numeric = panel(&EN.replace(
        r#"Text="Fictional selector" ParameterType="PT-MODE" Value="1""#,
        r#"Text="Fictional selector" ParameterType="PT-MODE" Value="one""#,
    ))
    .await;
    assert_eq!(passive_ids(&non_numeric), ["B-MIX"]);
    assert!(layout(device_section(&non_numeric), "PB-ONLY").is_none());
    assert!(
        non_numeric["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["kind"] == "nonNumericValue"),
        "the existing evaluator diagnostic names the non-numeric value: {non_numeric}"
    );

    let unresolved = panel(&EN.replace(
        r#"<choose ParamRefId="PR-MODE">"#,
        r#"<choose ParamRefId="PR-ABSENT">"#,
    ))
    .await;
    assert_eq!(passive_ids(&unresolved), ["B-MIX"]);
    assert!(
        unresolved["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["detail"].as_str().unwrap().contains("PR-ABSENT")),
        "the existing evaluator diagnostic names the unresolved choice: {unresolved}"
    );
}

#[tokio::test]
async fn ancestor_access_survives_into_the_dto_verbatim() {
    for token in ["Read", "None", "Sometimes"] {
        let dto = panel(&EN.replace(
            OUTER,
            &format!(
                r#"<ParameterBlock Id="PB-OUTER" Text="Fictional outer page" Access="{token}">"#
            ),
        ))
        .await;
        let labels = device_section(&dto)["passiveLabels"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(labels[0]["elementId"], "B-ONLY");
        assert_eq!(labels[0]["blockAccess"], token, "{token}");
        assert_eq!(labels[1]["blockAccess"], Value::Null);
    }
}

/// Everything a display read must leave exactly as it was.
#[derive(Debug, PartialEq)]
struct Observable {
    project: Option<Project>,
    undo_redo: (usize, usize),
    revision: u64,
    sources: Vec<(String, Vec<u8>)>,
}

fn observe(state: &knx_server::AppState) -> Observable {
    let sources = state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|products| -> Result<_, knx_productdb::ProductDbError> {
            let mut stmt =
                products.prepare("SELECT sha256, bytes FROM source_file ORDER BY sha256")?;
            let rows = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .unwrap()
        .unwrap();
    Observable {
        project: state.project.lock().unwrap().clone(),
        undo_redo: state.command_stack.lock().unwrap().history_lengths(),
        revision: state
            .project_revision
            .load(std::sync::atomic::Ordering::SeqCst),
        sources,
    }
}

fn field<'a>(dto: &'a Value, ets_id: &str) -> &'a Value {
    device_section(dto)["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|field| field["etsId"] == ets_id)
        .unwrap_or_else(|| panic!("{ets_id} listed"))
}

#[tokio::test]
async fn display_reads_change_nothing_and_a_button_id_is_no_write_target() {
    let (_dir, state) = state(EN);
    let before = observe(&state);
    let first = panel_at(&state, PARAMETERS).await;
    let _ = panel_at(&state, &format!("{PARAMETERS}?language=de")).await;
    assert_eq!(observe(&state), before, "GETs are pure reads");

    let (_, tree) = call(&state, "GET", "/api/project", None).await;
    for button in ["B-ONLY", "B-MIX"] {
        let unbound = call(
            &state,
            "POST",
            PARAMETERS,
            Some(json!({ "etsId": button, "raw": "1" })),
        )
        .await;
        let bound = call(
            &state,
            "POST",
            PARAMETERS,
            Some(json!({ "etsId": button, "raw": "1",
                "serverIncarnation": tree["server_incarnation"], "projectIncarnation": tree["project_incarnation"],
                "snapshotRevision": tree["snapshot_revision"], "expectedValue": null })),
        )
        .await;
        // The real writer's lookup refusal: a Button Id is no declared parameter.
        let refusal = json!({ "error": format!("'{button}' is not a parameter declared by this program (program 'A-FIX')") });
        for (status, body) in [&unbound, &bound] {
            assert_eq!(*status, StatusCode::BAD_REQUEST, "{button}: {body}");
            assert_eq!(*body, refusal, "{button}");
        }
        assert_eq!(
            observe(&state),
            before,
            "{button}: refused, nothing written"
        );
    }

    // The ordinary editable field in the same grid stays a real write.
    let edit = field(&first, "PR-EDIT");
    assert_eq!(edit["writeEtsId"], "PR-EDIT");
    let (status, written) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(json!({ "etsId": edit["writeEtsId"], "raw": "19",
            "serverIncarnation": tree["server_incarnation"], "projectIncarnation": tree["project_incarnation"],
            "snapshotRevision": tree["snapshot_revision"], "expectedValue": edit["value"] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{written}");
    assert!(written["tree"]["can_undo"].as_bool().unwrap());
    assert_eq!(field(&written, "PR-EDIT")["value"], "19");
    assert_eq!(
        device_section(&written)["passiveLabels"],
        device_section(&first)["passiveLabels"],
        "source text and coordinates are unchanged by a write"
    );
    let after = observe(&state);
    assert_eq!(
        after.undo_redo,
        (before.undo_redo.0 + 1, 0),
        "one undo step"
    );
    assert_eq!(after.sources, before.sources, "no source bytes touched");

    // A stale repeat of the same binding keeps its existing conflict.
    let (stale, _) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(json!({ "etsId": "PR-EDIT", "raw": "20",
            "serverIncarnation": tree["server_incarnation"], "projectIncarnation": tree["project_incarnation"],
            "snapshotRevision": tree["snapshot_revision"], "expectedValue": edit["value"] })),
    )
    .await;
    assert_eq!(stale, StatusCode::CONFLICT);
    assert_eq!(observe(&state).undo_redo, after.undo_redo);
}

const BUDGET_ERROR: &str = "device text projection budget exhausted";

/// The Button-only grid with two literals of the given byte lengths, in both cells.
fn two_labels(first: usize, second: usize) -> String {
    ONLY.replace("Fictional source label", &"x".repeat(first))
        .replace(
            r#"Cell="1,2"/>"#,
            &format!(
                r#"Cell="1,2"/><Button Id="B-SECOND" Text="{}" Cell="1,1"/>"#,
                "y".repeat(second)
            ),
        )
}

#[tokio::test]
async fn passive_labels_share_one_copy_budget_and_exhaustion_refuses_the_whole_panel() {
    // Nothing else in this grid carries text, so the two literals alone meet
    // the shared ceiling exactly: admitted verbatim.
    let half = knx_productdb::dynamic::MAX_TEXT_PROJECTION_WORK / 2;
    let (_dir, admitted) = state(&two_labels(half, half));
    let before = observe(&admitted);
    let dto = panel_at(&admitted, PARAMETERS).await;
    let texts: Vec<String> = device_section(&dto)["passiveLabels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|label| label["text"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(texts, ["x".repeat(half), "y".repeat(half)]);
    assert_eq!(observe(&admitted), before);

    // One byte more refuses the GET outright: no partial panel or label.
    let (_dir, refused) = state(&two_labels(half, half + 1));
    let before = observe(&refused);
    let generation = *refused.history_generation.lock().unwrap();
    let (status, body) = call(&refused, "GET", PARAMETERS, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, json!({ "error": BUDGET_ERROR }));
    assert_eq!(observe(&refused), before);
    assert_eq!(*refused.history_generation.lock().unwrap(), generation);
}

#[tokio::test]
async fn an_over_budget_label_refuses_a_write_without_publishing_it() {
    let huge = "x".repeat(knx_productdb::dynamic::MAX_TEXT_PROJECTION_WORK / 2 + 1);
    let xml = EN
        .replace("Fictional source label", &huge)
        .replace("Fictional mixed label", &huge);
    let (_dir, state) = state(&xml);
    let before = observe(&state);
    let generation = *state.history_generation.lock().unwrap();
    let (status, body) = call(&state, "GET", PARAMETERS, None).await;
    assert_eq!(
        (status, body),
        (StatusCode::BAD_REQUEST, json!({ "error": BUDGET_ERROR }))
    );
    let (_, tree) = call(&state, "GET", "/api/project", None).await;
    let (status, body) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(json!({ "etsId": "PR-EDIT", "raw": "19",
            "serverIncarnation": tree["server_incarnation"], "projectIncarnation": tree["project_incarnation"],
            "snapshotRevision": tree["snapshot_revision"], "expectedValue": "17" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body, json!({ "error": BUDGET_ERROR }));
    assert_eq!(
        observe(&state),
        before,
        "project, history, revision and sources unchanged"
    );
    assert_eq!(*state.history_generation.lock().unwrap(), generation);
}
