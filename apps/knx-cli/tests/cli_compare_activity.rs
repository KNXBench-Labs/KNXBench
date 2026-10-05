//! Optional compare history: fail closed before input migration or a loopback adapter.
use knx_app::commissioning_activity::OneShotLog;
use std::net::UdpSocket;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn compare_usage_explains_optional_metadata_and_legacy_exit_codes() {
    let output = Command::new(env!("CARGO_BIN_EXE_knx")).output().unwrap();
    let usage = String::from_utf8(output.stderr).unwrap();
    let start = usage.find("knx device compare <").unwrap();
    let end = start + usage[start..].find("knx device restore <").unwrap();
    let section = &usage[start..end];
    assert!(
        section.contains("[--activity-history <path>]")
            && section.contains("optional")
            && section.contains("metadata, not recovery"),
        "compare usage hides its optional history boundary"
    );
    assert!(section.contains("Exit 0:") && section.contains("2: it differs; 1: not compared."));
}

#[test]
fn explicit_unavailable_compare_history_refuses_before_project_or_connector() {
    let dir = tempfile::tempdir().unwrap();
    let history = dir.path().join("foreign-history.bin");
    let project = dir.path().join("unopened-project.knxdb");
    let products = dir.path().join("uncreated-products.sqlite");
    std::fs::write(&history, b"foreign bytes must remain intact").unwrap();
    std::fs::write(&project, b"project must not be opened or migrated").unwrap();
    let before_history = std::fs::read(&history).unwrap();
    let before_project = std::fs::read(&project).unwrap();
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_millis(150)))
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "compare", "1.1.67", "--project"])
        .arg(&project)
        .arg("--product-db")
        .arg(&products)
        .arg("--activity-history")
        .arg(&history)
        .arg("--gateway")
        .arg(peer.local_addr().unwrap().to_string())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.contains("activity history unavailable; comparison not started"),
        "explicit history was not admitted before project I/O: {error}"
    );
    assert_eq!(std::fs::read(&history).unwrap(), before_history);
    assert_eq!(std::fs::read(&project).unwrap(), before_project);
    assert!(!products.exists());
    assert!(matches!(
        peer.recv_from(&mut [0; 64]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
}

#[cfg(unix)]
#[test]
fn compare_refuses_all_primary_and_product_history_aliases_without_changing_rows() {
    for input_kind in ["primary", "products"] {
        for alias_kind in ["same", "symlink", "hardlink"] {
            let dir = tempfile::tempdir().unwrap();
            let history = dir.path().join("history.sqlite");
            let log = Arc::new(OneShotLog::for_run(history.clone()).unwrap());
            log.start("deviceCompare", Some("1.1.67".into()))
                .finish("finished");
            let before = std::fs::read(&history).unwrap();
            let before_rows = serde_json::to_value(log.history_page(0, 100).unwrap().0).unwrap();
            let input = if alias_kind == "same" {
                history.clone()
            } else {
                let input = dir.path().join("alias");
                if alias_kind == "symlink" {
                    std::os::unix::fs::symlink(&history, &input).unwrap();
                } else {
                    std::fs::hard_link(&history, &input).unwrap();
                }
                input
            };
            let primary = if input_kind == "primary" {
                input.clone()
            } else {
                let primary = dir.path().join("unopened-project");
                std::fs::write(&primary, b"project must remain unopened").unwrap();
                primary
            };
            let products = if input_kind == "products" {
                input.clone()
            } else {
                dir.path().join("uncreated-products")
            };
            let primary_bytes = std::fs::read(&primary).unwrap();
            let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
            peer.set_read_timeout(Some(Duration::from_millis(150)))
                .unwrap();
            let output = Command::new(env!("CARGO_BIN_EXE_knx"))
                .args(["device", "compare", "1.1.67", "--project"])
                .arg(&primary)
                .arg("--product-db")
                .arg(&products)
                .arg("--activity-history")
                .arg(&history)
                .arg("--gateway")
                .arg(peer.local_addr().unwrap().to_string())
                .output()
                .unwrap();
            assert!(!output.status.success());
            let error = String::from_utf8(output.stderr).unwrap();
            assert!(error.contains("activity history") && error.contains("separate files"),
                "{input_kind}/{alias_kind}: comparison did not refuse aliased inputs first: {error}");
            assert_eq!(std::fs::read(&history).unwrap(), before);
            assert_eq!(std::fs::read(&primary).unwrap(), primary_bytes);
            assert_eq!(
                serde_json::to_value(log.history_page(0, 100).unwrap().0).unwrap(),
                before_rows
            );
            if input_kind == "primary" {
                assert!(!products.exists());
            }
            assert!(matches!(
                peer.recv_from(&mut [0; 64]).unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ));
        }
    }
}

#[test]
fn missing_history_and_product_alias_is_refused_before_either_file_is_created() {
    missing_history_alias_case("same");
}

#[test]
#[cfg(unix)]
fn missing_history_parent_alias_is_refused_before_either_file_is_created() {
    for alias_kind in ["parent-traversal", "parent-symlink"] {
        missing_history_alias_case(alias_kind);
    }
}

#[test]
#[cfg(unix)]
fn missing_history_leaf_alias_is_refused_before_either_file_is_created() {
    for alias_kind in ["input-leaf-symlink", "history-leaf-symlink"] {
        missing_history_alias_case(alias_kind);
    }
}

fn missing_history_alias_case(alias_kind: &str) {
    let dir = tempfile::tempdir().unwrap();
    let (project, _) = comparison_inputs(dir.path());
    let input = dir.path().join("uncreated-shared-store.sqlite");
    let products = if alias_kind == "input-leaf-symlink" {
        #[cfg(unix)]
        {
            let link = dir.path().join("products-link.sqlite");
            std::os::unix::fs::symlink(&input, &link).unwrap();
            link
        }
        #[cfg(not(unix))]
        input.clone()
    } else {
        input.clone()
    };
    let history = match alias_kind {
        #[cfg(unix)]
        "history-leaf-symlink" => {
            let link = dir.path().join("history-link.sqlite");
            std::os::unix::fs::symlink(&input, &link).unwrap();
            link
        }
        "parent-traversal" => {
            std::fs::create_dir(dir.path().join("child")).unwrap();
            dir.path().join("child/../uncreated-shared-store.sqlite")
        }
        #[cfg(unix)]
        "parent-symlink" => {
            std::os::unix::fs::symlink(dir.path(), dir.path().join("parent-link")).unwrap();
            dir.path().join("parent-link/uncreated-shared-store.sqlite")
        }
        _ => input.clone(),
    };
    let before = std::fs::read(&project).unwrap();
    let project_connection = knx_store::open_and_migrate(&project).unwrap();
    let saved = knx_store::load_project(&project_connection).unwrap();
    assert_eq!(saved.devices.iter().count(), 1);
    drop(project_connection);
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_millis(150)))
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "compare", "1.1.67", "--project"])
        .arg(&project)
        .arg("--product-db")
        .arg(&products)
        .arg("--activity-history")
        .arg(&history)
        .arg("--gateway")
        .arg(peer.local_addr().unwrap().to_string())
        .output()
        .unwrap();
    assert!(
        !input.exists(),
        "shared missing history/product destination was created before alias refusal"
    );
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("separate files"));
    assert_eq!(std::fs::read(&project).unwrap(), before);
    assert_eq!(
        knx_store::load_project(&knx_store::open_and_migrate(&project).unwrap()).unwrap(),
        saved
    );
    assert!(matches!(
        peer.recv_from(&mut [0; 64]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
}

#[test]
fn a_refused_compare_connector_records_failed_not_uncertain() {
    use knx_net::core::services::{CONNECT_RESPONSE, E_CONNECTION_TYPE};
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = comparison_inputs(dir.path());
    let history = dir.path().join("history.sqlite");
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let gateway = peer.local_addr().unwrap().to_string();
    let responder = std::thread::spawn(move || {
        let (_, client) = peer.recv_from(&mut [0; 2048]).unwrap();
        peer.send_to(
            &knx_net::frame::encode_frame(CONNECT_RESPONSE, &[0, E_CONNECTION_TYPE]),
            client,
        )
        .unwrap();
    });
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "compare", "1.1.67", "--project"])
        .arg(&project)
        .arg("--product-db")
        .arg(&products)
        .arg("--activity-history")
        .arg(&history)
        .arg("--gateway")
        .arg(gateway)
        .output()
        .unwrap();
    responder.join().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("could not connect"));
    let reader = OneShotLog::for_run(history).unwrap();
    let (rows, more) = reader.history_page(0, 100).unwrap();
    assert!(!more);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].activity.kind, "deviceCompare");
    assert_eq!(rows[0].activity.state, "failed");
    assert!(rows[0].activity.finished_at.is_some());
    assert!(!rows[0].interrupted);
}

