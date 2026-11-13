//! Exercise service-control reads and authorised writes through the public TCP CLI.

use super::{
    probe::{
        collect,
        fixture::Fixture,
        peer::{gateway, no_retry, no_udp},
    },
    stderr, stdout,
};
use knx_net::{
    commissioning::simulator::{SimulatedDevice, SimulatorConfig},
    ApplicationService as A,
};
use std::{
    net::{Ipv4Addr, TcpListener, UdpSocket},
    process::{Command, Output},
    sync::Arc,
    thread,
};

pub(super) const SKIPPED: &str = "warning: 2 TCP frame(s) skipped before disconnecting (unknown service or larger than the local receive limit); their content was not kept";

pub(super) fn exit(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        stderr(output)
    );
}
pub(super) fn connected(output: &Output, addr: &str, skipped: bool) {
    let err = stderr(output);
    assert!(
        err.contains(&format!(
            "connected to {addr} over TCP, assigned individual address 1.1.250"
        )),
        "{err}"
    );
    assert_eq!(
        err.lines().filter(|line| *line == SKIPPED).count(),
        usize::from(skipped),
        "{err}"
    );
    assert_eq!(
        err.matches("TCP frame(s) skipped").count(),
        usize::from(skipped),
        "{err}"
    );
}
pub(super) fn device() -> Arc<SimulatedDevice> {
    let device = Arc::new(SimulatedDevice::with_config_at(
        "1.1.67".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0701,
            ..Default::default()
        },
    ));
    device.preset_property(0, 12, &[0, 2]);
    device.preset_property(0, 8, &[0x12, 0x30]);
    device.preset_property(0, 14, &[0x20]);
    device
}
fn command(f: &Fixture, addr: &str, transport: Option<&str>, write: bool) -> Command {
    let mut cmd = f.bare_command();
    cmd.args(["device", "service-control", "1.1.67", "--gateway", addr]);
    if let Some(t) = transport {
        cmd.args(["--tunnel-transport", t]);
    }
    if write {
        cmd.args([
            "--enable",
            "--confirm",
            &knx_core::commissioning::mutation::required_confirmation_phrase(
                "1.1.67".parse().unwrap(),
                knx_core::WriteScope::IndividualAddressWriteEnable,
            ),
        ])
        .arg("--activity-history")
        .arg(f.dir.path().join("history.sqlite"))
        .arg("--backup-dir")
        .arg(f.dir.path().join("backups"));
    }
    cmd
}
pub(super) fn history(f: &Fixture, kind: &str, state: &str) -> serde_json::Value {
    let log =
        knx_app::commissioning_activity::OneShotLog::for_run(f.dir.path().join("history.sqlite"))
            .unwrap();
    let (rows, more) = log.history_page(0, 100).unwrap();
    assert!(!more);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].activity.kind, kind);
    assert_eq!(rows[0].activity.state, state);
    assert_eq!(rows[0].activity.address.as_deref(), Some("1.1.67"));
    assert!(rows[0].activity.finished_at.is_some());
    assert!(!rows[0].interrupted);
    let value = serde_json::to_value(&rows[0]).unwrap();
    eprintln!("consumer-history {value}");
    value
}

#[test]
fn tcp_read_and_confirmed_write_complete_with_backup_and_exact_control_bytes() {
    for write in [false, true] {
        for skips in [false, true] {
            let f = Fixture::new();
            let device = device();
            let simulated = Arc::clone(&device);
            let backups = f.dir.path().join("backups");
            let (addr, worker, trap) = gateway(250, move |peer| {
                peer.simulate(&simulated, skips, |frame| {
                    if let A::PropertyValueWrite {
                        nr_of_elem,
                        start_index,
                        ..
                    } = frame.service
                    {
                        assert_eq!((nr_of_elem, start_index), (1, 1));
                        assert_eq!(
                            std::fs::read_dir(&backups).unwrap().count(),
                            1,
                            "backup before any write, including Verify Mode"
                        );
                    }
                })
            });
            let output = collect(&mut command(&f, &addr.to_string(), Some("tcp"), write));
            worker.join().unwrap();
            exit(&output, 0);
            connected(&output, &addr.to_string(), skips);
            no_retry(&trap);
            let writes: Vec<_> = device
                .seen()
                .into_iter()
                .filter_map(|seen| match seen {
                    knx_net::commissioning::simulator::Seen::PropertyWrite {
                        object_index,
                        property_id,
                        data,
                    } => Some((object_index, property_id, data)),
                    _ => None,
                })
                .collect();
            if write {
                assert_eq!(writes, [(0, 14, vec![0x24]), (0, 8, vec![0x12, 0x34])]);
                assert!(
                    stdout(&output).contains("written: yes"),
                    "{}",
                    stdout(&output)
                );
                let path = std::fs::read_dir(f.dir.path().join("backups"))
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path();
                let backup: knx_app::service_control_backup::ServiceControlBackup =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                assert_eq!(backup.octets, "1230");
                assert_eq!(backup.device_control_octets, "20");
                assert_eq!(backup.device, "1.1.67");
                assert_eq!(backup.mask, "0701");
                history(&f, "serviceControlWrite", "verified");
            } else {
                assert!(writes.is_empty());
                assert!(stdout(&output).contains("PID_SERVICE_CONTROL = 1230h"));
                assert!(!f.dir.path().join("backups").exists());
            }
            assert_eq!(device.authorize_requests(), 0);
            assert!(!device.memory_was_written());
            f.intact();
        }
    }
}

