//! Written-section settlement, independent comparisons and exact reference-to-row attribution.

use super::project_edits::{edit_link, install_rich_project, rich_project};
use super::*;
use crate::domain;
use knx_core::commissioning::parameter_baseline::{
    BaselineChange, BaselineEdit, BaselineLink, LinkBaseline, LinkBaselineChange,
    ParameterBaseline, WrittenObjectFlags,
};
use knx_core::{
    ComObjectInstanceId, Direction, GroupAddress, GroupAddressId, GroupLink, Layer, Override,
    Resolved,
};
use knx_projection::{DownloadComparisonState as State, ObjectLastDownload};

fn flags() -> WrittenObjectFlags {
    WrittenObjectFlags {
        read: true,
        write: false,
        transmit: true,
        update: false,
        communication: true,
        read_on_init: None,
    }
}
fn seed(state: &SharedState) {
    state
        .parameter_baselines
        .lock()
        .unwrap()
        .apply(&BaselineEdit {
            device: DeviceId(1),
            parameter: BaselineChange::Record(ParameterBaseline {
                program_id: PROGRAM_ID.into(),
                values: Default::default(),
                object_flags: Some([(0, flags())].into()),
                at: chrono::DateTime::UNIX_EPOCH,
            }),
            links: LinkBaselineChange::Record(LinkBaseline {
                program_id: PROGRAM_ID.into(),
                at: chrono::DateTime::UNIX_EPOCH,
                links: vec![BaselineLink {
                    object: 0,
                    group_address: GroupAddress::from_raw(0x1035),
                    sending: true,
                }],
            }),
        });
}
fn detail(state: &SharedState) -> ObjectLastDownload {
    domain::device_detail(state, 1, None)
        .unwrap()
        .last_download
        .expect("GET comparison")
}
fn edit(state: &SharedState, change: impl FnOnce(&mut Project)) {
    change(state.project.lock().unwrap().as_mut().unwrap());
}
fn stated(value: bool) -> Override<bool> {
    Override::Value(Resolved {
        value,
        layer: Layer::UserEdit,
    })
}
fn row2(project: &mut Project, reference: &str, number: u16) {
    let mut row = project
        .devices
        .com_object(ComObjectInstanceId(1))
        .unwrap()
        .clone();
    row.id = ComObjectInstanceId(2);
    row.source.ets_id = reference.into();
    row.number = number;
    row.links.clear();
    project.devices.insert_com_object(row);
    project
        .devices
        .get_mut(DeviceId(1))
        .unwrap()
        .com_objects
        .push(ComObjectInstanceId(2));
    project.ids.next_com_object_instance_id().unwrap();
}

#[test]
fn completed_settlement_records_written_links_and_effective_flags_despite_current_edits() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    let prepared = prepare(&f.state);
    let settler = download_settler(&f.state, super::super::incarnation(&f.state), &prepared);
    edit_link(&f.state);
    edit(&f.state, |p| {
        p.devices
            .com_object_mut(ComObjectInstanceId(1))
            .unwrap()
            .flags
            .write = stated(true)
    });
    settler(&finished());
    let all = f.state.parameter_baselines.lock().unwrap().clone();
    assert_eq!(
        all.get(DeviceId(1)).unwrap().object_flags,
        Some([(0, flags())].into())
    );
    assert_eq!(
        all.get_links(DeviceId(1)).unwrap().links,
        vec![BaselineLink {
            object: 0,
            group_address: GroupAddress::from_raw(0x1035),
            sending: true
        }]
    );
    let comparison = detail(&f.state);
    assert_eq!(
        (comparison.flags.state, comparison.links.state),
        (State::Usable, State::Usable)
    );
    assert_eq!(
        comparison.differing_objects, 1,
        "mixed differences on one number count once"
    );
    assert_eq!(comparison.differing[0].instance_id, Some(1));
    assert_eq!(comparison.differing[0].links.len(), 1);
    assert_eq!(
        comparison.differing[0]
            .flags
            .as_ref()
            .unwrap()
            .downloaded
            .as_ref()
            .unwrap()
            .read_on_init,
        None
    );
    assert_eq!(
        comparison.differing[0]
            .flags
            .as_ref()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .read_on_init,
        None
    );
}