// Public synthetic program; shape follows the existing productdb parser fixtures.
fn comparison_inputs(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    use knx_core::{
        CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId, Language,
        Project, SourceRef, Topology,
    };
    let products = dir.join("products.sqlite");
    let db = knx_productdb::open_and_migrate(&products).unwrap();
    let program = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-01-0000" Name="Synthetic comparison" ApplicationNumber="1"
ApplicationVersion="1" PeiType="1" MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="AS-4000" Address="16384" Size="2"><Data>AQI=</Data></AbsoluteSegment></Code>
<LoadProcedures><LoadProcedure><LdCtrlConnect/><LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="16384" Size="2" Access="255" MemType="3" SegFlags="128"/>
<LdCtrlDisconnect/></LoadProcedure></LoadProcedures></Static><Dynamic/>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let hardware = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">
<Hardware><Hardware Id="H-1" Name="Synthetic" SerialNumber="synthetic" VersionNumber="1">
<Products><Product Id="M-0001_H-1_P-1"/></Products><Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="M-0001_A-0001-01-0000"/></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
    knx_productdb::ingest_file(&db, "M-0001/A.xml", program).unwrap();
    knx_productdb::ingest_file(&db, "M-0001/Hardware.xml", hardware).unwrap();
    let mut project = Project::new(Language("en".into()));
    project.info.name = "private fixture project name".into();
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
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "DEV-1".into(),
        },
        name: "private fixture device name".into(),
        description: None,
        address: Some("1.1.67".parse().unwrap()),
        product_ref: "M-0001_H-1_P-1".into(),
        program_ref: "H-1_HP-1".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let plan =
        knx_app::device_download::prepare_device_download(&db, &project, "1.1.67".parse().unwrap())
            .expect("synthetic fixture must prepare a real plan before testing the caller");
    assert_eq!(plan.plan.data_octets(), 2);
    let path = dir.join("project.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    (path, products)
}

