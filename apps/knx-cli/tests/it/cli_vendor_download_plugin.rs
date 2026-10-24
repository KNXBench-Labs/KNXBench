//! KL-006 at the command line: a declared download plug-in refuses every download scope.
//!
//! The refusal comes before any connector, while `knx device compare` still
//! reads the device.
//!
//! Synthetic product data only (`knx_testsupport::vendor_plugin`); the "gateway" is a
//! loopback UDP socket owned by the test, so nothing leaves the machine.

use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use knx_testsupport::vendor_plugin::{
    declaration, program_xml, HARDWARE2PROGRAM_REF, HARDWARE_XML, PLUGIN_GUID as PLUGIN,
    PRODUCT_REF, PROGRAM_ID, TARGET,
};

/// `(project.knxdb, products.sqlite)` with one device at `1.1.7`.
fn inputs(dir: &Path, plugin: bool) -> (PathBuf, PathBuf) {
    use knx_core::{
        CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId, Language,
        Project, SourceRef, Topology,
    };
    let products = dir.join("products.sqlite");
    let db = knx_productdb::open_and_migrate(&products).unwrap();
    knx_productdb::ingest_file(
        &db,
        "M-FFF6/A.xml",
        program_xml(&if plugin { declaration() } else { String::new() }).as_bytes(),
    )
    .unwrap();
    knx_productdb::ingest_file(&db, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    let mut project = Project::new(Language("en".into()));
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
        name: "Synthetic actuator".into(),
        description: None,
        address: Some(TARGET.parse().unwrap()),
        product_ref: PRODUCT_REF.into(),
        program_ref: HARDWARE2PROGRAM_REF.into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let path = dir.join("project.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    (path, products)
}

fn stored_backup(dir: &Path) -> knx_app::device_backup::StoredBackup {
    use knx_core::commissioning::device_backup::{written_regions, BackupRegion, DeviceBackup};
    use knx_core::commissioning::load_state::LoadState;
    use knx_core::commissioning::memory_download::machines;

    let products = knx_productdb::open_and_migrate(&dir.join("backup-source.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/A.xml", program_xml("").as_bytes()).unwrap();
    let plan = knx_app::download_support::offline_plan(&products, PROGRAM_ID).unwrap();
    let stored = knx_app::device_backup::StoredBackup {
        backup: DeviceBackup {
            target: TARGET.parse().unwrap(),
            mask: plan.mask,
            manufacturer: plan.manufacturer,
            load_states: machines(&plan)
                .into_iter()
                .map(|machine| (machine, LoadState::Loaded))
                .collect(),
            regions: written_regions(&plan)
                .into_iter()
                .map(|(address, length)| BackupRegion {
                    address,
                    octets: vec![0xA5; length],
                })
                .collect(),
        },
        application: PROGRAM_ID.into(),
        partial: None,
        taken: "2026-10-24T12:00:00Z".into(),
        plan_steps: plan.steps.iter().map(ToString::to_string).collect(),
    };
    knx_app::device_backup::prepare_restore(&products, &stored)
        .expect("the synthetic backup fits the same program without the declaration");
    stored
}

fn knx(args: &[&str], project: &Path, products: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .arg("--project")
        .arg(project)
        .arg("--product-db")
        .arg(products)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_kl_006_detail(shown: &str) {
    assert!(shown.contains("KL-006"), "{shown}");
    assert!(shown.contains("ETS download plug-in"), "{shown}");
    assert!(shown.contains(PLUGIN), "{shown}");
    assert!(shown.contains("no download scope is offered"), "{shown}");
}

fn assert_kl_006(shown: &str) {
    assert!(shown.contains("vendor-download-plugin"), "{shown}");
    assert_kl_006_detail(shown);
}

#[test]
fn every_download_plan_scope_is_refused_and_readiness_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = inputs(dir.path(), true);
    for partial in [
        None,
        Some("parameters"),
        Some("group-addresses"),
        Some("both"),
    ] {
        let mut args = vec!["device", "download", TARGET];
        if let Some(partial) = partial {
            args.extend(["--partial", partial]);
        }
        let output = knx(&args, &project, &products);
        let shown = text(&output);
        assert!(!output.status.success(), "{partial:?}: {shown}");
        assert_kl_006(&shown);
        assert!(!shown.contains("untested"), "{partial:?}: {shown}");
    }
    let output = knx(&["device", "readiness"], &project, &products);
    let shown = text(&output);
    assert!(output.status.success(), "{shown}");
    assert!(shown.contains("unsupported"), "{shown}");
    assert_kl_006(&shown);
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["products", "coverage", "--product-db"])
        .arg(&products)
        .output()
        .unwrap();
    let shown = text(&output);
    assert!(output.status.success(), "{shown}");
    assert!(
        shown.contains("unsupported vendor-download-plugin: 1"),
        "{shown}"
    );
}

/// The field case: a group-address-only partial download of a plug-in
/// program, confirmed and acknowledged, must not reach the gateway at all.
#[test]
fn a_confirmed_group_address_download_never_reaches_the_connector() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = inputs(dir.path(), true);
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let gateway = peer.local_addr().unwrap().to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "download", TARGET, "--partial", "group-addresses"])
        .arg("--project")
        .arg(&project)
        .arg("--product-db")
        .arg(&products)
        .arg("--activity-history")
        .arg(dir.path().join("history.sqlite"))
        .arg("--backup-dir")
        .arg(dir.path().join("backups"))
        .args(["--gateway", &gateway])
        .args(["--confirm", &format!("I confirm download to {TARGET}")])
        .args([
            "--accept-untested",
            &format!("I accept an untested download to {TARGET}"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let received = peer.recv_from(&mut [0; 2048]);
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let shown = text(&output);
    assert!(
        received.is_err(),
        "a plug-in program's download reached the connector: {shown}"
    );
    assert!(!output.status.success(), "{shown}");
    assert_kl_006(&shown);
}

#[test]
fn a_confirmed_restore_is_refused_before_any_gateway_traffic() {
    let dir = tempfile::tempdir().unwrap();
    let (_, products) = inputs(dir.path(), true);
    let backup = knx_app::device_backup::write_backup(
        &dir.path().join("restore-input"),
        &stored_backup(dir.path()),
    )
    .unwrap();
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_millis(250)))
        .unwrap();
    let gateway = peer.local_addr().unwrap().to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "restore"])
        .arg(&backup)
        .arg("--product-db")
        .arg(&products)
        .args(["--gateway", &gateway])
        .arg("--activity-history")
        .arg(dir.path().join("history.sqlite"))
        .arg("--backup-dir")
        .arg(dir.path().join("pre-restore-backups"))
        .args(["--confirm", &format!("I confirm download to {TARGET}")])
        .args([
            "--accept-untested",
            &format!("I accept an untested download to {TARGET}"),
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let shown = text(&output);
    assert!(!output.status.success(), "{shown}");
    assert_kl_006_detail(&shown);
    assert!(
        peer.recv_from(&mut [0; 2048]).is_err(),
        "a plug-in program's restore reached the connector: {shown}"
    );
}

#[test]
fn compare_still_reads_a_plugin_program_and_plain_programs_still_plan() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = inputs(dir.path(), true);
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let gateway = peer.local_addr().unwrap().to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "compare", TARGET, "--project"])
        .arg(&project)
        .arg("--product-db")
        .arg(&products)
        .arg("--activity-history")
        .arg(dir.path().join("history.sqlite"))
        .args(["--gateway", &gateway])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let received = peer.recv_from(&mut [0; 2048]);
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    assert!(
        received.is_ok(),
        "read-only compare of a plug-in program never reached its loopback connector: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let dir = tempfile::tempdir().unwrap();
    let (project, products) = inputs(dir.path(), false);
    for partial in [None, Some("group-addresses")] {
        let mut args = vec!["device", "download", TARGET];
        if let Some(partial) = partial {
            args.extend(["--partial", partial]);
        }
        let output = knx(&args, &project, &products);
        let shown = text(&output);
        assert!(output.status.success(), "{partial:?}: {shown}");
        assert!(shown.contains("support: UNTESTED"), "{partial:?}: {shown}");
        assert!(!shown.contains("vendor-download-plugin"), "{shown}");
    }
}
