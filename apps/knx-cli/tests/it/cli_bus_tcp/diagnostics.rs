//! Run the read-only diagnostics export through the built CLI over explicit TCP and UDP.

use super::{
    probe::{
        collect,
        peer::{gateway, no_retry, no_udp, Peer},
    },
    stderr, stdout,
};
use knx_core::{
    CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId, Language, Project,
    SourceRef, Topology,
};
use std::{
    net::{Ipv4Addr, TcpListener, UdpSocket},
    path::PathBuf,
    process::{Command, Output},
    thread,
    time::Duration,
};

const SKIPPED: &str = "warning: 2 TCP frame(s) skipped before disconnecting (unknown service or larger than the local receive limit); their content was not kept";

/// A saved project whose device 1 is at 1.1.67, with its own runtime and home directories.
struct Fixture {
    dir: tempfile::TempDir,
    project: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut project = Project::new(Language("en".into()));
        project.installations.push(Installation {
            id: InstallationId(0),
            name: "Synthetic".into(),
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
            name: "Synthetic".into(),
            description: None,
            address: Some("1.1.67".parse().unwrap()),
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        let path = dir.path().join("project.knxdb");
        let conn = knx_store::open_and_migrate(&path).unwrap();
        knx_store::save_project(&conn, &project).unwrap();
        for sub in ["runtime", "home"] {
            std::fs::create_dir(dir.path().join(sub)).unwrap();
        }
        Self { dir, project: path }
    }

    fn output(&self) -> PathBuf {
        self.dir.path().join("diagnostic.zip")
    }

    /// `knx device diagnostics` for device 1 via `gateway`; `extra` adds options.
    fn command(&self, gateway: &str, extra: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_knx"));
        cmd.env("XDG_RUNTIME_DIR", self.dir.path().join("runtime"))
            .env("HOME", self.dir.path().join("home"))
            .args(["device", "diagnostics", "--project"])
            .arg(&self.project)
            .args(["--device-id", "1", "--gateway", gateway, "--output"])
            .arg(self.output())
            .args(extra);
        cmd
    }
}

/// The identity recipe's wire sequence: descriptor, PID12, PID78, then both disconnects.
fn identity(peer: &mut Peer, skips: bool) {
    peer.begin(false);
    peer.descriptor_request();
    if skips {
        peer.skips();
    }
    peer.descriptor_answer(vec![0x00, 0x25]);
    peer.property_request(1, 12);
    peer.property_answer(1, 12, 1, vec![0x00, 0x01]);
    peer.property_request(2, 78);
    peer.property_answer(2, 78, 1, vec![0, 1, 0, 0, 0, 0x5b]);
    peer.management_end();
    peer.finish();
}

fn succeeded(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}\n{}",
        stdout(output),
        stderr(output)
    );
    assert!(
        stderr(output).contains("connected to 127.0.0.1:"),
        "{}",
        stderr(output)
    );
    assert!(
        stderr(output).contains(" over TCP, assigned individual address 1.1.250"),
        "{}",
        stderr(output)
    );
}

fn exported(fixture: &Fixture) -> knx_app::device_diagnostics::Report {
    let report =
        knx_app::device_diagnostics::read_bundle(&std::fs::read(fixture.output()).unwrap())
            .unwrap();
    assert!(!report.simulated);
    assert!(!report.written);
    assert_eq!(report.memory_status, "not-requested");
    report
}

#[test]
fn tcp_identity_export_reads_all_three_items_and_ends_both_sessions() {
    let fixture = Fixture::new();
    let (addr, worker, trap) = gateway(250, |peer| identity(peer, false));
    let output = collect(&mut fixture.command(
        &addr.to_string(),
        &["--tunnel-transport", "tcp", "--confirm-read"],
    ));
    worker.join().unwrap();
    succeeded(&output);
    assert!(stdout(&output).contains(&format!("via {addr} over TCP")));
    assert!(!stderr(&output).contains("skipped"), "{}", stderr(&output));
    let report = exported(&fixture);
    assert_eq!(report.items[0].bytes, [0x00, 0x25]);
    assert_eq!(report.items[1].bytes, [0x00, 0x01]);
    assert_eq!(report.items[2].bytes, [0, 1, 0, 0, 0, 0x5b]);
    assert_eq!(report.stop_reason, None);
    no_retry(&trap);
}