#[test]
fn section_known_states_and_program_resolution_are_independent() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    assert_eq!(detail(&f.state).flags.state, State::Absent);
    partial(&f, false, true, false);
    assert_eq!(
        (detail(&f.state).flags.state, detail(&f.state).links.state),
        (State::Absent, State::Usable)
    );
    seed(&f.state);
    let mut all = f.state.parameter_baselines.lock().unwrap();
    let mut parameter = all.get(DeviceId(1)).unwrap().clone();
    parameter.program_id = "Other".into();
    all.apply(&BaselineEdit {
        device: DeviceId(1),
        parameter: BaselineChange::Record(parameter),
        links: LinkBaselineChange::Keep,
    });
    drop(all);
    let d = detail(&f.state);
    assert_eq!(
        (d.flags.state, d.links.state),
        (State::OtherProgram, State::Usable)
    );
    edit(&f.state, |p| {
        p.devices.get_mut(DeviceId(1)).unwrap().program_ref = "missing".into()
    });
    let d = detail(&f.state);
    assert_eq!(
        (d.flags.state, d.links.state),
        (State::ProductUnavailable, State::ProductUnavailable)
    );
}

#[test]
fn missing_individual_address_refuses_objects_without_suppressing_parameter_comparison() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    full(&f, false);
    edit(&f.state, |p| {
        p.devices.get_mut(DeviceId(1)).unwrap().address = None
    });
    let d = detail(&f.state);
    assert_eq!(
        (d.links.state, d.flags.state),
        (State::CurrentUnreadable, State::CurrentUnreadable)
    );
    assert_eq!(
        domain::parameter_panel_impl(&f.state, 1, None)
            .unwrap()
            .last_download
            .unwrap()
            .state,
        crate::parameter_baseline::LastDownloadState::Usable
    );
}

#[test]
fn link_mapping_refusals_do_not_hide_usable_flags() {
    for case in 0..4 {
        let f = fixture(&OLD, &OLD);
        install_rich_project(&f.state);
        seed(&f.state);
        edit(&f.state, |p| match case {
            0 => {
                p.devices
                    .com_object_mut(ComObjectInstanceId(1))
                    .unwrap()
                    .number = 256
            }
            1 => {
                p.devices
                    .com_object_mut(ComObjectInstanceId(1))
                    .unwrap()
                    .links[0]
                    .ga = GroupAddressId(999)
            }
            2 => p
                .devices
                .com_object_mut(ComObjectInstanceId(1))
                .unwrap()
                .links
                .push(GroupLink {
                    ga: GroupAddressId(2),
                    direction: Direction::Send,
                }),
            _ => {
                row2(p, &format!("{PROGRAM_ID}_O-0_R-INACTIVE"), 0);
                p.devices
                    .com_object_mut(ComObjectInstanceId(2))
                    .unwrap()
                    .links = vec![GroupLink {
                    ga: GroupAddressId(2),
                    direction: Direction::Receive,
                }];
            }
        });
        let d = detail(&f.state);
        assert_eq!(d.links.state, State::CurrentUnreadable, "case {case}");
        assert_eq!(d.flags.state, State::Usable, "case {case}");
    }
}

#[test]
fn unreadable_overrides_and_duplicate_unscoped_lookup_ids_refuse_only_flags() {
    for case in 0..3 {
        let f = fixture(&OLD, &OLD);
        install_rich_project(&f.state);
        seed(&f.state);
        edit(&f.state, |p| match case {
            0 => {
                p.devices
                    .com_object_mut(ComObjectInstanceId(1))
                    .unwrap()
                    .flags
                    .write = Override::Empty
            }
            1 => {
                p.devices
                    .com_object_mut(ComObjectInstanceId(1))
                    .unwrap()
                    .flags
                    .read = Override::Malformed("maybe".into())
            }
            _ => row2(p, &format!("{PROGRAM_ID}_O-0_R-1"), 0),
        });
        let d = detail(&f.state);
        assert_eq!(
            (d.flags.state, d.links.state),
            (State::CurrentUnreadable, State::Usable),
            "case {case}"
        );
        assert!(d.differing.is_empty());
    }
}