#[test]
fn tcp_eof_reports_skips_once_and_never_retries() {
    for write in [false, true] {
        let f = Fixture::new();
        let (addr, worker, trap) = gateway(250, |peer| {
            peer.begin(false);
            peer.descriptor_request();
            peer.skips();
            peer.close_write();
        });
        let output = collect(&mut command(&f, &addr.to_string(), Some("tcp"), write));
        worker.join().unwrap();
        exit(&output, 1);
        connected(&output, &addr.to_string(), true);
        assert!(stdout(&output).contains("FAILED"), "{}", stdout(&output));
        no_retry(&trap);
        assert!(!f.dir.path().join("backups").exists());
        f.intact();
    }
}

/// Hold a bound TCP socket, not a released ephemeral port, through the fallback check.
pub(super) fn rejected(mut cmd: impl FnMut(&str) -> Command) {
    let socket = tokio::net::TcpSocket::new_v4().unwrap();
    socket.bind((Ipv4Addr::LOCALHOST, 0).into()).unwrap();
    let addr = socket.local_addr().unwrap();
    let udp = UdpSocket::bind(addr).unwrap();
    udp.set_nonblocking(true).unwrap();
    let output = collect(&mut cmd(&addr.to_string()));
    exit(&output, 1);
    assert!(
        stderr(&output).contains("could not connect"),
        "{}",
        stderr(&output)
    );
    no_udp(&udp);
    drop(socket);
}
pub(super) fn udp_controls(mut cmd: impl FnMut(&str, Option<&str>) -> Command) {
    for transport in [None, Some("udp")] {
        let udp = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        udp.set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let addr = udp.local_addr().unwrap();
        let tcp = TcpListener::bind(addr).unwrap();
        tcp.set_nonblocking(true).unwrap();
        let worker = thread::spawn(move || {
            let mut bytes = [0; 2048];
            let (n, client) = udp.recv_from(&mut bytes).unwrap();
            assert_eq!(n, 26);
            assert_eq!(&bytes[..4], [6, 0x10, 2, 5]);
            assert_eq!(bytes[7], 1, "UDP HPAI");
            udp.send_to(&[6, 0x10, 2, 6, 0, 8, 0, 0x24], client)
                .unwrap();
        });
        let output = collect(&mut cmd(&addr.to_string(), transport));
        worker.join().unwrap();
        exit(&output, 1);
        assert!(stderr(&output).contains("could not connect"));
        assert!(!stderr(&output).contains("TCP frame(s) skipped"));
        assert_eq!(
            tcp.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}
#[test]
fn rejection_has_no_udp_fallback_and_udp_controls_are_handshake_only() {
    let f = Fixture::new();
    rejected(|addr| command(&f, addr, Some("tcp"), false));
    udp_controls(|addr, transport| command(&f, addr, transport, false));
    f.intact();
}

#[test]
fn admission_refuses_invalid_transport_gateway_plan_phrase_and_exclusion_before_io() {
    let f = Fixture::new();
    let tcp = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    tcp.set_nonblocking(true).unwrap();
    let addr = tcp.local_addr().unwrap().to_string();
    let missing = f.dir.path().join("missing-key");
    let history = f.dir.path().join("never-history");
    let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
    let mut cases = vec![
        (vec!["--tunnel-transport"], "needs a value"),
        (vec!["--tunnel-transport", "tls"], "must be udp or tcp"),
        (
            vec!["--tunnel-transport", "tcp", "--tunnel-transport", "udp"],
            "duplicate",
        ),
        (vec!["--enable", "--tunnel-transport", "tcp"], "plan only"),
        (
            vec![
                "--enable",
                "--confirm",
                "wrong",
                "--tunnel-transport",
                "tcp",
            ],
            "must read exactly",
        ),
    ];
    for gateway in [
        "0.0.0.0:3671",
        "224.0.23.12:3671",
        "255.255.255.255:3671",
        "127.0.0.1:0",
        "[::1]:3671",
        "bad",
    ] {
        cases.push((
            vec!["--gateway", gateway, "--tunnel-transport", "tcp"],
            "--gateway",
        ));
    }
    for (extra, expected) in cases {
        let output = collect(
            f.bare_command()
                .args(["device", "service-control", "1.1.67", "--gateway", &addr])
                .args(extra)
                .arg("--key-file")
                .arg(&missing)
                .arg("--activity-history")
                .arg(&history),
        );
        exit(&output, 1);
        assert!(stderr(&output).contains(expected), "{}", stderr(&output));
        assert!(!stderr(&output).contains("--key-file") || stderr(&output).contains("usage:"));
        assert!(!history.exists());
    }
    let output = collect(f.bare_command().args([
        "device",
        "service-control",
        &excluded,
        "--gateway",
        &addr,
        "--tunnel-transport",
        "tcp",
    ]));
    exit(&output, 1);
    assert!(stderr(&output).contains("exclusion list"));
    assert_eq!(
        tcp.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    f.intact();
}
