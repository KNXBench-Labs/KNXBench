//! #50 over HTTP: reconciling an external CLI download report with the
//! server's pending marks, only where the evidence matches the project (ADR-0179).
//!
//! `POST /api/device/{id}/programming/reconcile` accepts a real CLI `--report`
//! JSON (all its fields, parsed by the report type itself) plus the target
//! address the run was started for. A completed run whose post-download check
//! matches the plan, whose program id + source hash match the catalogue, whose
//! plan content digest matches the offline-recomputed plan for that address and whose
//! snapshot binding is current clears exactly the scope's parts; stale,
//! mismatched, unchecked or incomplete reports clear nothing and refuse with
//! 422 (409 when the snapshot is stale). No bus access: the plan is recomputed
//! offline from the open project.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use knx_testsupport::vendor_plugin::{
    program_xml, HARDWARE2PROGRAM_REF, HARDWARE_XML, PRODUCT_REF, PROGRAM_ID, TARGET,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn device(id: u32, programmed: bool) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(id),
        source: SourceRef {
            path: format!("device-{id}"),
            ets_id: format!("device-{id}"),
        },
        name: format!("Device {id}"),
        description: None,
        address: Some(TARGET.parse().unwrap()),
        product_ref: PRODUCT_REF.into(),
        program_ref: HARDWARE2PROGRAM_REF.into(),
        commissioning: CommissioningState {
            individual_address_loaded: programmed,
            application_program_loaded: programmed,
            parameters_loaded: programmed,
            communication_part_loaded: programmed,
            ..CommissioningState::default()
        },
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    }
}

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(device(1, false));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Home".into(),
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
    project
}