#[test]
fn compare_journals_before_connector_and_preserves_interruption_without_payload() {
    use std::process::Stdio;
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = comparison_inputs(dir.path());
    let history = dir.path().join("history.sqlite");
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let gateway = peer.local_addr().unwrap().to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "compare", "1.1.67", "--project"])
        .arg(&project)
        .arg("--product-db")
        .arg(&products)
        .arg("--activity-history")
        .arg(&history)
        .arg("--gateway")
        .arg(&gateway)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let received = peer.recv_from(&mut [0; 2048]);
    let published = history.exists();
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    assert!(
        received.is_ok(),
        "comparison never reached its own loopback connector: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        published,
        "comparison was not durably started before the first adapter request"
    );
    let reader = OneShotLog::for_run(history.clone()).unwrap();
    let (rows, more) = reader.history_page(0, 100).unwrap();
    assert!(!more);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].activity.kind, "deviceCompare");
    assert_eq!(rows[0].activity.address.as_deref(), Some("1.1.67"));
    assert_eq!(rows[0].activity.state, "unknown");
    assert!(rows[0].interrupted);
    assert!(rows[0].activity.finished_at.is_none());
    let metadata = serde_json::to_string(&rows).unwrap();
    for private in [
        gateway.as_str(),
        history.to_str().unwrap(),
        project.to_str().unwrap(),
        products.to_str().unwrap(),
        "private fixture project name",
        "private fixture device name",
        "M-0001_A-0001-01-0000",
        "AQI=",
    ] {
        assert!(
            !metadata.contains(private),
            "private request data entered comparison metadata"
        );
    }
}
