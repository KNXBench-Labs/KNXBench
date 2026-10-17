//! Note-aware deletes over HTTP: a named refusal first, then one bound choice.
//!
//! Every entity delete route that would strand a project note answers a 409
//! `noteDispositionRequired` naming each note, the targets the delete removes
//! and the snapshot it was computed on. `POST /api/project/delete-with-notes`
//! applies the user's choices and the delete as one undo step, bound to that
//! snapshot (ADR-0138). Refusals change nothing.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::note::{ProjectNote, ProjectNoteTarget as Target};
use knx_core::{
    Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance,
    ComObjectInstanceId, CommissioningState, CompletionStatus, DeviceId, DeviceInstance,
    GroupAddress, GroupAddressEntry, GroupAddressId, GroupRange, GroupRangeId, Installation,
    InstallationId, Language, Line, LineId, Override, Project, ProjectNoteId, ResolvedFlags,
    SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const APPLY: &str = "/api/project/delete-with-notes";

fn source(id: &str) -> SourceRef {
    SourceRef {
        path: id.into(),
        ets_id: id.into(),
    }
}
fn line(id: u32, address: u8, devices: Vec<DeviceId>) -> Line {
    Line {
        id: LineId(id),
        source: source("line"),
        name: format!("line {id}"),
        address,
        medium_ref: "MT-0".into(),
        domain_address: None,
        domain_address_is_checked: None,
        ip_routing_multicast_address: None,
        multicast_ttl: None,
        completion: CompletionStatus::Editing,
        devices,
    }
}
fn room(id: u32) -> BuildingPart {
    BuildingPart {
        id: BuildingPartId(id),
        source: source("room"),
        name: format!("room {id}"),
        number: None,
        kind: BuildingPartType::Room,
        default_line: None,
        completion: CompletionStatus::Editing,
        children: vec![],
        devices: vec![],
        parent: None,
    }
}
fn group(id: u32) -> GroupAddressEntry {
    GroupAddressEntry {
        id: GroupAddressId(id),
        source: source("ga"),
        name: format!("group {id}"),
        address: GroupAddress::from_raw(u16::try_from(id).unwrap()),
        central: false,
        unfiltered: false,
        range: None,
        declared_dpt: Override::Absent,
    }
}
fn range(id: u32) -> GroupRange {
    let start = 0x0800 * u16::try_from(id).unwrap();
    GroupRange {
        id: GroupRangeId(id),
        source: source("range"),
        name: format!("range {id}"),
        start: GroupAddress::from_raw(start),
        end: GroupAddress::from_raw(start + 0x07ff),
        parent: None,
        children: vec![],
    }
}

/// Devices 1 (object 1) and 2 on line 1, empty line 2, empty area 2, rooms
/// 1 and 2, ranges 1 and 2, group addresses 1 and 2; nothing linked.
fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    for (device, object) in [(1, Some(1)), (2, None)] {
        project.devices.insert(DeviceInstance {
            id: DeviceId(device),
            source: source("device"),
            name: format!("device {device}"),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "A".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: object.map(ComObjectInstanceId).into_iter().collect(),
            binary_data: vec![],
        });
    }
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: source("object"),
        device: DeviceId(1),
        number: 0,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
        module_instance: None,
    });
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "install".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::Editing,
        topology: Topology {
            areas: vec![
                Area {
                    id: AreaId(1),
                    source: source("area"),
                    name: "area 1".into(),
                    address: 1,
                    completion: CompletionStatus::Editing,
                    lines: vec![LineId(1), LineId(2)],
                },
                Area {
                    id: AreaId(2),
                    source: source("area"),
                    name: "area 2".into(),
                    address: 2,
                    completion: CompletionStatus::Editing,
                    lines: vec![],
                },
            ],
            lines: vec![
                line(1, 1, vec![DeviceId(1), DeviceId(2)]),
                line(2, 2, vec![]),
            ],
            unassigned: vec![],
        },
        buildings: vec![room(1), room(2)],
        group_ranges: vec![range(1), range(2)],
        group_addresses: vec![group(1), group(2)],
        parameters: vec![],
    });
    project.ids = knx_core::project::IdAllocators::from_counts(2, 2, 2, 1, 2, 2, 2, 0, 0);
    project
}
fn noted(notes: &[(u32, Target, &str)]) -> Project {
    let mut project = project();
    for (id, target, title) in notes {
        project.notes.push(
            ProjectNote::new(ProjectNoteId(*id), *target, (*title).into(), "body".into()).unwrap(),
        );
    }
    let highest = notes.iter().map(|(id, ..)| *id).max().unwrap_or(0);
    project.ids = project.ids.clone().with_project_note_count(highest);
    project
}
fn state(project: Project) -> Arc<knx_server::AppState> {
    let state = knx_server::AppState::default();
    *state.clean_project.lock().unwrap() = Some(project.clone());
    *state.project.lock().unwrap() = Some(project);
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
fn current(state: &knx_server::AppState) -> Project {
    state.project.lock().unwrap().clone().unwrap()
}
fn undo_depth(state: &knx_server::AppState) -> (usize, usize) {
    state.command_stack.lock().unwrap().history_lengths()
}
fn apply_body(refusal: &Value, delete: Value, choices: Value) -> Value {
    json!({
        "serverIncarnation": refusal["serverIncarnation"],
        "projectIncarnation": refusal["projectIncarnation"],
        "snapshotRevision": refusal["snapshotRevision"],
        "delete": delete,
        "choices": choices,
    })
}

/// Each delete route, the request that reaches it, the wire form of the
/// same deletion for the apply route, and the note target it strands.
fn routes() -> Vec<(&'static str, String, Option<Value>, Value, Target)> {
    vec![
        (
            "DELETE",
            "/api/devices/2".into(),
            None,
            json!({"kind":"devices","ids":[2]}),
            Target::Device(DeviceId(2)),
        ),
        (
            "POST",
            "/api/devices/batch-delete".into(),
            Some(json!({"ids":[2]})),
            json!({"kind":"devices","ids":[2]}),
            Target::Device(DeviceId(2)),
        ),
        (
            "DELETE",
            "/api/group-addresses/1".into(),
            None,
            json!({"kind":"groupAddresses","ids":[1]}),
            Target::GroupAddress(GroupAddressId(1)),
        ),
        (
            "POST",
            "/api/group-addresses/batch-delete".into(),
            Some(json!({"ids":[1]})),
            json!({"kind":"groupAddresses","ids":[1]}),
            Target::GroupAddress(GroupAddressId(1)),
        ),
        (
            "DELETE",
            "/api/areas/2".into(),
            None,
            json!({"kind":"area","id":2}),
            Target::Area(AreaId(2)),
        ),
        (
            "DELETE",
            "/api/lines/2".into(),
            None,
            json!({"kind":"line","id":2}),
            Target::Line(LineId(2)),
        ),
        (
            "DELETE",
            "/api/building-parts/2".into(),
            None,
            json!({"kind":"buildingPart","id":2}),
            Target::BuildingPart(BuildingPartId(2)),
        ),
        (
            "DELETE",
            "/api/group-ranges/2".into(),
            None,
            json!({"kind":"groupRange","id":2}),
            Target::GroupRange(GroupRangeId(2)),
        ),
    ]
}

#[tokio::test]
async fn every_delete_route_names_the_stranded_note_and_changes_nothing() {
    for (method, uri, body, _, target) in routes() {
        let state = state(noted(&[(7, target, "Keep me")]));
        let before = current(&state);
        let depth = undo_depth(&state);
        let (status, refusal) = call(&state, method, &uri, body).await;
        assert_eq!(status, StatusCode::CONFLICT, "{uri}: {refusal}");
        assert_eq!(refusal["kind"], "noteDispositionRequired", "{uri}");
        assert_eq!(refusal["notes"].as_array().unwrap().len(), 1, "{uri}");
        assert_eq!(refusal["notes"][0]["id"], 7);
        assert_eq!(refusal["notes"][0]["title"], "Keep me");
        let error = refusal["error"].as_str().unwrap();
        assert!(error.contains("note 7 \"Keep me\""), "{error}");
        assert!(!error.contains("note validation refused"), "{error}");
        assert_eq!(
            refusal["serverIncarnation"].as_str().unwrap(),
            state.server_incarnation
        );
        assert!(refusal["snapshotRevision"].is_u64());
        assert!(refusal["projectIncarnation"].is_u64());
        assert_eq!(current(&state), before, "{uri}");
        assert_eq!(undo_depth(&state), depth, "{uri}");
    }
}

#[tokio::test]
async fn the_refusal_lists_cascaded_notes_and_the_targets_the_delete_removes() {
    let state = state(noted(&[
        (1, Target::Device(DeviceId(1)), "device"),
        (2, Target::Project, "project"),
        (3, Target::ComObject(ComObjectInstanceId(1)), "object"),
    ]));
    let (status, refusal) = call(&state, "DELETE", "/api/devices/1", None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        refusal["notes"],
        json!([
            {"id":1,"title":"device","target":{"kind":"device","id":1}},
            {"id":3,"title":"object","target":{"kind":"comObject","id":1}},
        ])
    );
    assert_eq!(
        refusal["removedTargets"],
        json!([{"kind":"device","id":1},{"kind":"comObject","id":1}])
    );
}

#[tokio::test]
async fn a_delete_without_noted_targets_keeps_its_old_answer() {
    let state = state(noted(&[(1, Target::Device(DeviceId(1)), "other device")]));
    let (status, tree) = call(&state, "DELETE", "/api/devices/2", None).await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    assert!(current(&state).devices.get(DeviceId(2)).is_none());
}

#[tokio::test]
async fn chosen_dispositions_and_the_delete_are_one_undo_step_for_every_route() {
    for (method, uri, body, delete, target) in routes() {
        let state = state(noted(&[
            (7, target, "Keep me"),
            (8, Target::Project, "Untouched"),
        ]));
        let before = current(&state);
        let (_, refusal) = call(&state, method, &uri, body).await;
        let depth = undo_depth(&state);
        let (status, tree) = call(
            &state,
            "POST",
            APPLY,
            Some(apply_body(
                &refusal,
                delete,
                json!([{"note":7,"action":{"kind":"project"}}]),
            )),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{uri}: {tree}");
        assert!(
            tree["snapshot_revision"].is_u64(),
            "a project tree comes back"
        );
        let after = current(&state);
        assert!(target.validate(&after).is_err(), "{uri}: entity deleted");
        assert_eq!(after.notes[0].target(), Target::Project);
        assert_eq!(after.notes[0].title(), "Keep me");
        assert_eq!(undo_depth(&state).0, depth.0 + 1, "{uri}: one undo step");
        let (status, _) = call(&state, "POST", "/api/undo", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(current(&state), before, "{uri}: undo restores everything");
        let (status, _) = call(&state, "POST", "/api/redo", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(current(&state), after, "{uri}: redo repeats it");
    }
}

#[tokio::test]
async fn move_and_delete_choices_reach_the_model() {
    let state = state(noted(&[
        (1, Target::Device(DeviceId(1)), "device"),
        (3, Target::ComObject(ComObjectInstanceId(1)), "object"),
    ]));
    let (_, refusal) = call(&state, "DELETE", "/api/devices/1", None).await;
    let (status, tree) = call(
        &state,
        "POST",
        APPLY,
        Some(apply_body(
            &refusal,
            json!({"kind":"devices","ids":[1]}),
            json!([
                {"note":1,"action":{"kind":"move","target":{"kind":"line","id":1}}},
                {"note":3,"action":{"kind":"delete"}},
            ]),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let after = current(&state);
    assert_eq!(after.notes.len(), 1);
    assert_eq!(after.notes[0].id(), ProjectNoteId(1));
    assert_eq!(after.notes[0].target(), Target::Line(LineId(1)));
}

#[tokio::test]
async fn a_stale_binding_is_refused_without_effect() {
    let state = state(noted(&[(1, Target::Device(DeviceId(2)), "device")]));
    let (_, mut refusal) = call(&state, "DELETE", "/api/devices/2", None).await;
    refusal["snapshotRevision"] = json!(refusal["snapshotRevision"].as_u64().unwrap() + 1);
    let before = current(&state);
    let depth = undo_depth(&state);
    let (status, body) = call(
        &state,
        "POST",
        APPLY,
        Some(apply_body(
            &refusal,
            json!({"kind":"devices","ids":[2]}),
            json!([{"note":1,"action":{"kind":"delete"}}]),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "noteDispositionStale");
    assert_eq!(current(&state), before);
    assert_eq!(undo_depth(&state), depth);
}

#[tokio::test]
async fn incomplete_or_dangling_choices_are_refused_with_a_reason_and_without_effect() {
    let state = state(noted(&[
        (1, Target::Device(DeviceId(1)), "device"),
        (3, Target::ComObject(ComObjectInstanceId(1)), "object"),
    ]));
    let (_, refusal) = call(&state, "DELETE", "/api/devices/1", None).await;
    let before = current(&state);
    let depth = undo_depth(&state);
    for (choices, reason) in [
        (
            json!([{"note":1,"action":{"kind":"delete"}}]),
            "notes without a choice: 3",
        ),
        (
            json!([
                {"note":1,"action":{"kind":"move","target":{"kind":"comObject","id":1}}},
                {"note":3,"action":{"kind":"delete"}},
            ]),
            "which this delete removes",
        ),
    ] {
        let (status, body) = call(
            &state,
            "POST",
            APPLY,
            Some(apply_body(
                &refusal,
                json!({"kind":"devices","ids":[1]}),
                choices,
            )),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["kind"], "noteDispositionRefused");
        assert!(body["error"].as_str().unwrap().contains(reason), "{body}");
        assert_eq!(current(&state), before);
        assert_eq!(undo_depth(&state), depth);
    }
    let mut unknown = apply_body(&refusal, json!({"kind":"devices","ids":[1]}), json!([]));
    unknown["command"] = json!("anything");
    let (status, _) = call(&state, "POST", APPLY, Some(unknown)).await;
    assert!(
        status.is_client_error(),
        "unknown fields are refused: {status}"
    );
    assert_eq!(current(&state), before);
}
