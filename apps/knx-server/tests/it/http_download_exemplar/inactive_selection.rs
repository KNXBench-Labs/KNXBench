//! Verify selected HTTP frames, capture identity and pre-connector freshness refusals.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

const XML: &str =
    include_str!("../../../../knx-cli/tests/fixtures/inactive-selection/source-default-one.xml");

struct CountingConnector(Arc<SimulatedDevice>, Arc<AtomicUsize>);
impl GatewayConnector for CountingConnector {
    fn connect_tunnel(
        &self,
        _: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.1.fetch_add(1, Ordering::SeqCst);
        let tunnel: Box<dyn BusTunnel> = Box::new(Tunnel(self.0.clone()));
        Box::pin(async move { Ok(tunnel) })
    }
    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async { Ok(vec![]) })
    }
}

fn app(
    dir: &std::path::Path,
) -> (
    axum::Router,
    Arc<knx_server::AppState>,
    Arc<SimulatedDevice>,
    Arc<AtomicUsize>,
) {
    let path = dir.join("products.sqlite");
    let products = knx_productdb::open_and_migrate(&path).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/A.xml", XML.as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    let plan = knx_app::download_support::offline_plan(&products, PROGRAM_ID).unwrap();
    let device = Arc::new(device_for(&plan));
    device.preset_memory(0x4400, &[1, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8]);
    let calls = Arc::new(AtomicUsize::new(0));
    let mut state = knx_server::AppState::new(dir.to_path_buf());
    state.product_db = Some(knx_server::CatalogueOwner::adopt_opened(products, path).unwrap());
    state.connector = Box::new(CountingConnector(device.clone(), calls.clone()));
    state.device_download_timing = fast();
    *state.project.lock().unwrap() = Some(project());
    let state = Arc::new(state);
    (knx_server::app(state.clone(), None), state, device, calls)
}

fn scopes() -> [Value; 4] {
    [
        Value::Null,
        json!({"parameters":true,"groupAddresses":false}),
        json!({"parameters":false,"groupAddresses":true}),
        json!({"parameters":true,"groupAddresses":true}),
    ]
}

fn request(partial: Value) -> Value {
    let mut value = json!({"address":TARGET});
    if !partial.is_null() {
        value["partial"] = partial;
    }
    value
}

fn start(shown: &Value) -> Request<Body> {
    post(
        "/api/device-download/start",
        json!({"planId":shown["planId"],"gateway":"192.0.2.10:3671","confirmation":shown["confirmationPhrase"],"acceptUntested":format!("I accept an untested download to {TARGET}")}),
    )
}

