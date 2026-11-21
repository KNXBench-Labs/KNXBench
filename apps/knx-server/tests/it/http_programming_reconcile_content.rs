//! WP-48.1 over HTTP: a CLI report reconciles only against the plan's content, not its counts.
//!
//! Fixture: the synthetic link program (one parameter, three objects, address and association
//! tables) with one send link. After a run the project changes without changing any count
//! (same-width parameter edit, link replaced by another link); the old report must refuse and
//! leave marks, baseline, history and snapshot untouched. The unchanged report still reconciles.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use knx_core::commissioning::parameter_baseline::{
    BaselineChange, BaselineEdit, LinkBaselineChange, ParameterBaseline,
};
use knx_core::{
    ComObjectInstance, CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Direction,
    GroupAddress, GroupAddressEntry, GroupLink, Installation, InstallationId, Language, Override,
    Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const TARGET: &str = "1.1.7";
const PROGRAM_ID: &str = "M-0001_A-0002-01-0000";
const PARAMETER_REF: &str = "M-0001_A-0002-01-0000_P-1_R-1";

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    let device = project.ids.next_device_id().unwrap();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![device],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    let loaded = CommissioningState {
        individual_address_loaded: true,
        application_program_loaded: true,
        parameters_loaded: true,
        communication_part_loaded: true,
        ..CommissioningState::default()
    };
    project.devices.insert(DeviceInstance {
        id: device,
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "D".into(),
        },
        name: "D".into(),
        description: None,
        address: Some(TARGET.parse().unwrap()),
        product_ref: "P".into(),
        program_ref: "HP".into(),
        commissioning: loaded,
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let mut first = None;
    for raw in [0x0801, 0x0802, 0x0803] {
        let id = project.ids.next_group_address_id().unwrap();
        first.get_or_insert(id);
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id,
                source: SourceRef {
                    path: "synthetic".into(),
                    ets_id: format!("GA-{}", id.0),
                },
                name: format!("GA {raw:04X}"),
                address: GroupAddress::from_raw(raw),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Default::default(),
                description: None,
            });
    }
    for number in 0..3u16 {
        let id = project.ids.next_com_object_instance_id().unwrap();
        project.devices.insert_com_object(ComObjectInstance {
            priority_edit: None,
            saved_instance_priority: Default::default(),
            id,
            source: SourceRef {
                path: "synthetic".into(),
                ets_id: format!("M-0001_A-0002-01-0000_O-{number}_R-{}", number + 1),
            },
            device,
            number,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: knx_core::ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: if number == 0 {
                vec![GroupLink {
                    ga: first.unwrap(),
                    direction: Direction::Send,
                }]
            } else {
                vec![]
            },
            module_instance: None,
        });
        project
            .devices
            .get_mut(device)
            .unwrap()
            .com_objects
            .push(id);
    }
    project.ids = knx_core::project::IdAllocators::from_counts(1, 3, 3, 0, 0, 0, 0, 0, 0);
    project
}

fn state() -> Arc<knx_server::AppState> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.sqlite");
    let products = knx_productdb::open_and_migrate(&path).unwrap();
    knx_productdb::ingest_file(
        &products,
        "program.xml",
        knx_testsupport::LINK_ADOPTION_PROGRAM_XML.as_bytes(),
    )
    .unwrap();
    knx_productdb::ingest_file(
        &products,
        "Hardware.xml",
        knx_testsupport::LINK_ADOPTION_HARDWARE_XML.as_bytes(),
    )
    .unwrap();
    let owner = knx_server::CatalogueOwner::adopt_opened(products, path).unwrap();
    std::mem::forget(dir);
    let project = project();
    let mut state = knx_server::AppState {
        product_db: Some(owner),
        ..Default::default()
    };
    *state.clean_project.lock().unwrap() = Some(project.clone());
    state.project = Mutex::new(Some(project));
    Arc::new(state)
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