#[test]
fn attribution_uses_exact_active_reference_or_linked_preference_not_same_number_row_count() {
    for linked in [false, true] {
        let f = fixture(&OLD, &OLD);
        install_rich_project(&f.state);
        seed(&f.state);
        edit(&f.state, |p| {
            row2(p, &format!("{PROGRAM_ID}_O-0_R-1"), 0);
            let one = p.devices.com_object_mut(ComObjectInstanceId(1)).unwrap();
            one.source.ets_id = format!("{PROGRAM_ID}_O-0_R-INACTIVE");
            if !linked {
                one.links.clear();
            }
            p.devices
                .com_object_mut(ComObjectInstanceId(2))
                .unwrap()
                .flags
                .write = stated(true);
        });
        let d = detail(&f.state);
        assert_eq!(d.flags.state, State::Usable);
        assert_eq!(d.differing_objects, 1);
        assert_eq!(d.no_selected_row, 0);
        assert_eq!(d.differing[0].instance_id, Some(if linked { 1 } else { 2 }));
        assert!(d.instances.contains_key(&if linked { 1 } else { 2 }));
    }
}

#[test]
fn current_only_product_objects_without_project_rows_and_stored_only_inactive_rows_are_counted() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    seed(&f.state);
    edit(&f.state, |p| {
        let row = p.devices.com_object_mut(ComObjectInstanceId(1)).unwrap();
        row.source.ets_id = format!("{PROGRAM_ID}_O-7_R-INACTIVE");
        row.number = 7;
        row.links.clear();
    });
    let mut parameter = f
        .state
        .parameter_baselines
        .lock()
        .unwrap()
        .get(DeviceId(1))
        .unwrap()
        .clone();
    parameter.object_flags = Some([(7, flags())].into());
    f.state
        .parameter_baselines
        .lock()
        .unwrap()
        .apply(&BaselineEdit {
            device: DeviceId(1),
            parameter: BaselineChange::Record(parameter),
            links: LinkBaselineChange::Record(LinkBaseline {
                program_id: PROGRAM_ID.into(),
                at: chrono::DateTime::UNIX_EPOCH,
                links: vec![],
            }),
        });
    let d = detail(&f.state);
    assert_eq!(d.flags.state, State::Usable);
    assert_eq!(d.differing_objects, 2);
    assert_eq!(d.no_selected_row, 2);
    assert!(d.instances.is_empty());
    assert!(d.differing[0].flags.as_ref().unwrap().downloaded.is_none());
    assert!(d.differing[1].flags.as_ref().unwrap().current.is_none());
}

#[test]
fn raw_address_presence_and_direction_are_compared_exactly_and_sorted() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    seed(&f.state);
    edit(&f.state, |p| {
        let row = p.devices.com_object_mut(ComObjectInstanceId(1)).unwrap();
        row.links[0].direction = Direction::Receive;
        row.links.push(GroupLink {
            ga: GroupAddressId(2),
            direction: Direction::Send,
        });
    });
    let d = detail(&f.state);
    let diffs = &d.differing[0].links;
    assert_eq!(diffs.len(), 2);
    assert_eq!(
        (
            diffs[0].group_address,
            diffs[0].downloaded,
            diffs[0].current
        ),
        (0x1035, Some(true), Some(false))
    );
    assert_eq!(
        (
            diffs[1].group_address,
            diffs[1].downloaded,
            diffs[1].current
        ),
        (0x1236, None, Some(true))
    );
}

fn custom(xml: &str) -> Fixture {
    let mut f = fixture(&OLD, &OLD);
    // Replace via an independent catalogue file, never mutate a leased connection.
    let path = f._dir.path().join("custom.sqlite");
    let conn = catalogue_file(&path, xml);
    f.state = Arc::new(AppState {
        product_db: Some(CatalogueOwner::adopt_opened(conn, path).unwrap()),
        ..AppState::default()
    });
    *f.state.project.lock().unwrap() = Some(rich_project());
    f
}

