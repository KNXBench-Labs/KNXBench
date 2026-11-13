//! Exercise read-only CLI comparison over TCP, with strict admission and UDP handshake controls.

use super::{
    probe::{
        collect,
        fixture::Fixture,
        peer::{gateway, no_retry, no_udp, Peer},
        Joined,
    },
    stderr, stdout,
};
use knx_core::commissioning::memory_download::MemoryDownloadStep;
use knx_net::ApplicationService as A;
use std::{
    net::{Ipv4Addr, TcpListener, UdpSocket},
    process::{Command, Output},
    thread,
};

const SKIPPED: &str = "warning: 2 TCP frame(s) skipped before disconnecting (unknown service or larger than the local receive limit); their content was not kept";
// Independent literal image: U16s 300/258 (high octet first), octet 7, zero and text "ab".
const MEMORY: [u8; 32] = [
    1, 0x2c, 1, 2, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, b'a', b'b', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0,
];

/// Admit the actual saved inputs before any peer, and pin the plan independently of the reader.
fn fixture() -> Fixture {
    let fixture = Fixture::comparison();
    let project = knx_store::open_existing_read_only(&fixture.project).unwrap();
    let saved = knx_store::load_project(&project.conn).unwrap();
    let products = knx_productdb::open_read_only(&fixture.products).unwrap();
    let prepared = knx_app::device_download::prepare_device_download_for_reading(
        &products.conn,
        &saved,
        "1.1.67".parse().unwrap(),
    )
    .unwrap();
    assert_eq!(prepared.plan.mask.0, 0x0701);
    assert_eq!(prepared.plan.manufacturer, 2);
    assert_eq!(
        knx_core::commissioning::partial_memory_download::application_identity(&prepared.plan),
        Some([0, 2, 0, 1, 1])
    );
    let writes: Vec<_> = prepared
        .plan
        .steps
        .iter()
        .filter_map(|step| match step {
            MemoryDownloadStep::WriteMemory { address, octets } => Some((*address, octets.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(writes, [(0x4400, MEMORY.to_vec())], "{:#?}", prepared.plan);
    drop(products);
    drop(project);
    fixture
}

fn command(fixture: &Fixture, addr: &str, transport: Option<&str>) -> Command {
    let mut cmd = fixture.bare_command();
    cmd.args(["device", "compare", "1.1.67", "--project"])
        .arg(&fixture.project)
        .arg("--product-db")
        .arg(&fixture.products)
        .args(["--gateway", addr]);
    if let Some(transport) = transport {
        cmd.args(["--tunnel-transport", transport]);
    }
    cmd
}

/// DD0, manufacturer, application identity, APDU limit, then the plan's sole load state.
fn prelude(peer: &mut Peer) {
    peer.begin(false);
    peer.descriptor_request();
    peer.descriptor_answer(vec![7, 1]);
    peer.property_request(1, 12);
    peer.property_answer(1, 12, 1, vec![0, 2]);
    peer.request(
        2,
        A::PropertyValueRead {
            object_index: 3,
            property_id: 13,
            nr_of_elem: 1,
            start_index: 1,
        },
    );
    peer.answer(
        2,
        A::PropertyValueResponse {
            object_index: 3,
            property_id: 13,
            nr_of_elem: 1,
            start_index: 1,
            data: vec![0, 2, 0, 1, 1],
        },
    );
    peer.property_request(3, 56);
    peer.property_answer(3, 56, 1, vec![0, 15]);
}

fn identity(peer: &mut Peer) {
    prelude(peer);
    peer.request(
        4,
        A::MemoryRead {
            address: 0xb6ec,
            number: 1,
        },
    );
    peer.answer(
        4,
        A::MemoryResponse {
            address: 0xb6ec,
            data: vec![1],
        },
    );
}

/// Exact standard-frame chunks; any extra service (including any write) fails the peer.
fn complete(peer: &mut Peer, different: bool, skips: bool) {
    identity(peer);
    memory_and_finish(peer, different, skips, 5);
}

fn memory_and_finish(peer: &mut Peer, different: bool, skips: bool, first_seq: u8) {
    let mut bytes = MEMORY;
    if different {
        bytes[0] = 0x99;
        bytes[31] = 0x55;
    }
    for (seq, address, from, to) in [
        (first_seq, 0x4400, 0, 12),
        (first_seq + 1, 0x440c, 12, 24),
        (first_seq + 2, 0x4418, 24, 32),
    ] {
        peer.request(
            seq,
            A::MemoryRead {
                address,
                number: (to - from) as u8,
            },
        );
        if skips && seq == first_seq {
            peer.skips();
        }
        peer.answer(
            seq,
            A::MemoryResponse {
                address,
                data: bytes[from..to].to_vec(),
            },
        );
    }
    peer.management_end();
    peer.finish();
}

fn exit(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        stderr(output)
    );
}

fn connected(output: &Output, address: &str) {
    assert!(
        stderr(output).contains(&format!(
            "connected to {address} over TCP, assigned individual address 1.1.250"
        )),
        "{}",
        stderr(output)
    );
}

fn failed(output: &Output) {
    exit(output, 1);
    let out = stdout(output);
    assert!(!out.contains("the device holds all"), "{out}");
    assert!(!out.contains("a download would change"), "{out}");
    assert!(
        !out.contains("written to the device: no (read only)"),
        "{out}"
    );
}

fn history(fixture: &Fixture, state: &str) {
    let reader = knx_app::commissioning_activity::OneShotLog::for_run(
        fixture.dir.path().join("history.sqlite"),
    )
    .unwrap();
    let (rows, more) = reader.history_page(0, 100).unwrap();
    assert!(!more);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].activity.kind, "deviceCompare");
    assert_eq!(rows[0].activity.address.as_deref(), Some("1.1.67"));
    assert_eq!(rows[0].activity.state, state);
    assert!(rows[0].activity.finished_at.is_some());
    assert!(!rows[0].interrupted);
}

#[test]
fn tcp_same_reads_the_literal_plan_and_closes_both_sessions_with_zero_skips_silent() {
    let fixture = fixture();
    let (addr, worker, trap) = gateway(250, |peer| complete(peer, false, false));
    let output = collect(&mut command(&fixture, &addr.to_string(), Some("tcp")));
    worker.join().unwrap();
    exit(&output, 0);
    let out = stdout(&output);
    assert!(
        out.contains("the device holds all 32 octets a download would write"),
        "{out}"
    );
    assert!(out.contains("application program: Loaded"), "{out}");
    assert!(
        out.contains("written to the device: no (read only)"),
        "{out}"
    );
    connected(&output, &addr.to_string());
    assert!(!stderr(&output).contains("skipped"));
    no_retry(&trap);
    fixture.intact();
    assert_eq!(
        std::fs::read_dir(fixture.dir.path()).unwrap().count(),
        1,
        "no recording/history by default"
    );
}

#[test]
fn tcp_partial_parameters_retains_the_selected_read_only_recipe() {
    let fixture = fixture();
    // Partial selection drops the allocation record, so no load-state read remains.
    let (addr, worker, trap) = gateway(250, |peer| {
        prelude(peer);
        memory_and_finish(peer, false, false, 4);
    });
    let output = collect(
        command(&fixture, &addr.to_string(), Some("tcp")).args(["--partial", "parameters"]),
    );
    worker.join().unwrap();
    exit(&output, 0);
    assert!(stdout(&output).contains("the device holds all 32 octets"));
    no_retry(&trap);
    fixture.intact();
}

#[test]
fn tcp_difference_exits_two_with_literal_deltas_and_finished_history() {
    let fixture = fixture();
    let (addr, worker, trap) = gateway(250, |peer| complete(peer, true, false));
    let output = collect(
        command(&fixture, &addr.to_string(), Some("tcp"))
            .arg("--activity-history")
            .arg(fixture.dir.path().join("history.sqlite")),
    );
    worker.join().unwrap();
    exit(&output, 2);
    let out = stdout(&output);
    assert!(
        out.contains("a download would change 2 of 32 octets, in 2 runs:"),
        "{out}"
    );
    assert!(
        out.contains("4400h+1 (PAR): device 99 -> project 01"),
        "{out}"
    );
    assert!(
        out.contains("441Fh+1 (PAR): device 55 -> project 00"),
        "{out}"
    );
    assert!(
        out.contains("written to the device: no (read only)"),
        "{out}"
    );
    connected(&output, &addr.to_string());
    history(&fixture, "finished");
    no_retry(&trap);
    fixture.intact();
}

#[test]
fn tcp_skips_then_valid_reply_are_reported_once_without_recording() {
    let fixture = fixture();
    let (addr, worker, trap) = gateway(250, |peer| complete(peer, false, true));
    let output = collect(&mut command(&fixture, &addr.to_string(), Some("tcp")));
    worker.join().unwrap();
    exit(&output, 0);
    assert!(stdout(&output).contains("the device holds all 32 octets"));
    assert_eq!(stderr(&output).matches(SKIPPED).count(), 1);
    assert_eq!(stderr(&output).matches("skipped").count(), 1);
    no_retry(&trap);
    fixture.intact();
    assert_eq!(
        std::fs::read_dir(fixture.dir.path()).unwrap().count(),
        1,
        "no recording/history by default"
    );
}

#[test]
fn tcp_skips_then_eof_fail_without_success_output_and_keep_failed_history() {
    let fixture = fixture();
    let (addr, worker, trap) = gateway(250, |peer| {
        identity(peer);
        peer.request(
            5,
            A::MemoryRead {
                address: 0x4400,
                number: 12,
            },
        );
        peer.skips();
        peer.close_write();
    });
    let output = collect(
        command(&fixture, &addr.to_string(), Some("tcp"))
            .arg("--activity-history")
            .arg(fixture.dir.path().join("history.sqlite")),
    );
    worker.join().unwrap();
    failed(&output);
    assert!(
        stdout(&output).contains("not compared:"),
        "{}",
        stdout(&output)
    );
    assert_eq!(stderr(&output).matches(SKIPPED).count(), 1);
    assert_eq!(stderr(&output).matches("skipped").count(), 1);
    connected(&output, &addr.to_string());
    history(&fixture, "failed");
    no_retry(&trap);
    fixture.intact();
}

#[test]
fn tcp_refused_memory_read_is_not_a_success_and_settles_without_replay() {
    let fixture = fixture();
    let (addr, worker, trap) = gateway(250, |peer| {
        identity(peer);
        peer.request(
            5,
            A::MemoryRead {
                address: 0x4400,
                number: 12,
            },
        );
        peer.answer(
            5,
            A::MemoryResponse {
                address: 0x4400,
                data: vec![],
            },
        );
        peer.management_end();
        peer.finish();
    });
    let output = collect(&mut command(&fixture, &addr.to_string(), Some("tcp")));
    worker.join().unwrap();
    failed(&output);
    assert!(stdout(&output).contains("not compared:"));
    assert!(!stderr(&output).contains("skipped"));
    no_retry(&trap);
    fixture.intact();
}

#[test]
fn refused_tcp_uses_a_reserved_port_and_never_falls_back_to_udp() {
    let fixture = fixture();
    let reserved = tokio::net::TcpSocket::new_v4().unwrap();
    reserved.bind((Ipv4Addr::LOCALHOST, 0).into()).unwrap();
    let addr = reserved.local_addr().unwrap();
    let udp = UdpSocket::bind(addr).unwrap();
    udp.set_nonblocking(true).unwrap();
    let output = collect(&mut command(&fixture, &addr.to_string(), Some("tcp")));
    failed(&output);
    assert!(stderr(&output).contains("could not connect"));
    assert!(stdout(&output).is_empty());
    no_udp(&udp);
    fixture.intact();
    drop(reserved);
}

#[test]
fn default_and_explicit_udp_select_udp_handshake_only_not_a_complete_comparison() {
    for transport in [None, Some("udp")] {
        let fixture = fixture();
        let udp = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = udp.local_addr().unwrap();
        udp.set_read_timeout(Some(super::WAIT)).unwrap();
        let tcp = TcpListener::bind(addr).unwrap();
        tcp.set_nonblocking(true).unwrap();
        let socket = udp.try_clone().unwrap();
        let worker = Joined::new(thread::spawn(move || {
            let mut bytes = [0; 2048];
            let (n, from) = socket.recv_from(&mut bytes).unwrap();
            assert_eq!(n, 26);
            assert_eq!(&bytes[..6], [6, 0x10, 2, 5, 0, 26]);
            assert_eq!(&bytes[6..8], [8, 1]);
            assert_eq!(&bytes[14..16], [8, 1]);
            assert_eq!(&bytes[22..26], [4, 4, 2, 0]);
            socket
                .send_to(&[6, 0x10, 2, 6, 0, 8, 0, 0x24], from)
                .unwrap();
        }));
        let output = collect(&mut command(&fixture, &addr.to_string(), transport));
        worker.join().unwrap();
        failed(&output);
        assert!(stderr(&output).contains("could not connect"));
        assert!(stdout(&output).is_empty());
        assert!(!stderr(&output).contains("skipped"));
        assert_eq!(
            tcp.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        udp.set_nonblocking(true).unwrap();
        no_udp(&udp);
        fixture.intact();
    }
}

#[test]
fn malformed_transport_refuses_before_project_product_history_io_or_contact() {
    let fixture = fixture();
    let tcp = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let addr = tcp.local_addr().unwrap();
    tcp.set_nonblocking(true).unwrap();
    let udp = UdpSocket::bind(addr).unwrap();
    udp.set_nonblocking(true).unwrap();
    for (flags, expected) in [
        (vec!["--tunnel-transport"], "needs a value"),
        (vec!["--tunnel-transport", "--partial"], "needs a value"),
        (vec!["--tunnel-transport", ""], "must be udp or tcp"),
        (vec!["--tunnel-transport", "TCP"], "must be udp or tcp"),
        (vec!["--tunnel-transport", "tls"], "must be udp or tcp"),
        (
            vec!["--tunnel-transport", "udp", "--tunnel-transport", "tcp"],
            "duplicate --tunnel-transport",
        ),
        (
            vec!["--tunnel-transport", "tcp", "--tunnel-transport", "tcp"],
            "duplicate --tunnel-transport",
        ),
    ] {
        let history = fixture.dir.path().join("refused.sqlite");
        let products = fixture.dir.path().join("absent-products.sqlite");
        let output = collect(
            fixture
                .bare_command()
                .args(["device", "compare", "1.1.67", "--project"])
                .arg(fixture.dir.path().join("absent.knxdb"))
                .arg("--product-db")
                .arg(&products)
                .arg("--activity-history")
                .arg(&history)
                .args(["--gateway", &addr.to_string()])
                .args(flags),
        );
        failed(&output);
        assert!(
            stderr(&output).lines().next().unwrap().contains(expected),
            "{}",
            stderr(&output)
        );
        assert!(!stderr(&output).contains("project not found"));
        assert!(!history.exists());
        assert!(!products.exists());
        assert_eq!(
            tcp.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        no_udp(&udp);
        fixture.intact();
    }
}

#[test]
fn compare_help_names_the_source_build_option_default_and_no_fallback() {
    let fixture = fixture();
    let output = collect(fixture.bare_command().args(["device", "compare"]));
    exit(&output, 1);
    let error = stderr(&output);
    let help = error
        .split("knx device compare")
        .nth(1)
        .unwrap()
        .split("knx device system2-validation")
        .next()
        .unwrap();
    for expected in [
        "--tunnel-transport <udp|tcp>",
        "source builds",
        "defaults to udp",
        "never falls back to udp",
        "Exit 0:",
        "2: it differs",
        "1: not compared",
    ] {
        assert!(help.contains(expected), "missing {expected}: {help}");
    }
    fixture.intact();
}

#[test]
fn unrelated_download_restore_and_service_control_still_refuse_the_transport_option() {
    let fixture = fixture();
    for subcommand in ["download", "restore", "service-control"] {
        let output = collect(fixture.bare_command().args([
            "device",
            subcommand,
            "--tunnel-transport",
            "tcp",
        ]));
        failed(&output);
        assert!(
            stderr(&output)
                .lines()
                .next()
                .unwrap()
                .contains("unknown flag --tunnel-transport"),
            "{subcommand}: {}",
            stderr(&output)
        );
        fixture.intact();
    }
}