/// `(digest, steps, data_octets, group_links)` of the plan the current project yields.
fn current_plan(state: &Arc<knx_server::AppState>, scope: &str) -> (String, usize, usize, usize) {
    use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
    let project = state.project.lock().unwrap().clone().unwrap();
    let target = TARGET.parse().unwrap();
    let parts = match scope {
        "complete" => None,
        "partial-parameters" => Some(PartialDownloadParts {
            parameters: true,
            group_addresses: false,
        }),
        "partial-group-addresses" => Some(PartialDownloadParts {
            parameters: false,
            group_addresses: true,
        }),
        other => panic!("scope {other}"),
    };
    state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|conn| {
            let mut prepared = knx_app::device_download::prepare_device_download_for_reading(
                conn, &project, target,
            )
            .unwrap();
            if let Some(parts) = parts {
                prepared = prepared.into_partial(parts).unwrap();
            }
            Ok::<_, String>((
                knx_app::download_plan_digest::plan_content_digest(&prepared),
                prepared.plan.steps.len(),
                prepared.plan.data_octets(),
                prepared.request.links.len(),
            ))
        })
        .unwrap()
        .unwrap()
}

fn catalogue_hash(state: &Arc<knx_server::AppState>) -> String {
    state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|conn| {
            Ok::<_, String>(
                knx_productdb::query::program_identity(conn, PROGRAM_ID)
                    .unwrap()
                    .unwrap()
                    .source_sha256,
            )
        })
        .unwrap()
        .unwrap()
}

fn check_matches_plan() -> Value {
    json!({
        "observation": {
            "status": "attempted",
            "reach": {
                "result": "reached",
                "identity": {
                    "result": "match",
                    "exemplar": "notVerified",
                    "loadStates": [{
                        "machine": "applicationProgram",
                        "expected": "loaded",
                        "observed": { "result": "observed", "loadState": "loaded" },
                    }],
                    "planLoadVerdict": "matchesPlan",
                },
            },
        },
        "runtime": "unknown",
    })
}

/// The report a CLI run of the project's current plan writes; `digest` false makes it legacy.
fn report(state: &Arc<knx_server::AppState>, scope: &str, digest: bool) -> Value {
    let (content, steps, data_octets, group_links) = current_plan(state, scope);
    let mut plan = json!({ "steps": steps, "dataOctets": data_octets, "groupLinks": group_links });
    if digest {
        plan["contentSha256"] = json!(content);
    }
    json!({
        "kind": "knxbench-download-report",
        "format": 1,
        "knxbenchVersion": "0.1.0-alpha.22+test",
        "program": {
            "id": PROGRAM_ID,
            "name": "Links",
            "applicationVersion": "1",
            "sourceSha256": catalogue_hash(state),
            "manufacturer": "0001",
            "mask": "0701",
        },
        "scope": scope,
        "support": "untested",
        "inferences": [],
        "warnings": [],
        "plan": plan,
        "outcome": {
            "completed": true,
            "written": "yes",
            "restart": "notInPlan",
            "stoppedInStep": null,
        },
        "postDownloadCheck": check_matches_plan(),
        "parameterValues": {},
    })
}

async fn reconcile(
    state: &Arc<knx_server::AppState>,
    report: &Value,
    shown: &Value,
) -> (StatusCode, Value) {
    call(
        state,
        "POST",
        "/api/device/1/programming/reconcile",
        Some(json!({
            "report": report,
            "target": TARGET,
            "serverIncarnation": shown["server_incarnation"],
            "projectIncarnation": shown["project_incarnation"],
            "snapshotRevision": shown["snapshot_revision"],
        })),
    )
    .await
}

fn seed_baseline(state: &Arc<knx_server::AppState>) {
    state
        .parameter_baselines
        .lock()
        .unwrap()
        .apply(&BaselineEdit {
            device: DeviceId(1),
            parameter: BaselineChange::Record(ParameterBaseline {
                program_id: PROGRAM_ID.into(),
                values: Default::default(),
                scoped_values: Default::default(),
                object_flags: None,
                at: Utc::now(),
            }),
            links: LinkBaselineChange::Keep,
        });
}

/// Everything a refused reconcile must leave alone.
struct Witness {
    tree: Value,
    baselines: knx_core::commissioning::parameter_baseline::ParameterBaselines,
    history: (usize, usize),
}