#[test]
fn default_equal_and_inactive_overrides_are_not_written_differences() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    seed(&f.state);
    edit(&f.state, |p| {
        row2(p, &format!("{PROGRAM_ID}_O-9_R-INACTIVE"), 9);
        p.devices
            .com_object_mut(ComObjectInstanceId(1))
            .unwrap()
            .flags
            .read = stated(true);
        p.devices
            .com_object_mut(ComObjectInstanceId(2))
            .unwrap()
            .flags
            .write = stated(true);
    });
    let d = detail(&f.state);
    assert_eq!(d.flags.state, State::Usable);
    assert_eq!(d.differing_objects, 0);
}

#[test]
fn uncertain_activation_and_duplicate_active_refs_refuse_flags_without_hiding_link_differences() {
    for duplicate in [false, true] {
        let xml = if duplicate {
            program_xml(&OLD).replace("</ComObjectRefs>",&format!("<ComObjectRef Id=\"{PROGRAM_ID}_O-0_R-2\" RefId=\"{PROGRAM_ID}_O-0\" /></ComObjectRefs>"))
                .replace("</ParameterBlock>",&format!("<ComObjectRefRef RefId=\"{PROGRAM_ID}_O-0_R-2\" /></ParameterBlock>"))
        } else {
            program_xml(&OLD).replace(&format!("<ComObjectRefRef RefId=\"{PROGRAM_ID}_O-0_R-1\" />"),&format!("<choose ParamRefId=\"missing-controller\"><when test=\"1\"><ComObjectRefRef RefId=\"{PROGRAM_ID}_O-0_R-1\" /></when></choose>"))
        };
        let f = custom(&xml);
        seed(&f.state);
        edit(&f.state, |p| {
            p.devices
                .com_object_mut(ComObjectInstanceId(1))
                .unwrap()
                .links
                .clear()
        });
        let d = detail(&f.state);
        assert_eq!(
            (d.flags.state, d.links.state),
            (State::CurrentUnreadable, State::Usable)
        );
        assert_eq!(d.differing_objects, 1);
        assert_eq!(d.no_selected_row, 1);
        assert!(d.differing[0].flags.is_none());
    }
}

#[test]
fn object_size_and_priority_image_admission_do_not_suppress_independently_readable_sections() {
    for bad_size in [false, true] {
        let xml = if bad_size {
            program_xml(&OLD).replace("ObjectSize=\"1 Bit\"", "ObjectSize=\"unreadable\"")
        } else {
            program_xml(&OLD).replace("Name=\"Switch\"", "Name=\"Switch\" Priority=\"Alarm\"")
        };
        let f = custom(&xml);
        seed(&f.state);
        edit(&f.state, |p| {
            p.devices
                .com_object_mut(ComObjectInstanceId(1))
                .unwrap()
                .flags
                .write = stated(true)
        });
        assert!(f
            .state
            .product_db
            .as_ref()
            .unwrap()
            .with_read(|conn| prepare_device_download(
                conn,
                f.state.project.lock().unwrap().as_ref().unwrap(),
                address()
            ))
            .unwrap()
            .is_err());
        let d = detail(&f.state);
        assert_eq!(
            (d.flags.state, d.links.state),
            (State::Usable, State::Usable)
        );
        assert_eq!(d.differing_objects, 1);
    }
}

#[test]
fn link_only_evidence_survives_first_save_save_as_plain_save_and_failed_settlement_is_atomic() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    partial(&f, false, true, false);
    let before = f.state.parameter_baselines.lock().unwrap().clone();
    assert!(before.get(DeviceId(1)).is_none());
    assert!(before.get_links(DeviceId(1)).is_some());
    let first = f._dir.path().join("first.knxdb");
    domain::save_project_as(&f.state, &first).unwrap();
    let second = f._dir.path().join("second.knxdb");
    domain::save_project_as(&f.state, &second).unwrap();
    domain::save_project(&f.state).unwrap();
    for path in [&first, &second] {
        let file = knx_store::open_existing_read_only(path).unwrap();
        assert_eq!(
            knx_store::project_history::load_parameter_baselines(&file.conn).unwrap(),
            before
        );
    }
    let flags_before = belief(&f.state);
    let revision = domain::current_project_revision(&f.state);
    let generation = *f.state.history_generation.value.lock().unwrap();
    f.state
        .store_path
        .lock()
        .unwrap()
        .set(Some(f._dir.path().join("missing/file.knxdb")));
    full(&f, false);
    assert_eq!(*f.state.parameter_baselines.lock().unwrap(), before);
    assert_eq!(belief(&f.state), flags_before);
    assert_eq!(domain::current_project_revision(&f.state), revision);
    assert_eq!(
        *f.state.history_generation.value.lock().unwrap(),
        generation
    );
}

