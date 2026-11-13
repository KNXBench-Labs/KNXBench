//! Exercise CLI TCP restore using an admitted synthetic backup and exact byte/history oracles.

use super::{
    download::{self, backup_before_write, completed, simulated, MEMORY, OLD},
    probe::{
        collect,
        fixture::Fixture,
        peer::{gateway, no_retry},
    },
    service_control::{connected, exit, history, rejected, udp_controls},
    stderr, stdout,
};
use std::{path::PathBuf, process::Command, sync::Arc};

fn fixture() -> (Fixture, PathBuf) {
    use knx_core::commissioning::{
        device_backup::{BackupRegion, DeviceBackup, ExemplarEvidence, PriorApplication},
        load_state::LoadState,
        memory_download::{machines, source_procedure_descriptions, ExemplarSerial},
    };
    let f = download::fixture();
    let db = knx_productdb::open_read_only(&f.products).unwrap();
    let project = knx_store::open_existing_read_only(&f.project).unwrap();
    let prepared = knx_app::device_download::prepare_device_download(
        &db.conn,
        &knx_store::load_project(&project.conn).unwrap(),
        "1.1.67".parse().unwrap(),
    )
    .unwrap();
    let stored = knx_app::device_backup::StoredBackup {
        write_selection: prepared.write_selection,
        backup: DeviceBackup {
            target: "1.1.67".parse().unwrap(),
            mask: prepared.plan.mask,
            manufacturer: 2,
            load_states: machines(&prepared.plan)
                .into_iter()
                .map(|m| (m, LoadState::Loaded))
                .collect(),
            regions: vec![BackupRegion {
                address: 0x4400,
                octets: OLD.to_vec(),
            }],
            prior_application: PriorApplication::Recorded([0, 2, 0, 1, 1]),
            exemplar_serial: ExemplarEvidence::Recorded(ExemplarSerial::new([0, 2, 3, 4, 5, 6])),
        },
        application: prepared.request.program_id.clone(),
        partial: None,
        taken: "2026-11-13T12:00:00Z".into(),
        plan_steps: source_procedure_descriptions(&prepared.plan),
    };
    let path =
        knx_app::device_backup::write_backup(&f.dir.path().join("restore-input"), &stored).unwrap();
    let reopened = knx_app::device_backup::read_backup(&path).unwrap();
    assert_eq!(reopened, stored);
    let plan = knx_app::device_backup::prepare_restore(&db.conn, &reopened).unwrap();
    let writes: Vec<_> = plan
        .steps
        .iter()
        .filter_map(|step| match step {
            knx_core::commissioning::memory_download::MemoryDownloadStep::WriteMemory {
                address,
                octets,
            } => Some((*address, octets.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(writes, [(0x4400, OLD.to_vec())]);
    (f, path)
}
fn command(f: &Fixture, input: &std::path::Path, addr: &str, transport: Option<&str>) -> Command {
    let mut cmd = f.bare_command();
    cmd.args(["device", "restore"])
        .arg(input)
        .arg("--product-db")
        .arg(&f.products)
        .args(["--gateway", addr]);
    if let Some(t) = transport {
        cmd.args(["--tunnel-transport", t]);
    }
    cmd.args([
        "--confirm",
        &knx_core::commissioning::mutation::required_confirmation_phrase(
            "1.1.67".parse().unwrap(),
            knx_core::WriteScope::Download,
        ),
        "--accept-untested",
        &knx_app::download_support::untested_acknowledgement("1.1.67".parse().unwrap()),
    ])
    .arg("--activity-history")
    .arg(f.dir.path().join("history.sqlite"))
    .arg("--backup-dir")
    .arg(f.dir.path().join("backups"));
    cmd
}
#[test]
fn tcp_restore_completes_exact_backup_bytes_and_records_recovery_before_writes() {
    for skips in [false, true] {
        let (f, input) = fixture();
        let original = std::fs::read(&input).unwrap();
        let d = simulated(&MEMORY);
        d.preset_property(0, 11, &[0, 2, 3, 4, 5, 6]);
        let other = Arc::clone(&d);
        let backups = f.dir.path().join("backups");
        let (addr, worker, trap) = gateway(250, move |peer| {
            peer.simulate(&other, skips, |frame| backup_before_write(&backups, frame))
        });
        let output = collect(&mut command(&f, &input, &addr.to_string(), Some("tcp")));
        worker.join().unwrap();
        exit(&output, 0);
        connected(&output, &addr.to_string(), skips);
        completed(&f, &d, &OLD);
        download::backup(&f, &MEMORY);
        no_retry(&trap);
        assert_eq!(std::fs::read(&input).unwrap(), original);
        f.intact();
    }
}
#[test]
fn tcp_restore_eof_reports_failure_skips_and_no_replay() {
    let (f, input) = fixture();
    let (addr, worker, trap) = gateway(250, |peer| {
        peer.begin(false);
        peer.descriptor_request();
        peer.skips();
        peer.close_write();
    });
    let output = collect(&mut command(&f, &input, &addr.to_string(), Some("tcp")));
    worker.join().unwrap();
    exit(&output, 1);
    connected(&output, &addr.to_string(), true);
    assert!(stdout(&output).contains("written to the device: no"));
    assert!(!f.dir.path().join("backups").exists());
    let row = history(&f, "deviceDownload", "failed");
    assert_eq!(row["downloadEvidence"]["written"], "no");
    no_retry(&trap);
    f.intact();
}
#[test]
fn tcp_restore_rejection_never_falls_back_and_udp_controls_are_handshake_only() {
    let (f, input) = fixture();
    rejected(|addr| command(&f, &input, addr, Some("tcp")));
    let (f, input) = fixture();
    udp_controls(|addr, t| command(&f, &input, addr, t));
    f.intact();
}
#[test]
fn restore_admission_refuses_transport_plan_gateway_phrase_history_and_excluded_backup() {
    let (f, input) = fixture();
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let missing = f.dir.path().join("missing-backup");
    let history_path = f.dir.path().join("never-history");
    for (extra, expected) in [
        (vec!["--tunnel-transport"], "needs a value"),
        (vec!["--tunnel-transport", "TCP"], "must be udp or tcp"),
        (
            vec!["--tunnel-transport", "udp", "--tunnel-transport", "tcp"],
            "duplicate",
        ),
        (vec!["--tunnel-transport", "tcp"], "plan only"),
    ] {
        let out = collect(
            f.bare_command()
                .args(["device", "restore"])
                .arg(&missing)
                .args(["--gateway", &addr])
                .args(extra)
                .arg("--activity-history")
                .arg(&history_path),
        );
        exit(&out, 1);
        assert!(stderr(&out).contains(expected), "{}", stderr(&out));
        assert!(!stderr(&out).contains("backup not found"));
        assert!(!history_path.exists());
    }
    for gateway in [
        "bad",
        "0.0.0.0:3671",
        "224.0.23.12:3671",
        "255.255.255.255:3671",
        "127.0.0.1:0",
        "[::1]:3671",
    ] {
        let out = collect(
            f.bare_command()
                .args(["device", "restore"])
                .arg(&missing)
                .args(["--gateway", gateway, "--tunnel-transport", "tcp"]),
        );
        exit(&out, 1);
        assert!(stderr(&out).contains("--gateway"));
        assert!(!stderr(&out).contains("backup not found"));
    }
    let mut cmd = command(&f, &input, &addr, Some("tcp"));
    let mut args: Vec<_> = cmd.get_args().map(|a| a.to_os_string()).collect();
    let i = args.iter().position(|a| a == "--confirm").unwrap();
    args[i + 1] = "wrong".into();
    cmd = f.bare_command();
    cmd.args(&args);
    let out = collect(&mut cmd);
    exit(&out, 1);
    assert!(stderr(&out).contains("must read exactly"));
    let mut excluded: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    excluded["device"] = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0]
        .to_string()
        .into();
    let refused = f.dir.path().join("excluded.json");
    std::fs::write(&refused, serde_json::to_vec(&excluded).unwrap()).unwrap();
    let out = collect(&mut command(&f, &refused, &addr, Some("tcp")));
    exit(&out, 1);
    assert!(stderr(&out).contains("exclusion list"), "{}", stderr(&out));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(!f.dir.path().join("backups").exists());
    f.intact();
}