async fn witness(state: &Arc<knx_server::AppState>) -> Witness {
    Witness {
        tree: tree(state).await,
        baselines: state.parameter_baselines.lock().unwrap().clone(),
        history: state.command_stack.lock().unwrap().history_lengths(),
    }
}

async fn assert_refused_unchanged(state: &Arc<knx_server::AppState>, report: &Value, needle: &str) {
    seed_baseline(state);
    let before = witness(state).await;
    let (status, refused) = reconcile(state, report, &before.tree).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    let message = refused["error"].as_str().unwrap_or_default();
    assert!(message.contains(needle), "{refused}");
    let after = witness(state).await;
    assert_eq!(pending(&after.tree), pending(&before.tree), "pending marks");
    assert_eq!(after.tree, before.tree, "the project snapshot");
    assert_eq!(after.baselines, before.baselines, "parameter baselines");
    assert!(after.baselines.get(DeviceId(1)).is_some(), "baseline kept");
    assert_eq!(after.history, before.history, "undo/redo history");
}

#[tokio::test]
async fn an_unchanged_report_still_reconciles_pending_parts() {
    let state = state();
    // A newer edit left parameters pending; the run then wrote exactly this project state.
    let (status, _) = call(
        &state,
        "POST",
        "/api/device/1/parameters",
        Some(json!({ "etsId": PARAMETER_REF, "raw": "2" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let shown = tree(&state).await;
    assert_eq!(pending(&shown)["parameters"], true);
    let (status, reconciled) =
        reconcile(&state, &report(&state, "partial-parameters", true), &shown).await;
    assert_eq!(status, StatusCode::OK, "{reconciled}");
    assert_eq!(pending(&reconciled)["parameters"], false);
}

#[tokio::test]
async fn an_old_report_is_refused_after_a_same_width_parameter_change() {
    let state = state();
    let old = report(&state, "partial-parameters", true);
    let before = current_plan(&state, "partial-parameters");
    let (status, _) = call(
        &state,
        "POST",
        "/api/device/1/parameters",
        Some(json!({ "etsId": PARAMETER_REF, "raw": "2" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let after = current_plan(&state, "partial-parameters");
    assert_eq!((before.1, before.2, before.3), (after.1, after.2, after.3));
    assert_ne!(before.0, after.0, "the content differs");
    assert_eq!(pending(&tree(&state).await)["parameters"], true);
    assert_refused_unchanged(&state, &old, "plan content does not match").await;
}

#[tokio::test]
async fn an_old_report_is_refused_after_a_same_count_link_replacement() {
    let state = state();
    let old = report(&state, "partial-group-addresses", true);
    let before = current_plan(&state, "partial-group-addresses");
    let (object, first, second) = {
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let object = project.devices.get(DeviceId(1)).unwrap().com_objects[0];
        let gas = &project.installations[0].group_addresses;
        (object.0, gas[0].id.0, gas[1].id.0)
    };
    let (status, _) = call(
        &state,
        "DELETE",
        "/api/group-links",
        Some(json!({ "comObjectId": object, "gaId": first, "direction": "Send" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &state,
        "POST",
        "/api/group-links",
        Some(json!({ "comObjectId": object, "gaId": second, "direction": "Send" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let after = current_plan(&state, "partial-group-addresses");
    assert_eq!((before.1, before.2, before.3), (after.1, after.2, after.3));
    assert_ne!(before.0, after.0, "the content differs");
    assert_eq!(pending(&tree(&state).await)["group_addresses"], true);
    assert_refused_unchanged(&state, &old, "same size, different content").await;
}

#[tokio::test]
async fn a_legacy_report_without_the_content_digest_fails_closed() {
    let state = state();
    let (status, _) = call(
        &state,
        "POST",
        "/api/device/1/parameters",
        Some(json!({ "etsId": PARAMETER_REF, "raw": "2" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_refused_unchanged(
        &state,
        &report(&state, "partial-parameters", false),
        "no plan content digest",
    )
    .await;
}
