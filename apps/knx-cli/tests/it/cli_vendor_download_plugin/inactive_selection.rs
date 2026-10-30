//! Verify real CLI writes, restore identity, freshness and persisted inactive values.
use super::*;
use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::properties::{ObjectIndex, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
use knx_net::commissioning::simulator_gateway::{serve, GatewayConfig};
use std::sync::Arc;

const XML: &str = include_str!("../../fixtures/inactive-selection/source-default-one.xml");

fn inputs(dir: &Path) -> (PathBuf, PathBuf) {
    let (project, _) = super::inputs(dir, false);
    let path = dir.join("inactive-products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    knx_productdb::ingest_file(&conn, "M-FFF6/A.xml", XML.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    (project, path)
}

fn mode(project_path: &Path, value: &str) {
    let conn = knx_store::open_and_migrate(project_path).unwrap();
    let mut project = knx_store::load_project(&conn).unwrap();
    let id = project.ids.next_parameter_instance_id().unwrap();
    project.installations[0]
        .parameters
        .push(knx_core::ParameterInstance {
            id,
            device: knx_core::DeviceId(1),
            source: knx_core::SourceRef {
                path: "synthetic".into(),
                ets_id: format!("{PROGRAM_ID}_P-Mode_R-Mode"),
            },
            raw: value.into(),
        });
    knx_store::save_project(&conn, &project).unwrap();
}

struct Gateway {
    address: String,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Gateway {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.worker.take().unwrap().join().unwrap();
    }
}
fn gateway(device: Arc<SimulatedDevice>) -> Gateway {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let std::net::SocketAddr::V4(address) = socket.local_addr().unwrap() else {
        unreachable!()
    };
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let (ready, started) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            socket.set_nonblocking(true).unwrap();
            let socket = tokio::net::UdpSocket::from_std(socket).unwrap();
            let config = GatewayConfig::new(address,"1.1.0".parse().unwrap());
            ready.send(()).unwrap();
            tokio::select! { result = serve(device,socket,config) => { result.unwrap(); }, _ = stopped => {} }
        });
    });
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    Gateway {
        address: address.to_string(),
        stop: Some(stop),
        worker: Some(worker),
    }
}
fn device() -> Arc<SimulatedDevice> {
    let device = SimulatedDevice::with_config_at(
        TARGET.parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0701,
            serial_number: Some([0xFF, 0xF6, 1, 2, 3, 4]),
            ..SimulatorConfig::default()
        },
    );
    device.preset_property(0, PID_MANUFACTURER_ID, &[0xFF, 0xF6]);
    device.preset_property(3, PID_PROGRAM_VERSION, &[0xFF, 0xF6, 0, 1, 1]);
    for machine in MemoryLoadStateMachine::ALL {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Loaded);
    }
    device.preset_memory(0x4000, &[0xB4; 8]);
    device.preset_memory(0x4400, &[1, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8]);
    Arc::new(device)
}
fn write(args: &[&str], project: &Path, products: &Path, gateway: &str, dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .arg("--project")
        .arg(project)
        .arg("--product-db")
        .arg(products)
        .arg("--activity-history")
        .arg(dir.join("history.sqlite"))
        .arg("--backup-dir")
        .arg(dir.join("backups"))
        .args([
            "--gateway",
            gateway,
            "--confirm",
            &format!("I confirm download to {TARGET}"),
            "--accept-untested",
            &format!("I accept an untested download to {TARGET}"),
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn real_cli_all_concrete_scopes_write_exact_selected_application_frames_and_new_backup_identity() {
    for (index, partial) in [
        None,
        Some("parameters"),
        Some("group-addresses"),
        Some("both"),
    ]
    .into_iter()
    .enumerate()
    {
        let dir = tempfile::tempdir().unwrap();
        let (project, products) = inputs(dir.path());
        let mut args = vec!["device", "download", TARGET];
        if let Some(partial) = partial {
            args.extend(["--partial", partial]);
        }
        let shown = knx(&args, &project, &products);
        assert!(shown.status.success(), "{}", text(&shown));
        let device = device();
        let gateway = gateway(device.clone());
        let output = write(&args, &project, &products, &gateway.address, dir.path());
        assert!(output.status.success(), "{}", text(&output));
        drop(gateway);
        let frames: Vec<_> = device
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
            frames,
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
        let files: Vec<_> = std::fs::read_dir(dir.path().join("backups"))
            .unwrap()
            .map(|f| f.unwrap().path())
            .collect();
        assert_eq!(files.len(), 1);
        assert_eq!(
            knx_app::device_backup::read_backup(&files[0])
                .unwrap()
                .write_selection,
            knx_app::device_backup::WriteSelection::SourceDefaultInactiveWholeBytesV1
        );
    }
}

#[test]
fn confirmed_cli_reprepares_mode_and_source_changes_and_emits_no_datagram() {
    for change_mode in [false, true] {
        for partial in [
            None,
            Some("parameters"),
            Some("group-addresses"),
            Some("both"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let (project, products) = inputs(dir.path());
            let mut args = vec!["device", "download", TARGET];
            if let Some(partial) = partial {
                args.extend(["--partial", partial]);
            }
            assert!(knx(&args, &project, &products).status.success());
            if change_mode {
                mode(&project, "0");
            } else {
                let conn = knx_productdb::open_and_migrate(&products).unwrap();
                let bytes=XML.replace(&format!("<Memory CodeSegment=\"{PROGRAM_ID}_AS-4400\" Offset=\"1\" BitOffset=\"0\" />"),"<Property ObjectIndex=\"3\" PropertyId=\"99\" />").into_bytes();
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
            let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
            peer.set_nonblocking(true).unwrap();
            let output = write(
                &args,
                &project,
                &products,
                &peer.local_addr().unwrap().to_string(),
                dir.path(),
            );
            assert!(!output.status.success());
            let shown = text(&output);
            assert!(shown.contains("inactive-write-selection"), "{shown}");
            assert!(
                shown.contains(if change_mode {
                    "different-source-default-topology"
                } else {
                    "unresolved-ownership"
                }),
                "{shown}"
            );
            assert!(shown.contains("written to the device: no"));
            assert!(!shown.contains(PROGRAM_ID));
            assert_eq!(
                peer.recv_from(&mut [0; 2048]).unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            assert!(!dir.path().join("backups").exists());
        }
    }
}

#[test]
fn real_cli_legacy_restore_and_its_fresh_capture_preserve_the_bound_generator() {
    for fixture in [
        include_str!("../../fixtures/inactive-selection/legacy-complete.backup.json"),
        include_str!("../../fixtures/inactive-selection/legacy-parameters.backup.json"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (_, products) = inputs(dir.path());
        let original = dir.path().join("original.backup.json");
        std::fs::write(&original, fixture).unwrap();
        let device = device();
        device.preset_property(
            0,
            knx_core::commissioning::properties::PID_SERIAL_NUMBER,
            &[0xFF, 0xF6, 0x12, 0x34, 0x56, 0x78],
        );
        device.preset_memory(0x4400, &[1, 0x9A, 0xC7, 0x53, 0x6D, 0x68]);
        let gateway = gateway(device.clone());
        let mut input = original.clone();
        for round in 0..2 {
            let backups = dir.path().join(format!("restore-{round}"));
            let before = device.seen().len();
            let output = Command::new(env!("CARGO_BIN_EXE_knx"))
                .args(["device", "restore"])
                .arg(&input)
                .arg("--product-db")
                .arg(&products)
                .arg("--activity-history")
                .arg(dir.path().join("restore-history.sqlite"))
                .arg("--backup-dir")
                .arg(&backups)
                .args([
                    "--gateway",
                    &gateway.address,
                    "--confirm",
                    &format!("I confirm download to {TARGET}"),
                    "--accept-untested",
                    &format!("I accept an untested download to {TARGET}"),
                ])
                .stdin(Stdio::null())
                .output()
                .unwrap();
            assert!(output.status.success(), "{}", text(&output));
            let frames: Vec<_> = device
                .seen()
                .into_iter()
                .skip(before)
                .filter_map(|s| match s {
                    Seen::MemoryWrite { address, data, .. } if address >= 0x4400 => {
                        Some((address, data))
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                frames,
                if round == 0 {
                    vec![(0x4400, vec![1, 0xD4, 0xD5, 0xD6]), (0x4405, vec![0xD8])]
                } else {
                    vec![(0x4400, vec![1, 0x9A, 0xC7, 0x53]), (0x4405, vec![0x68])]
                }
            );
            let files: Vec<_> = std::fs::read_dir(&backups)
                .unwrap()
                .map(|f| f.unwrap().path())
                .collect();
            assert_eq!(files.len(), 1);
            input = files[0].clone();
            let stored = knx_app::device_backup::read_backup(&input).unwrap();
            assert_eq!(
                stored.write_selection,
                knx_app::device_backup::WriteSelection::SourceMaskOnlyV1
            );
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
            assert_eq!(value["format"], 4);
            assert_eq!(value["write_selection"], "source-mask-only-v1");
        }
        drop(gateway);
        assert_eq!(std::fs::read(&original).unwrap(), fixture.as_bytes());
    }
}

#[test]
fn cli_changes_and_save_reopen_keep_inactive_values_and_do_not_persist_a_write_mask() {
    let dir = tempfile::tempdir().unwrap();
    let (path, products) = inputs(dir.path());
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let mut project = knx_store::load_project(&conn).unwrap();
    let id = project.ids.next_parameter_instance_id().unwrap();
    project.installations[0]
        .parameters
        .push(knx_core::ParameterInstance {
            id,
            device: knx_core::DeviceId(1),
            source: knx_core::SourceRef {
                path: "synthetic".into(),
                ets_id: format!("{PROGRAM_ID}_P-I_R-I"),
            },
            raw: "226".into(),
        });
    knx_store::save_project(&conn, &project).unwrap();
    drop(conn);
    let conn = knx_store::open_and_migrate(&path).unwrap();
    assert_eq!(knx_store::load_project(&conn).unwrap(), project);
    drop(conn);
    for (parameters, ga, application, expected) in [
        (true, true, true, "complete"),
        (true, false, false, "parameters"),
        (false, true, false, "group-addresses"),
        (true, true, false, "both"),
        (false, false, false, "nothing to download"),
    ] {
        let conn = knx_store::open_and_migrate(&path).unwrap();
        let flags = &mut project
            .devices
            .get_mut(knx_core::DeviceId(1))
            .unwrap()
            .commissioning;
        flags.application_program_loaded = !application;
        flags.parameters_loaded = !parameters;
        flags.communication_part_loaded = !ga;
        knx_store::save_project(&conn, &project).unwrap();
        drop(conn);
        let output = knx(
            &["device", "download", TARGET, "--changes"],
            &path,
            &products,
        );
        assert!(output.status.success(), "{}", text(&output));
        assert!(text(&output).contains(expected), "{}", text(&output));
    }
    let conn = knx_store::open_and_migrate(&path).unwrap();
    assert_eq!(
        knx_store::load_project(&conn).unwrap().installations[0].parameters[0].raw,
        "226"
    );
}

/// Only the fixture's catalogue key is aliased to an existing historical evidence key.
/// No shipped evidence or hardware claim is added by this synthetic test.
const HISTORICAL_ID: &str = "M-0083_A-0027-15-0BAC";
fn correction_inputs(dir: &Path, xml: &str, historical: bool) -> (PathBuf, PathBuf) {
    let (project, _) = super::inputs(dir, false);
    let path = dir.join("correction-products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    let key = if historical {
        HISTORICAL_ID
    } else {
        PROGRAM_ID
    };
    knx_productdb::ingest_file(
        &conn,
        "M-FFF6/A.xml",
        xml.replace(PROGRAM_ID, key).as_bytes(),
    )
    .unwrap();
    knx_productdb::ingest_file(
        &conn,
        "M-FFF6/Hardware.xml",
        HARDWARE_XML.replace(PROGRAM_ID, key).as_bytes(),
    )
    .unwrap();
    (project, path)
}

#[test]
fn f2_real_cli_historical_evidence_does_not_waive_changed_plan_acknowledgement() {
    for partial in [None, Some("parameters"), Some("both")] {
        let dir = tempfile::tempdir().unwrap();
        let (project, products) = correction_inputs(dir.path(), XML, true);
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_nonblocking(true).unwrap();
        let mut args = vec!["device", "download", TARGET];
        if let Some(partial) = partial {
            args.extend(["--partial", partial]);
        }
        let output = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args(&args)
            .arg("--project")
            .arg(&project)
            .arg("--product-db")
            .arg(&products)
            .arg("--activity-history")
            .arg(dir.path().join("history.sqlite"))
            .arg("--backup-dir")
            .arg(dir.path().join("backups"))
            .arg("--gateway")
            .arg(peer.local_addr().unwrap().to_string())
            .args(["--confirm", &format!("I confirm download to {TARGET}")])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let shown = text(&output);
        assert!(!output.status.success(), "{shown}");
        assert!(
            shown.contains("--accept-untested"),
            "historical evidence must not bypass acknowledgement: {shown}"
        );
        assert!(shown.contains("support: UNTESTED"), "{shown}");
        assert!(!shown.contains("support: verified"), "{shown}");
        assert_eq!(
            peer.recv_from(&mut [0; 2048]).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let device = device();
        let gateway = gateway(device.clone());
        device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x83]);
        device.preset_property(3, PID_PROGRAM_VERSION, &[0x00, 0x83, 0, 1, 1]);
        let accepted = write(&args, &project, &products, &gateway.address, dir.path());
        assert!(accepted.status.success(), "{}", text(&accepted));
        drop(gateway);
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
fn f2_real_cli_identical_historical_plan_control_remains_verified() {
    for (xml, partial) in [
        (
            XML.replace("Value=\"1\"><Memory", "Value=\"0\"><Memory"),
            None,
        ),
        (XML.to_string(), Some("group-addresses")),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (project, products) = correction_inputs(dir.path(), &xml, true);
        let mut args = vec!["device", "download", TARGET];
        if let Some(partial) = partial {
            args.extend(["--partial", partial]);
        }
        let output = knx(&args, &project, &products);
        assert!(output.status.success(), "{}", text(&output));
        assert!(
            text(&output).contains("support: verified"),
            "{}",
            text(&output)
        );
    }
}

include!("../../fixtures/inactive-selection/namespaces.rs");

#[test]
fn f3_namespace_real_cli_refuses_before_datagram() {
    for (case, xml) in namespace_cases(XML) {
        let dir = tempfile::tempdir().unwrap();
        let (project, products) = correction_inputs(dir.path(), &xml, false);
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_nonblocking(true).unwrap();
        let output = write(
            &["device", "download", TARGET],
            &project,
            &products,
            &peer.local_addr().unwrap().to_string(),
            dir.path(),
        );
        let shown = text(&output);
        assert!(
            !output.status.success()
                && shown.contains("inactive-write-selection: unresolved-ownership"),
            "{case}: {shown}"
        );
        for private in ["urn:synthetic", "e:Parameter", "M-FFF6", "A.xml"] {
            assert!(!shown.contains(private), "{shown}");
        }
        assert_eq!(
            peer.recv_from(&mut [0; 256]).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[test]
fn f3_namespace_real_cli_knx_prefix_positive_writes_exact_frames() {
    for scope in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (project, products) =
            correction_inputs(dir.path(), &namespace_positive(XML, scope), false);
        let device = device();
        let gateway = gateway(device.clone());
        let output = write(
            &["device", "download", TARGET],
            &project,
            &products,
            &gateway.address,
            dir.path(),
        );
        assert!(output.status.success(), "{}", text(&output));
        drop(gateway);
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
fn f3_namespace_genuine_old_database_reopens_app_offline_and_built_cli() {
    let original = PathBuf::from(
        std::env::var_os("KNX_NAMESPACE_PRE_DB").expect("retained producer database"),
    );
    let before = std::fs::read(&original).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let (project_path, products) = correction_inputs(dir.path(), XML, false);
    std::fs::copy(&original, &products).unwrap();
    let conn = knx_productdb::open_and_migrate(&products).unwrap();
    let store = knx_store::open_and_migrate(&project_path).unwrap();
    let project = knx_store::load_project(&store).unwrap();
    let reading = knx_app::device_download::prepare_device_download_for_reading(
        &conn,
        &project,
        TARGET.parse().unwrap(),
    )
    .unwrap();
    assert_eq!(
        reading
            .image
            .segment(&format!("{PROGRAM_ID}_AS-4400"))
            .unwrap()
            .octets
            .len(),
        6
    );
    let app =
        knx_app::device_download::prepare_device_download(&conn, &project, TARGET.parse().unwrap());
    assert!(
        matches!(
            app,
            Err(knx_app::device_download::PrepareError::Plan(
                knx_productdb::download_plan::DownloadPlanError::InactiveWriteSelection(
                    knx_productdb::download_plan::InactiveWriteSelectionError::UnresolvedOwnership
                )
            ))
        ),
        "{app:?}"
    );
    assert_eq!(
        knx_app::download_support::offline_plan(&conn, PROGRAM_ID)
            .unwrap_err()
            .0,
        knx_app::download_support::UnsupportedCategory::InactiveWriteSelection
    );
    drop(conn);
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_nonblocking(true).unwrap();
    let output = write(
        &["device", "download", TARGET],
        &project_path,
        &products,
        &peer.local_addr().unwrap().to_string(),
        dir.path(),
    );
    assert!(
        text(&output).contains("inactive-write-selection: unresolved-ownership"),
        "{}",
        text(&output)
    );
    assert_eq!(
        peer.recv_from(&mut [0; 256]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(std::fs::read(&original).unwrap(), before);
}

#[test]
fn f3_real_cli_opaque_dynamic_attributes_refuse_before_any_datagram() {
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
        let dir = tempfile::tempdir().unwrap();
        let (project, products) = correction_inputs(dir.path(), &xml, false);
        for partial in [
            None,
            Some("parameters"),
            Some("group-addresses"),
            Some("both"),
        ] {
            let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
            peer.set_nonblocking(true).unwrap();
            let mut args = vec!["device", "download", TARGET];
            if let Some(partial) = partial {
                args.extend(["--partial", partial]);
            }
            let output = write(
                &args,
                &project,
                &products,
                &peer.local_addr().unwrap().to_string(),
                dir.path(),
            );
            let shown = text(&output);
            assert!(
                shown.contains("inactive-write-selection: unresolved-ownership"),
                "{shown}"
            );
            assert!(!output.status.success());
            assert!(shown.contains("written to the device: no"));
            assert!(
                !shown.contains("Opaque") && !shown.contains(PROGRAM_ID),
                "{shown}"
            );
            assert_eq!(
                peer.recv_from(&mut [0; 2048]).unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            assert!(!dir.path().join("backups").exists());
        }
    }
}