/// A state with a real ingested catalogue, so the plan can be recomputed.
/// Returns the state and the catalogue path (kept alive by the caller's
/// tempdir, leaked for the test process).
fn state_with_catalogue() -> (Arc<knx_server::AppState>, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let products = knx_productdb::open_and_migrate(&path).unwrap();
    let a = knx_productdb::ingest_file(&products, "M-FFF6/A.xml", program_xml("").as_bytes())
        .expect("program file ingests");
    assert!(
        matches!(a, knx_productdb::IngestOutcome::Ingested { .. }),
        "the program file was not ingested: {a:?}"
    );
    knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    let adopted = knx_server::CatalogueOwner::adopt_opened(products, path.clone()).unwrap();
    let mut state = knx_server::AppState {
        product_db: Some(adopted),
        ..Default::default()
    };
    let project = project();
    *state.clean_project.lock().unwrap() = Some(project.clone());
    state.project = Mutex::new(Some(project));
    let state = Arc::new(state);
    (state, path)
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
    let text = String::from_utf8_lossy(&bytes).to_string();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

async fn tree(state: &Arc<knx_server::AppState>) -> Value {
    let (status, tree) = call(state, "GET", "/api/project", None).await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    tree
}

fn pending(tree: &Value) -> Value {
    tree["installations"][0]["unassigned"][0]["pending_programming"].clone()
}

fn all(value: bool) -> Value {
    json!({
        "address": value,
        "application": value,
        "parameters": value,
        "group_addresses": value,
    })
}

fn binding(tree: &Value) -> Value {
    json!({
        "serverIncarnation": tree["server_incarnation"],
        "projectIncarnation": tree["project_incarnation"],
        "snapshotRevision": tree["snapshot_revision"],
    })
}

/// The plan shape the current project yields for `scope`, computed offline
/// exactly as the CLI computes it — the matching report's own numbers.
/// Reads the catalogue through the state's owner lease (the file is the
/// same database the server adopted; a second connection would race it).
fn expected_plan(state: &Arc<knx_server::AppState>, scope: &str) -> (usize, usize, usize, String) {
    let project = state.project.lock().unwrap().clone().unwrap();
    let device = project
        .devices
        .iter()
        .find(|d| d.id == DeviceId(1))
        .expect("device 1 in the project");
    let target = device.address.expect("device 1 has an individual address");
    let owner = state.product_db.as_ref().expect("catalogue adopted");
    owner
        .with_read(|conn| {
            let mut prepared = knx_app::device_download::prepare_device_download_for_reading(
                conn, &project, target,
            )
            .unwrap();
            match scope {
                "complete" => {}
                "partial-parameters" => {
                    prepared = prepared
                        .into_partial(knx_core::commissioning::partial_memory_download::PartialDownloadParts {
                            parameters: true,
                            group_addresses: false,
                        })
                        .unwrap();
                }
                "partial-group-addresses" => {
                    prepared = prepared
                        .into_partial(knx_core::commissioning::partial_memory_download::PartialDownloadParts {
                            parameters: false,
                            group_addresses: true,
                        })
                        .unwrap();
                }
                "partial-both" => {
                    prepared = prepared
                        .into_partial(knx_core::commissioning::partial_memory_download::PartialDownloadParts {
                            parameters: true,
                            group_addresses: true,
                        })
                        .unwrap();
                }
                other => panic!("scope {other}"),
            }
            Ok::<_, String>((
                prepared.plan.steps.len(),
                prepared.plan.data_octets(),
                prepared.request.links.len(),
                knx_app::download_plan_digest::plan_content_digest(&prepared),
            ))
        })
        .expect("catalogue read lease")
        .expect("plan preparation")
}

/// The catalogue's hash for PROGRAM_ID, as the CLI report would carry it.
/// Read through the state's owner lease, like the server does.
fn catalogue_hash(state: &Arc<knx_server::AppState>) -> String {
    let owner = state.product_db.as_ref().expect("catalogue adopted");
    owner
        .with_read(|conn| {
            Ok::<_, String>(
                knx_productdb::query::program_identity(conn, PROGRAM_ID)
                    .expect("program identity query"),
            )
        })
        .expect("catalogue read lease")
        .unwrap_or_else(|_| panic!("no application_program row for {PROGRAM_ID}"))
        .expect("program identity present")
        .source_sha256
}

/// A post-download check as the CLI's read-only verification writes it:
/// reached, identity matched, every load state as the plan leaves it.
fn check_matches_plan() -> Value {
    json!({
        "observation": {
            "status": "attempted",
            "reach": {
                "result": "reached",
                "identity": {
                    "result": "match",
                    "exemplar": "notVerified",
                    "loadStates": [
                        {
                            "machine": "applicationProgram",
                            "expected": "loaded",
                            "observed": { "result": "observed", "loadState": "loaded" },
                        },
                    ],
                    "planLoadVerdict": "matchesPlan",
                },
            },
        },
        "runtime": "unknown",
    })
}

/// A complete CLI `--report` file's JSON, exactly the fields
/// `knx_app::download_report::DownloadReport` serialises: kind, format,
/// version, program, scope, support, inferences, warnings, plan, outcome,
/// check and parameter values.
fn report(
    state: &Arc<knx_server::AppState>,
    scope: &str,
    completed: bool,
    hash: Option<&str>,
    check: Option<Value>,
) -> Value {
    let (steps, data_octets, group_links, digest) = expected_plan(state, scope);
    json!({
        "kind": "knxbench-download-report",
        "format": 1,
        "knxbenchVersion": "0.1.0-alpha.22+test",
        "program": {
            "id": PROGRAM_ID,
            "name": "Synthetic plug-in witness",
            "applicationVersion": "1",
            "sourceSha256": hash.unwrap_or("0000000000000000000000000000000000000000000000000000000000000000"),
            "manufacturer": "FFF6",
            "mask": "0701",
        },
        "scope": scope,
        "support": "untested",
        "inferences": [],
        "warnings": [],
        "plan": {
            "steps": steps,
            "dataOctets": data_octets,
            "groupLinks": group_links,
            "contentSha256": digest,
        },
        "outcome": {
            "completed": completed,
            "written": if completed { "yes" } else { "partially" },
            "restart": if completed { json!("acknowledged") } else { Value::Null },
            "stoppedInStep": if completed { Value::Null } else { json!(3) },
        },
        "postDownloadCheck": check,
        "parameterValues": {},
    })
}

/// The reconcile request body: the report plus the run's target address and
/// the snapshot binding.
fn request(report: Value, target: &str, shown: &Value) -> Value {
    let mut body = json!({ "report": report, "target": target });
    body.as_object_mut()
        .unwrap()
        .extend(binding(shown).as_object().unwrap().clone());
    body
}

async fn reconcile(state: &Arc<knx_server::AppState>, body: Value) -> (StatusCode, Value) {
    call(
        state,
        "POST",
        "/api/device/1/programming/reconcile",
        Some(body),
    )
    .await
}

#[tokio::test]
async fn reconcile_clears_only_the_scoped_parts_of_a_matching_completed_run() {
    let (state, _catalogue) = state_with_catalogue();
    let shown = tree(&state).await;
    assert_eq!(pending(&shown), all(true), "device starts fully pending");

    let body = request(
        report(
            &state,
            "partial-group-addresses",
            true,
            Some(&catalogue_hash(&state)),
            Some(check_matches_plan()),
        ),
        TARGET,
        &shown,
    );
    let (status, reconciled) = reconcile(&state, body).await;
    assert_eq!(status, StatusCode::OK, "{reconciled}");
    assert_eq!(
        pending(&reconciled),
        json!({
            "address": true,
            "application": true,
            "parameters": true,
            "group_addresses": false,
        }),
        "only the run's scope is cleared"
    );

    // The reconcile is undoable like a manual mark.
    let (status, undone) = call(&state, "POST", "/api/undo", None).await;
    assert_eq!(status, StatusCode::OK, "{undone}");
    assert_eq!(pending(&undone), all(true));
}

#[tokio::test]
async fn a_serialized_cli_report_file_parses_and_reconciles() {
    // The wire format is a read contract now: a report written by
    // DownloadReport::to_json (what --report writes) must parse back and be
    // accepted field for field, extra diagnostic fields included.
    let (state, _catalogue) = state_with_catalogue();
    let shown = tree(&state).await;
    let wire = report(
        &state,
        "partial-parameters",
        true,
        Some(&catalogue_hash(&state)),
        Some(check_matches_plan()),
    );
    let parsed: knx_app::download_report::DownloadReport =
        serde_json::from_value(wire).expect("the CLI report file parses");
    assert_eq!(parsed.kind, "knxbench-download-report");
    assert_eq!(parsed.scope, "partial-parameters");
    // Round-trip through the type changes no field the server reads.
    let round_tripped: Value = serde_json::to_value(&parsed).expect("the report serialises again");
    let body = request(round_tripped, TARGET, &shown);
    let (status, reconciled) = reconcile(&state, body).await;
    assert_eq!(status, StatusCode::OK, "{reconciled}");
    assert_eq!(
        pending(&reconciled),
        json!({
            "address": true,
            "application": true,
            "parameters": false,
            "group_addresses": true,
        }),
        "a partial-parameters run clears exactly parameters"
    );
}

#[tokio::test]
async fn reconcile_refuses_incomplete_mismatched_or_stale_reports_without_changes() {
    let (state, _catalogue) = state_with_catalogue();
    let shown = tree(&state).await;
    let hash = catalogue_hash(&state);

    // Not completed.
    let (status, refused) = reconcile(
        &state,
        request(
            report(&state, "partial-group-addresses", false, Some(&hash), None),
            TARGET,
            &shown,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(pending(&tree(&state).await), all(true));

    // Wrong program hash (device/program mismatch).
    let (status, refused) = reconcile(
        &state,
        request(
            report(
                &state,
                "partial-group-addresses",
                true,
                Some("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
                Some(check_matches_plan()),
            ),
            TARGET,
            &shown,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(pending(&tree(&state).await), all(true));

    // Completed but no post-download check: an unchecked run is not proof.
    let (status, refused) = reconcile(
        &state,
        request(
            report(&state, "partial-group-addresses", true, Some(&hash), None),
            TARGET,
            &shown,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("no post-download check"),
        "{refused}"
    );
    assert_eq!(pending(&tree(&state).await), all(true));

    // The check ran but the load states differ from the plan.
    let mut doubting = check_matches_plan();
    doubting["observation"]["reach"]["identity"]["planLoadVerdict"] = json!("differsFromPlan");
    let (status, refused) = reconcile(
        &state,
        request(
            report(
                &state,
                "partial-group-addresses",
                true,
                Some(&hash),
                Some(doubting),
            ),
            TARGET,
            &shown,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(pending(&tree(&state).await), all(true));

    // The run's target does not resolve to this device.
    let (status, refused) = reconcile(
        &state,
        request(
            report(
                &state,
                "partial-group-addresses",
                true,
                Some(&hash),
                Some(check_matches_plan()),
            ),
            "1.1.99",
            &shown,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("does not resolve"),
        "{refused}"
    );
    assert_eq!(pending(&tree(&state).await), all(true));

    // Stale snapshot: an edit bumped the revision after the client saw it.
    let (status, _) = call(
        &state,
        "POST",
        "/api/individual-address",
        Some(json!({ "deviceId": 1, "address": "1.1.8" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, conflict) = reconcile(
        &state,
        request(
            report(
                &state,
                "partial-group-addresses",
                true,
                Some(&hash),
                Some(check_matches_plan()),
            ),
            TARGET,
            &shown,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{conflict}");
    assert_eq!(conflict["kind"], "programmingStale");
}

#[tokio::test]
async fn reconcile_refuses_a_plan_that_no_longer_matches_the_project() {
    let (state, _catalogue) = state_with_catalogue();
    let shown = tree(&state).await;
    let hash = catalogue_hash(&state);

    // Report numbers for a different project state: one octet short, and a
    // digest of other content.
    let (steps, data_octets, group_links, _) = expected_plan(&state, "partial-group-addresses");
    let mut wire = report(
        &state,
        "partial-group-addresses",
        true,
        Some(&hash),
        Some(check_matches_plan()),
    );
    wire["plan"]["dataOctets"] = json!(data_octets - 1);
    wire["plan"]["steps"] = json!(steps);
    wire["plan"]["groupLinks"] = json!(group_links);
    wire["plan"]["contentSha256"] = json!("0".repeat(64));
    let (status, refused) = reconcile(&state, request(wire, TARGET, &shown)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("does not match"),
        "{refused}"
    );
    assert_eq!(
        pending(&tree(&state).await),
        all(true),
        "nothing was cleared"
    );
}

#[tokio::test]
async fn an_edit_after_the_run_stays_pending_and_a_stale_run_never_reconciles_again() {
    let (state, _catalogue) = state_with_catalogue();
    let shown = tree(&state).await;
    let hash = catalogue_hash(&state);

    // The external run reconciles: group addresses clear.
    let (status, reconciled) = reconcile(
        &state,
        request(
            report(
                &state,
                "partial-group-addresses",
                true,
                Some(&hash),
                Some(check_matches_plan()),
            ),
            TARGET,
            &shown,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reconciled}");
    assert_eq!(pending(&reconciled)["group_addresses"], false);

    // A later edit in the server marks its own part pending; the reconciled
    // part stays cleared — the reconcile never touches later edits.
    let (status, edited) = call(
        &state,
        "POST",
        "/api/individual-address",
        Some(json!({ "deviceId": 1, "address": "1.1.8" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{edited}");
    assert_eq!(
        pending(&tree(&state).await),
        json!({
            "address": true,
            "application": true,
            "parameters": true,
            "group_addresses": false,
        }),
        "the later edit is pending, the reconciled scope is not re-marked"
    );

    // The old report (target 1.1.7) no longer matches: the target does not
    // resolve to device 1 any more and the plan changed. Refused, nothing cleared.
    let refreshed = tree(&state).await;
    let (status, refused) = reconcile(
        &state,
        request(
            report(
                &state,
                "partial-group-addresses",
                true,
                Some(&hash),
                Some(check_matches_plan()),
            ),
            TARGET,
            &refreshed,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(
        pending(&tree(&state).await),
        json!({
            "address": true,
            "application": true,
            "parameters": true,
            "group_addresses": false,
        }),
        "stale evidence cleared nothing"
    );
}
