//! `knx device restore` refuses a backup whose prior application the plan cannot vouch for.
//!
//! The refusal comes from restore derivation, before any connector opens. Synthetic product
//! data only; the "gateway" is a loopback UDP socket owned by the test.

use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use knx_app::device_backup::{write_backup, StoredBackup};
use knx_core::commissioning::device_backup::{
    written_regions, BackupRegion, DeviceBackup, PriorApplication,
};
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::memory_download::machines;
use knx_core::commissioning::partial_memory_download::application_identity;
use knx_testsupport::vendor_plugin::{program_xml, HARDWARE_XML, PROGRAM_ID, TARGET};

/// A product database with the plain program, and a backup file taken with `prior`.
fn backup_with(dir: &Path, prior: impl Fn([u8; 5]) -> PriorApplication) -> (PathBuf, PathBuf) {
    let products = dir.join("products.sqlite");
    let db = knx_productdb::open_and_migrate(&products).unwrap();
    knx_productdb::ingest_file(&db, "M-FFF6/A.xml", program_xml("").as_bytes()).unwrap();
    knx_productdb::ingest_file(&db, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    let plan = knx_app::download_support::offline_plan(&db, PROGRAM_ID).unwrap();
    let identity = application_identity(&plan).expect("the program names its application");
    let stored = StoredBackup {
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
            prior_application: prior(identity),
        },
        application: PROGRAM_ID.into(),
        partial: None,
        taken: "2026-10-25T12:00:00Z".into(),
        plan_steps: plan.steps.iter().map(ToString::to_string).collect(),
    };
    let file = write_backup(&dir.join("backups"), &stored).unwrap();
    (file, products)
}

fn restore(dir: &Path, file: &Path, products: &Path, gateway: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_knx"));
    command
        .args(["device", "restore"])
        .arg(file)
        .arg("--product-db")
        .arg(products);
    if let Some(gateway) = gateway {
        command
            .args(["--gateway", gateway])
            .arg("--activity-history")
            .arg(dir.join("history.sqlite"))
            .arg("--backup-dir")
            .arg(dir.join("pre-restore-backups"))
            .args(["--confirm", &format!("I confirm download to {TARGET}")])
            .args([
                "--accept-untested",
                &format!("I accept an untested download to {TARGET}"),
            ]);
    }
    command.stdin(Stdio::null()).output().unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn the_plan_of_a_backup_from_the_same_application_is_shown() {
    let dir = tempfile::tempdir().unwrap();
    let (file, products) = backup_with(dir.path(), PriorApplication::Recorded);
    let output = restore(dir.path(), &file, &products, None);
    let shown = text(&output);
    assert!(output.status.success(), "{shown}");
    assert!(shown.contains("plan only"), "{shown}");
}

#[test]
fn a_backup_of_another_or_no_recorded_application_is_refused_before_any_connector() {
    for (name, prior, wanted) in [
        (
            "other",
            (|identity: [u8; 5]| {
                let mut other = identity;
                other[4] ^= 1;
                PriorApplication::Recorded(other)
            }) as fn([u8; 5]) -> PriorApplication,
            "was taken from application",
        ),
        (
            "unrecorded",
            |_| PriorApplication::NotRecorded,
            "does not record which application",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (file, products) = backup_with(dir.path(), prior);
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        let gateway = peer.local_addr().unwrap().to_string();

        let plan_only = text(&restore(dir.path(), &file, &products, None));
        assert!(plan_only.contains(wanted), "{name}: {plan_only}");
        assert!(
            !plan_only.contains("steps:"),
            "{name}: no plan shown: {plan_only}"
        );

        let output = restore(dir.path(), &file, &products, Some(&gateway));
        let shown = text(&output);
        assert!(!output.status.success(), "{name}: {shown}");
        assert!(shown.contains("no restore of"), "{name}: {shown}");
        assert!(shown.contains(wanted), "{name}: {shown}");
        assert!(
            peer.recv_from(&mut [0; 2048]).is_err(),
            "{name}: the refused restore reached the connector: {shown}"
        );
    }
}