async fn http(
    state: &SharedState,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> serde_json::Value {
    use tower::ServiceExt;
    let mut request = axum::http::Request::builder().method(method).uri(uri);
    let body = if let Some(body) = body {
        request = request.header("content-type", "application/json");
        axum::body::Body::from(body.to_string())
    } else {
        axum::body::Body::empty()
    };
    let response = crate::app(state.clone(), None)
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status, axum::http::StatusCode::OK, "{value}");
    value
}

#[tokio::test]
async fn real_settlement_durable_reopen_get_detail_and_edit_response_contract() {
    use serde_json::json;
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    let path = f._dir.path().join("settled.knxdb");
    domain::save_project_as(&f.state, &path).unwrap();
    full(&f, false);
    let initial = http(&f.state, "GET", "/api/device/1", None).await;
    assert_eq!(initial["last_download"]["flags"]["state"], "usable");
    assert_eq!(initial["last_download"]["links"]["state"], "usable");
    let changed = http(
        &f.state,
        "POST",
        "/api/com-object-flag",
        Some(json!({"comObjectId":1,"flag":"Write","value":true})),
    )
    .await;
    assert!(
        changed.get("last_download").is_none(),
        "edit responses do not carry comparison"
    );
    let edited = http(&f.state, "GET", "/api/device/1", None).await;
    assert_eq!(edited["last_download"]["differing_objects"], 1);
    assert!(
        edited["last_download"]["differing"][0]["flags"]["downloaded"]["read_on_init"].is_null()
    );
    let live = f.live.clone();
    drop(f.state);
    let conn = knx_productdb::open_and_migrate(&live).unwrap();
    let restarted = Arc::new(AppState {
        product_db: Some(CatalogueOwner::adopt_opened(conn, live).unwrap()),
        ..AppState::default()
    });
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
    domain::open_native_project(&restarted, &path, true, &progress).unwrap();
    let after = http(&restarted, "GET", "/api/device/1", None).await;
    assert_eq!(
        after["last_download"], edited["last_download"],
        "fresh server state reads durable evidence without settling again"
    );
}

#[test]
fn missing_flag_source_is_product_unavailable_without_hiding_links_or_parameter_values() {
    let mut f = fixture(&OLD, &OLD);
    let path = f._dir.path().join("missing-source.sqlite");
    let conn = catalogue_file(&path, &program_xml(&OLD));
    conn.execute(
        "UPDATE application_program SET source_sha256='missing-source'",
        [],
    )
    .unwrap();
    f.state = Arc::new(AppState {
        product_db: Some(CatalogueOwner::adopt_opened(conn, path).unwrap()),
        ..AppState::default()
    });
    install_rich_project(&f.state);
    seed(&f.state);
    let d = detail(&f.state);
    assert_eq!(d.links.state, State::Usable);
    assert_eq!(d.flags.state, State::ProductUnavailable);
    assert_eq!(
        domain::parameter_panel_impl(&f.state, 1, None)
            .unwrap()
            .last_download
            .unwrap()
            .state,
        crate::parameter_baseline::LastDownloadState::Usable
    );
}

#[test]
fn undo_redo_never_rolls_back_parameter_flags_or_link_evidence() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    full(&f, false);
    domain::set_com_object_flag_impl(&f.state, 1, "Write".into(), true).unwrap();
    full(&f, false);
    let held = f.state.parameter_baselines.lock().unwrap().clone();
    assert!(
        held.get(DeviceId(1))
            .unwrap()
            .object_flags
            .as_ref()
            .unwrap()[&0]
            .write
    );
    domain::undo_impl(&f.state).unwrap();
    assert_eq!(*f.state.parameter_baselines.lock().unwrap(), held);
    assert_eq!(detail(&f.state).differing_objects, 1);
    domain::redo_impl(&f.state).unwrap();
    assert_eq!(*f.state.parameter_baselines.lock().unwrap(), held);
    assert_eq!(detail(&f.state).differing_objects, 0);
}

mod process_restart;