async fn finished(app: &axum::Router) -> Value {
    for _ in 0..2000 {
        let (status, body, _) = call(app, get("/api/device-download/status?since=0")).await;
        if status == StatusCode::OK && body["status"]["state"] != "running" {
            assert_eq!(body["status"]["state"], "finished", "{body}");
            return body;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("download failed to settle");
}

#[tokio::test]
async fn http_all_scopes_execute_the_selected_frames_and_capture_the_actual_generator() {
    let cases = scopes()
        .into_iter()
        .enumerate()
        .map(|(i, p)| (i, p, false))
        .chain(scopes().into_iter().enumerate().map(|(i, p)| (i, p, true)));
    for (index, partial, changes) in cases {
        let dir = tempfile::tempdir().unwrap();
        let (app, state, device, calls) = app(dir.path());
        if changes {
            let mut lock = state.project.lock().unwrap();
            let flags = &mut lock
                .as_mut()
                .unwrap()
                .devices
                .get_mut(DeviceId(1))
                .unwrap()
                .commissioning;
            flags.application_program_loaded = index != 0;
            flags.parameters_loaded = index == 2;
            flags.communication_part_loaded = index == 1;
        }
        let request = if changes {
            json!({"address":TARGET,"scope":"changes"})
        } else {
            request(partial)
        };
        let (status, shown, _) = call(&app, post("/api/device-download/plan", request)).await;
        assert_eq!(status, StatusCode::OK, "{shown}");
        let expected_octets = [12, 4, 8, 12][index];
        assert_eq!(shown["dataOctets"], expected_octets);
        assert_eq!(
            shown["segments"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["written"].as_u64().unwrap())
                .sum::<u64>(),
            expected_octets
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let (status, body, _) = call(&app, start(&shown)).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let final_body = finished(&app).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let writes: Vec<_> = device
            .seen()
            .into_iter()
            .filter_map(|s| match s {
                Seen::MemoryWrite { address, data, .. } if address >= 0x4400 => {
                    Some((address, data))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            writes,
            if index == 2 {
                vec![]
            } else {
                vec![
                    (0x4400, vec![1]),
                    (0x4402, vec![0xC7, 0x53]),
                    (0x4405, vec![0x68]),
                ]
            }
        );
        let file = dir.path().join(final_body["backupFile"].as_str().unwrap());
        let stored = knx_app::device_backup::read_backup(&file).unwrap();
        assert_eq!(
            stored.write_selection,
            knx_app::device_backup::WriteSelection::SourceDefaultInactiveWholeBytesV1
        );
        let bytes = std::fs::read(&file).unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["format"], 4);
        assert_eq!(
            json["write_selection"],
            "source-default-inactive-whole-bytes-v1"
        );
        assert!(!final_body.to_string().contains("ABCDEF12"));
    }
}

#[tokio::test]
async fn http_start_reprepares_after_mode_or_source_changes_and_opens_no_connector() {
    for change_mode in [false, true] {
        for partial in scopes() {
            let dir = tempfile::tempdir().unwrap();
            let (app, state, device, calls) = app(dir.path());
            let (status, shown, _) =
                call(&app, post("/api/device-download/plan", request(partial))).await;
            assert_eq!(status, StatusCode::OK, "{shown}");
            if change_mode {
                state
                    .project
                    .lock()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .installations[0]
                    .parameters
                    .push(knx_core::ParameterInstance {
                        id: knx_core::ParameterInstanceId(1),
                        device: DeviceId(1),
                        source: SourceRef {
                            path: "synthetic".into(),
                            ets_id: format!("{PROGRAM_ID}_P-Mode_R-Mode"),
                        },
                        raw: "0".into(),
                    });
            } else {
                let conn =
                    knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
                let xml = XML.replace(&format!("<Memory CodeSegment=\"{PROGRAM_ID}_AS-4400\" Offset=\"1\" BitOffset=\"0\" />"), "<Property ObjectIndex=\"3\" PropertyId=\"99\" />");
                let bytes = xml.into_bytes();
                let sha = knx_productdb::blob::sha256_hex(&bytes);
                knx_productdb::store_source_file(
                    &conn,
                    &knx_productdb::SourceFile {
                        source_path: "M-FFF6/current.xml".into(),
                        manufacturer_id: Some("M-FFF6".into()),
                        bytes,
                    },
                )
                .unwrap();
                conn.execute(
                    "UPDATE application_program SET source_sha256=?1 WHERE id=?2",
                    [&sha, PROGRAM_ID],
                )
                .unwrap();
            }
            let (status, refusal, text) = call(&app, start(&shown)).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refusal}");
            assert_eq!(refusal["kind"], "downloadPreparationRefused");
            assert_eq!(refusal["category"], "inactive-write-selection");
            assert!(refusal["error"].as_str().unwrap().contains(if change_mode {
                "different-source-default-topology"
            } else {
                "unresolved-ownership"
            }));
            assert!(!text.contains(PROGRAM_ID));
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(device.seen().is_empty());
            assert!(state.device_download.lock().await.is_none());
            let (status, ready, _) = call(&app, get("/api/device-readiness")).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(ready["devices"][0]["category"], "inactive-write-selection");
        }
    }
}

#[tokio::test]
async fn http_changes_with_no_pending_does_not_build_or_start_a_plan() {
    let dir = tempfile::tempdir().unwrap();
    let (app, state, device, calls) = app(dir.path());
    {
        let mut lock = state.project.lock().unwrap();
        let flags = &mut lock
            .as_mut()
            .unwrap()
            .devices
            .get_mut(DeviceId(1))
            .unwrap()
            .commissioning;
        flags.application_program_loaded = true;
        flags.parameters_loaded = true;
        flags.communication_part_loaded = true;
        lock.as_mut().unwrap().installations[0]
            .parameters
            .push(knx_core::ParameterInstance {
                id: knx_core::ParameterInstanceId(1),
                device: DeviceId(1),
                source: SourceRef {
                    path: "synthetic".into(),
                    ets_id: format!("{PROGRAM_ID}_P-Mode_R-Mode"),
                },
                raw: "0".into(),
            });
    }
    let (status, body, _) = call(
        &app,
        post(
            "/api/device-download/plan",
            json!({"address":TARGET,"scope":"changes"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["kind"], "downloadNothingPending");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(device.seen().is_empty());
}

const HISTORICAL_ID: &str = "M-0083_A-0027-15-0BAC";
fn correction_app(
    dir: &std::path::Path,
    xml: &str,
    historical: bool,
) -> (
    axum::Router,
    Arc<knx_server::AppState>,
    Arc<SimulatedDevice>,
    Arc<AtomicUsize>,
) {
    let path = dir.join("products.sqlite");
    let products = knx_productdb::open_and_migrate(&path).unwrap();
    let key = if historical {
        HISTORICAL_ID
    } else {
        PROGRAM_ID
    };
    knx_productdb::ingest_file(
        &products,
        "M-FFF6/A.xml",
        xml.replace(PROGRAM_ID, key).as_bytes(),
    )
    .unwrap();
    knx_productdb::ingest_file(
        &products,
        "M-FFF6/Hardware.xml",
        HARDWARE_XML.replace(PROGRAM_ID, key).as_bytes(),
    )
    .unwrap();
    let reading = knx_app::device_download::prepare_device_download_for_reading(
        &products,
        &project(),
        TARGET.parse().unwrap(),
    )
    .unwrap();
    let device = Arc::new(device_for(&reading.plan));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut state = knx_server::AppState::new(dir.to_path_buf());
    state.product_db = Some(knx_server::CatalogueOwner::adopt_opened(products, path).unwrap());
    state.connector = Box::new(CountingConnector(device.clone(), calls.clone()));
    state.device_download_timing = fast();
    *state.project.lock().unwrap() = Some(project());
    let state = Arc::new(state);
    (knx_server::app(state.clone(), None), state, device, calls)
}

#[tokio::test]
async fn f2_http_historical_evidence_requires_acknowledgement_only_for_changed_scoped_plans() {
    for (index, partial) in scopes().into_iter().enumerate() {
        let dir = tempfile::tempdir().unwrap();
        let (app, state, device, calls) = correction_app(dir.path(), XML, true);
        let (status, shown, _) =
            call(&app, post("/api/device-download/plan", request(partial))).await;
        assert_eq!(status, StatusCode::OK, "{shown}");
        let changed = index != 2;
        let (status,body,_)=call(&app,post("/api/device-download/start",json!({"planId":shown["planId"],"gateway":"192.0.2.10:3671","confirmation":shown["confirmationPhrase"]}))).await;
        if status == StatusCode::OK {
            finished(&app).await;
        }
        assert_eq!(
            status,
            if changed {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::OK
            },
            "{body}"
        );
        assert_eq!(
            shown["support"]["level"],
            if changed { "untested" } else { "verified" },
            "{shown}"
        );
        if changed {
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(device.seen().is_empty());
            assert!(state.device_download.lock().await.is_none());
            let (status, body, _) = call(&app, start(&shown)).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            finished(&app).await;
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            let writes: Vec<_> = device
                .seen()
                .into_iter()
                .filter_map(|s| match s {
                    Seen::MemoryWrite { address, data, .. } if address >= 0x4400 => {
                        Some((address, data))
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                writes,
                vec![
                    (0x4400, vec![1]),
                    (0x4402, vec![0xC7, 0x53]),
                    (0x4405, vec![0x68])
                ]
            );
        }
    }
}

#[tokio::test]
async fn f2_http_unchanged_complete_plan_control_keeps_historical_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let xml = XML.replace("Value=\"1\"><Memory", "Value=\"0\"><Memory");
    let (app, _, _, calls) = correction_app(dir.path(), &xml, true);
    let (status, shown, _) = call(
        &app,
        post("/api/device-download/plan", request(Value::Null)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{shown}");
    assert_eq!(shown["support"]["level"], "verified", "{shown}");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

include!("../../../../knx-cli/tests/fixtures/inactive-selection/namespaces.rs");

fn namespace_replace_catalogue(dir: &std::path::Path, xml: &str) {
    let fresh = dir.join("namespace-fresh.sqlite");
    let conn = knx_productdb::open_and_migrate(&fresh).unwrap();
    knx_productdb::ingest_file(&conn, "M-FFF6/A.xml", xml.as_bytes()).unwrap();
    let target = knx_productdb::open_and_migrate(&dir.join("products.sqlite")).unwrap();
    target
        .execute("ATTACH DATABASE ?1 AS fresh", [fresh.to_str().unwrap()])
        .unwrap();
    target
        .execute_batch(
            "DELETE FROM dynamic_node; INSERT INTO dynamic_node SELECT * FROM fresh.dynamic_node;",
        )
        .unwrap();
    knx_productdb::store_source_file(
        &target,
        &knx_productdb::SourceFile {
            source_path: "M-FFF6/current.xml".into(),
            manufacturer_id: Some("M-FFF6".into()),
            bytes: xml.as_bytes().to_vec(),
        },
    )
    .unwrap();
    target
        .execute(
            "UPDATE application_program SET source_sha256=?1 WHERE id=?2",
            [
                knx_productdb::blob::sha256_hex(xml.as_bytes()),
                PROGRAM_ID.to_string(),
            ],
        )
        .unwrap();
}

async fn namespace_refused(
    app: &axum::Router,
    req: Request<Body>,
    calls: &AtomicUsize,
    device: &SimulatedDevice,
    case: &str,
) {
    let (status, body, text) = call(app, req).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{case}: {body}");
    assert_eq!(
        body["category"], "inactive-write-selection",
        "{case}: {body}"
    );
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("unresolved-ownership"),
        "{body}"
    );
    for private in ["urn:synthetic", "e:Parameter", "M-FFF6", "A.xml"] {
        assert!(!text.contains(private), "{text}");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(device.seen().is_empty());
}

#[tokio::test]
async fn f3_namespace_http_plan_refuses_before_connector() {
    for (case, xml) in namespace_cases(XML) {
        let dir = tempfile::tempdir().unwrap();
        let (app, _, device, calls) = correction_app(dir.path(), &xml, false);
        namespace_refused(
            &app,
            post("/api/device-download/plan", request(Value::Null)),
            &calls,
            &device,
            case,
        )
        .await;
    }
}

#[tokio::test]
async fn f3_namespace_http_start_rechecks_retained_context_before_connector() {
    for (case, xml) in namespace_cases(XML) {
        let dir = tempfile::tempdir().unwrap();
        let (app, _, device, calls) = app(dir.path());
        let (status, shown, _) = call(
            &app,
            post("/api/device-download/plan", request(Value::Null)),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{shown}");
        namespace_replace_catalogue(dir.path(), &xml);
        namespace_refused(&app, start(&shown), &calls, &device, case).await;
    }
}

#[tokio::test]
async fn f3_namespace_http_actual_knx_prefix_positive_executes_exact_frames() {
    for scope in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (app, _, device, calls) =
            correction_app(dir.path(), &namespace_positive(XML, scope), false);
        let (status, shown, _) = call(
            &app,
            post("/api/device-download/plan", request(Value::Null)),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{shown}");
        let (status, body, _) = call(&app, start(&shown)).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        finished(&app).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            device
                .seen()
                .into_iter()
                .filter_map(|s| match s {
                    Seen::MemoryWrite { address, data, .. } if address >= 0x4400 =>
                        Some((address, data)),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            vec![
                (0x4400, vec![1]),
                (0x4402, vec![0xC7, 0x53]),
                (0x4405, vec![0x68])
            ]
        );
    }
}

#[test]
#[ignore = "requires a genuine retained pre-namespace CLI-produced catalogue"]
fn f3_namespace_genuine_old_database_http_plan_refuses_before_connector() {
    let original = std::path::PathBuf::from(
        std::env::var_os("KNX_NAMESPACE_PRE_DB").expect("retained producer database"),
    );
    let before = std::fs::read(&original).unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy(&original, dir.path().join("products.sqlite")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let reading = knx_app::device_download::prepare_device_download_for_reading(
        &products,
        &project(),
        TARGET.parse().unwrap(),
    )
    .unwrap();
    let device = Arc::new(device_for(&reading.plan));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut state = knx_server::AppState::new(dir.path().to_path_buf());
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, dir.path().join("products.sqlite"))
            .unwrap(),
    );
    state.connector = Box::new(CountingConnector(device.clone(), calls.clone()));
    *state.project.lock().unwrap() = Some(project());
    let app = knx_server::app(Arc::new(state), None);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(namespace_refused(
            &app,
            post("/api/device-download/plan", request(Value::Null)),
            &calls,
            &device,
            "genuine old database",
        ));
    assert_eq!(std::fs::read(&original).unwrap(), before);
}

#[tokio::test]
async fn f3_http_opaque_dynamic_attributes_refuse_plan_and_start_before_connector() {
    for xml in [
        XML.replace(&format!("<ParameterRefRef RefId=\"{PROGRAM_ID}_P-I_R-I\" />"),
          &format!("<choose Opaque=\"\" ParamRefId=\"{PROGRAM_ID}_P-Mode_R-Mode\"><when test=\"0\"><ParameterRefRef RefId=\"{PROGRAM_ID}_P-I_R-I\" /></when></choose>")),
        XML.replace("<choose ParamRefId=", "<choose Opaque=\"\" ParamRefId="),
        XML.replace(
            "<when test=\"0\">",
            "<when Opaque=\"a=b&#10;c=d\" test=\"0\">",
        ),
        XML.replace(
            &format!("<ParameterRefRef RefId=\"{PROGRAM_ID}_P-I_R-I\""),
            &format!("<ParameterRefRef Opaque=\"private\" RefId=\"{PROGRAM_ID}_P-I_R-I\""),
        ),
    ] {
        for partial in scopes() {
            let dir = tempfile::tempdir().unwrap();
            let (app, state, device, calls) = app(dir.path());
            let (status, shown, _) = call(
                &app,
                post("/api/device-download/plan", request(partial.clone())),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{shown}");
            // Replace normalized rows through a fresh real parser, preserving source evidence.
            let fresh = dir.path().join("fresh.sqlite");
            let conn = knx_productdb::open_and_migrate(&fresh).unwrap();
            knx_productdb::ingest_file(&conn, "M-FFF6/A.xml", xml.as_bytes()).unwrap();
            knx_productdb::ingest_file(&conn, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes())
                .unwrap();
            // The catalogue owner is immutable while leased; re-ingest is represented by
            // updating both retained bytes and the exact rows in this test-owned database.
            let target =
                knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
            target
                .execute("ATTACH DATABASE ?1 AS fresh", [fresh.to_str().unwrap()])
                .unwrap();
            target.execute_batch("DELETE FROM dynamic_node; INSERT INTO dynamic_node SELECT * FROM fresh.dynamic_node;").unwrap();
            knx_productdb::store_source_file(
                &target,
                &knx_productdb::SourceFile {
                    source_path: "M-FFF6/current.xml".into(),
                    manufacturer_id: Some("M-FFF6".into()),
                    bytes: xml.as_bytes().to_vec(),
                },
            )
            .unwrap();
            target
                .execute(
                    "UPDATE application_program SET source_sha256=?1 WHERE id=?2",
                    [
                        knx_productdb::blob::sha256_hex(xml.as_bytes()),
                        PROGRAM_ID.to_string(),
                    ],
                )
                .unwrap();
            drop(target);
            drop(conn);
            for req in [
                start(&shown),
                post("/api/device-download/plan", request(partial)),
            ] {
                let (status, body, text) = call(&app, req).await;
                assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
                assert_eq!(body["category"], "inactive-write-selection", "{body}");
                assert!(
                    body["error"]
                        .as_str()
                        .unwrap()
                        .contains("unresolved-ownership"),
                    "{body}"
                );
                assert!(
                    !text.contains("Opaque") && !text.contains(PROGRAM_ID),
                    "{text}"
                );
                assert_eq!(calls.load(Ordering::SeqCst), 0);
                assert!(device.seen().is_empty());
                assert!(state.device_download.lock().await.is_none());
            }
        }
    }
}