#[test]
fn tcp_export_discloses_skipped_frames_once_and_still_completes() {
    let fixture = Fixture::new();
    let (addr, worker, trap) = gateway(250, |peer| identity(peer, true));
    let output = collect(&mut fixture.command(
        &addr.to_string(),
        &["--tunnel-transport", "tcp", "--confirm-read"],
    ));
    worker.join().unwrap();
    succeeded(&output);
    assert_eq!(stderr(&output).matches(SKIPPED).count(), 1);
    let report = exported(&fixture);
    assert_eq!(report.items[2].bytes, [0, 1, 0, 0, 0, 0x5b]);
    assert_eq!(report.stop_reason, None);
    no_retry(&trap);
}

#[test]
fn tcp_peer_eof_mid_read_keeps_the_partial_export_and_the_skip_count() {
    let fixture = Fixture::new();
    let (addr, worker, trap) = gateway(250, |peer| {
        peer.begin(false);
        peer.descriptor_request();
        peer.skips();
        peer.close_write();
    });
    let output = collect(&mut fixture.command(
        &addr.to_string(),
        &["--tunnel-transport", "tcp", "--confirm-read"],
    ));
    worker.join().unwrap();
    assert_eq!(stderr(&output).matches(SKIPPED).count(), 1);
    // Exit 0 confirms the saved report, not a complete or successful device read.
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains("Saved and re-read verified"));
    // The export records what stopped the read instead of inventing an answer.
    let report = exported(&fixture);
    assert!(report.complete);
    assert_eq!(report.items[0].status, "error");
    assert!(report.stop_reason.is_some(), "{report:?}");
    assert!(report.items.iter().all(|item| item.bytes.is_empty()));
    no_retry(&trap);
}

#[test]
fn refused_tcp_fails_without_a_file_and_never_falls_back_to_udp() {
    let fixture = Fixture::new();
    // Keep TCP bound but not listening until the child exits: no other test can
    // take the port between selection and the refused connect.
    let reserved = tokio::net::TcpSocket::new_v4().unwrap();
    reserved.bind((Ipv4Addr::LOCALHOST, 0).into()).unwrap();
    let address = reserved.local_addr().unwrap();
    let trap = UdpSocket::bind(address).unwrap();
    trap.set_nonblocking(true).unwrap();
    let gateway = address.to_string();
    let output =
        collect(&mut fixture.command(&gateway, &["--tunnel-transport", "tcp", "--confirm-read"]));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("gateway refused/unavailable"),
        "{}",
        stderr(&output)
    );
    assert!(!fixture.output().exists());
    no_udp(&trap);
}

#[test]
fn default_and_explicit_udp_still_open_a_udp_tunnel() {
    for extra in [vec![], vec!["--tunnel-transport", "udp"]] {
        let fixture = Fixture::new();
        let gateway = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        gateway
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let address = gateway.local_addr().unwrap();
        // Nothing listens for TCP on this port, so a TCP dial would be refused.
        let script = thread::spawn(move || {
            let mut buffer = [0u8; 128];
            let (length, client) = gateway.recv_from(&mut buffer).unwrap();
            let frame = &buffer[..length];
            assert_eq!(&frame[2..4], [0x02, 0x05], "CONNECT_REQUEST");
            assert_eq!(frame[7], 0x01, "control HPAI names UDP");
            assert_eq!(frame[15], 0x01, "data HPAI names UDP");
            // Refuse (E_NO_MORE_CONNECTIONS) so the command ends at once.
            gateway
                .send_to(&[0x06, 0x10, 0x02, 0x06, 0x00, 0x08, 0x00, 0x24], client)
                .unwrap();
        });
        let mut args = extra.clone();
        args.push("--confirm-read");
        let output = collect(&mut fixture.command(&address.to_string(), &args));
        script.join().expect("the UDP script failed");
        assert_eq!(output.status.code(), Some(1), "{extra:?}");
        assert!(
            stderr(&output).contains("gateway refused/unavailable"),
            "{extra:?}: {}",
            stderr(&output)
        );
        assert!(!fixture.output().exists(), "{extra:?}");
    }
}

#[test]
fn previews_name_only_an_explicit_tcp_and_contact_nothing() {
    for (extra, via) in [
        (vec![], ""),
        (vec!["--tunnel-transport", "udp"], ""),
        (vec!["--tunnel-transport", "tcp"], " over TCP"),
    ] {
        let fixture = Fixture::new();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let udp = UdpSocket::bind(address).unwrap();
        udp.set_nonblocking(true).unwrap();
        let output = collect(&mut fixture.command(&address.to_string(), &extra));
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        let out = stdout(&output);
        assert!(
            out.contains(&format!("via {address}{via}. Read only")),
            "{out}"
        );
        assert!(out.contains("Preview only"), "{out}");
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        no_udp(&udp);
        assert!(!fixture.output().exists());
    }
}
