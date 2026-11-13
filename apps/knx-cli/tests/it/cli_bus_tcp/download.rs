//! Exercise ordinary CLI downloads over TCP with synthetic saved inputs and a modeled device.

use super::{
    probe::{
        collect,
        fixture::Fixture,
        peer::{gateway, no_retry},
    },
    service_control::{connected, device, exit, history, rejected, udp_controls},
    stderr, stdout,
};
use knx_net::{commissioning::simulator::SimulatedDevice, ApplicationService as A};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

pub(super) const MEMORY: [u8; 32] = [
    1, 0x2c, 1, 2, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, b'a', b'b', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0,
];
pub(super) const OLD: [u8; 32] = [0x55; 32];
pub(super) fn fixture() -> Fixture {
    let f = Fixture::download();
    let project = knx_store::open_existing_read_only(&f.project).unwrap();
    let products = knx_productdb::open_read_only(&f.products).unwrap();
    let prepared = knx_app::device_download::prepare_device_download(
        &products.conn,
        &knx_store::load_project(&project.conn).unwrap(),
        "1.1.67".parse().unwrap(),
    )
    .unwrap();
    eprintln!("consumer-plan {:#?}", prepared.plan);
    let writes: Vec<_> = prepared
        .plan
        .steps
        .iter()
        .filter_map(|s| match s {
            knx_core::commissioning::memory_download::MemoryDownloadStep::WriteMemory {
                address,
                octets,
            } => Some((*address, octets.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(writes, [(0x4400, MEMORY.to_vec())]);
    f
}
pub(super) fn simulated(bytes: &[u8]) -> Arc<SimulatedDevice> {
    let d = device();
    d.preset_memory(0x4400, bytes);
    d.preset_property(3, 13, &[0, 2, 0, 1, 1]);
    d
}
pub(super) fn command(f: &Fixture, addr: &str, transport: Option<&str>) -> Command {
    let mut cmd = f.bare_command();
    cmd.args(["device", "download", "1.1.67", "--project"])
        .arg(&f.project)
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
pub(super) fn backup(f: &Fixture, expected: &[u8]) -> PathBuf {
    let files: Vec<_> = std::fs::read_dir(f.dir.path().join("backups"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    let stored = knx_app::device_backup::read_backup(&files[0]).unwrap();
    assert_eq!(stored.backup.target.to_string(), "1.1.67");
    assert_eq!(stored.backup.mask.0, 0x0701);
    assert_eq!(stored.backup.manufacturer, 2);
    assert_eq!(stored.application, "M-0002_A-0001-01-0000");
    assert_eq!(stored.backup.regions.len(), 1);
    assert_eq!(stored.backup.regions[0].address, 0x4400);
    assert_eq!(stored.backup.regions[0].octets, expected);
    files[0].clone()
}
pub(super) fn backup_before_write(path: &Path, frame: &knx_net::LDataFrame) {
    if matches!(
        frame.service,
        A::MemoryWrite { .. } | A::PropertyValueWrite { property_id: 5, .. }
    ) {
        assert_eq!(
            std::fs::read_dir(path).unwrap().count(),
            1,
            "backup must precede payload/load writes"
        );
    }
}
pub(super) fn completed(f: &Fixture, d: &SimulatedDevice, expected: &[u8]) {
    for seen in d.seen() {
        match seen {
            knx_net::commissioning::simulator::Seen::MemoryWrite { address, .. } => {
                assert!(
                    address == 0x0104 || (0x4400..0x4420).contains(&address),
                    "unexpected write at {address:04x}"
                );
            }
            knx_net::commissioning::simulator::Seen::PropertyWrite {
                object_index,
                property_id,
                data,
            } => {
                assert_eq!(
                    (object_index, property_id, data),
                    (0, 14, vec![0x24]),
                    "only the existing Verify Mode setup is allowed"
                );
            }
            knx_net::commissioning::simulator::Seen::Restart { .. } => {
                panic!("no restart in this synthetic plan")
            }
            _ => {}
        }
    }
    assert_eq!(
        d.memory(0x4400, expected.len()),
        expected.iter().copied().map(Some).collect::<Vec<_>>()
    );
    let writes: Vec<_> = d
        .seen()
        .into_iter()
        .filter_map(|s| match s {
            knx_net::commissioning::simulator::Seen::MemoryWrite { address, data, .. }
                if (0x4400..0x4420).contains(&address) =>
            {
                Some((address, data))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        writes,
        [
            (0x4400, expected[..12].to_vec()),
            (0x440c, expected[12..24].to_vec()),
            (0x4418, expected[24..].to_vec())
        ]
    );
    assert_eq!(d.authorize_requests(), 0);
    let row = history(f, "deviceDownload", "finished");
    assert_eq!(row["writeEvidence"]["backupRecorded"], true);
    assert_eq!(row["writeEvidence"]["sendPossible"], true);
    assert_eq!(row["downloadEvidence"]["written"], "yes");
    assert_eq!(row["downloadEvidence"]["cleanup"], "returnedOk");
    let controls: Vec<_> = d
        .seen()
        .into_iter()
        .filter_map(|seen| match seen {
            knx_net::commissioning::simulator::Seen::MemoryWrite {
                address: 0x0104,
                data,
                ..
            } => Some(data),
            _ => None,
        })
        .collect();
    assert_eq!(
        controls,
        [
            vec![0x34, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0x31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0x33, 0, 0, 0x44, 0, 0, 0x20, 0xff, 3, 0x80, 0],
            vec![0x32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ]
    );
    assert_eq!(
        d.load_state(knx_core::commissioning::properties::ObjectIndex::APPLICATION_PROGRAM),
        knx_core::commissioning::load_state::LoadState::Loaded
    );
}
#[test]
fn tcp_download_completes_exact_bytes_history_backup_and_cleanup() {
    for skips in [false, true] {
        let f = fixture();
        let d = simulated(&OLD);
        let worker_device = Arc::clone(&d);
        let path = f.dir.path().join("backups");
        let (addr, worker, trap) = gateway(250, move |peer| {
            peer.simulate(&worker_device, skips, |frame| {
                backup_before_write(&path, frame)
            })
        });
        let output = collect(&mut command(&f, &addr.to_string(), Some("tcp")));
        worker.join().unwrap();
        exit(&output, 0);
        connected(&output, &addr.to_string(), skips);
        assert!(
            stdout(&output).contains("written to the device: yes"),
            "{}",
            stdout(&output)
        );
        completed(&f, &d, &MEMORY);
        backup(&f, &OLD);
        no_retry(&trap);
        f.intact();
    }
}
#[test]
fn tcp_eof_before_backup_is_failed_with_skips_and_no_replay() {
    let f = fixture();
    let (addr, worker, trap) = gateway(250, |peer| {
        peer.begin(false);
        peer.descriptor_request();
        peer.skips();
        peer.close_write();
    });
    let output = collect(&mut command(&f, &addr.to_string(), Some("tcp")));
    worker.join().unwrap();
    exit(&output, 1);
    connected(&output, &addr.to_string(), true);
    assert!(!f.dir.path().join("backups").exists());
    assert!(stdout(&output).contains("written to the device: no"));
    let row = history(&f, "deviceDownload", "failed");
    assert_eq!(row["downloadEvidence"]["written"], "no");
    no_retry(&trap);
    f.intact();
}
#[test]
fn tcp_rejection_never_falls_back_and_udp_controls_are_handshake_only() {
    let f = fixture();
    rejected(|addr| command(&f, addr, Some("tcp")));
    // Each admitted invocation needs an independent history lifecycle.
    let f = fixture();
    udp_controls(|addr, transport| command(&f, addr, transport));
    f.intact();
}
#[test]
fn admission_refuses_invalid_plan_authorisation_and_excluded_target_before_io() {
    let f = fixture();
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let missing = f.dir.path().join("missing.knxdb");
    for (extra, expected) in [
        (vec!["--tunnel-transport"], "needs a value"),
        (vec!["--tunnel-transport", "TLS"], "must be udp or tcp"),
        (
            vec!["--tunnel-transport", "tcp", "--tunnel-transport", "tcp"],
            "duplicate",
        ),
        (vec!["--tunnel-transport", "tcp"], "plan only"),
        (
            vec!["--tunnel-transport", "tcp", "--confirm", "wrong"],
            "must read exactly",
        ),
    ] {
        let out = collect(
            f.bare_command()
                .args(["device", "download", "1.1.67", "--project"])
                .arg(&missing)
                .args(["--gateway", &addr])
                .args(extra),
        );
        exit(&out, 1);
        assert!(stderr(&out).contains(expected), "{}", stderr(&out));
        assert!(!stderr(&out).contains("project not found"));
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
                .args(["device", "download", "1.1.67", "--project"])
                .arg(&missing)
                .args(["--gateway", gateway, "--tunnel-transport", "tcp"]),
        );
        exit(&out, 1);
        assert!(stderr(&out).contains("--gateway"));
        assert!(!stderr(&out).contains("project not found"));
    }
    let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
    let out = collect(
        f.bare_command()
            .args(["device", "download", &excluded, "--project"])
            .arg(&missing)
            .args([
                "--gateway",
                &addr,
                "--tunnel-transport",
                "tcp",
                "--confirm",
                "wrong",
            ]),
    );
    exit(&out, 1);
    assert!(stderr(&out).contains("exclusion list"));
    let mut no_ack = command(&f, &addr, Some("tcp"));
    // Build a separate invocation without the acknowledgement, not a last-wins override.
    let args: Vec<_> = no_ack.get_args().map(|s| s.to_os_string()).collect();
    no_ack = f.bare_command();
    let index = args.iter().position(|s| s == "--accept-untested").unwrap();
    no_ack.args(&args[..index]).args(&args[index + 2..]);
    let out = collect(&mut no_ack);
    exit(&out, 1);
    assert!(stderr(&out).contains("--accept-untested"));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(!missing.exists());
    f.intact();
}
