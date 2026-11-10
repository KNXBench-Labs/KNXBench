//! Exercise both read-only find-serial queries through the built CLI and loopback TCP peers.

use super::{
    knx, read_frame, refused_tcp_with_udp_trap, stderr, stdout, KillOnDrop, CHANNEL,
    DISCONNECT_RESPONSE, TCP_CONNECT_REQUEST, TCP_CONNECT_RESPONSE,
};
use knx_app::commissioning_activity::OneShotLog;
use knx_net::{cemi::decode_l_data, ApplicationService, Destination, LDataMessageKind, Tpci};
use serde_json::Value;
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket},
    path::Path,
    process::{Command, Output, Stdio},
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const SERIAL: &str = "0083:12345678";
const OCTETS: [u8; 6] = [0, 0x83, 0x12, 0x34, 0x56, 0x78];
const WAIT: Duration = Duration::from_secs(35);
type Wire = Vec<(bool, Vec<u8>)>;

fn command(gateway: &str, by_address: bool) -> Command {
    let mut cmd = knx();
    cmd.args(["device", "find-serial"]);
    if by_address {
        cmd.args(["--address", "1.1.67"]);
    } else {
        cmd.arg(SERIAL);
    }
    cmd.args(["--gateway", gateway]);
    cmd
}

fn collect(cmd: &mut Command) -> Output {
    let mut child = KillOnDrop(
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut out = child.0.stdout.take().unwrap();
    let mut err = child.0.stderr.take().unwrap();
    let stdout = thread::spawn(move || {
        let mut v = Vec::new();
        out.read_to_end(&mut v).unwrap();
        v
    });
    let stderr = thread::spawn(move || {
        let mut v = Vec::new();
        err.read_to_end(&mut v).unwrap();
        v
    });
    let deadline = Instant::now() + WAIT;
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "find-serial did not settle");
        thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: stdout.join().unwrap(),
        stderr: stderr.join().unwrap(),
    }
}

fn run_tcp(gateway: &str, by_address: bool, flags: &[&str]) -> Output {
    collect(
        command(gateway, by_address)
            .args(["--tunnel-transport", "tcp"])
            .args(flags),
    )
}

struct Peer {
    stream: TcpStream,
    wire: Wire,
}
impl Peer {
    fn read(&mut self) -> Vec<u8> {
        let frame = read_frame(&mut self.stream);
        self.wire.push((true, frame.clone()));
        frame
    }
    fn send(&mut self, frame: &[u8]) {
        self.stream.write_all(frame).unwrap();
        self.wire.push((false, frame.to_vec()));
    }
    fn finish(&mut self) {
        assert_eq!(
            self.read(),
            [6, 0x10, 2, 9, 0, 16, CHANNEL, 0, 8, 2, 0, 0, 0, 0, 0, 0]
        );
        self.send(&DISCONNECT_RESPONSE);
        self.eof();
    }
    fn eof(&mut self) {
        let mut tail = Vec::new();
        self.stream
            .read_to_end(&mut tail)
            .expect("bounded peer-observed EOF, not a timeout");
        assert!(
            tail.is_empty(),
            "unexpected bytes after terminal boundary: {tail:02x?}"
        );
    }
    fn close_write(&mut self) {
        self.stream.shutdown(Shutdown::Write).unwrap();
        self.eof();
    }
}

fn gateway(
    script: impl FnOnce(&mut Peer) + Send + 'static,
) -> (SocketAddr, JoinHandle<Wire>, UdpSocket) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let trap = UdpSocket::bind(address).unwrap();
    trap.set_nonblocking(true).unwrap();
    let worker = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + WAIT;
        let stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "CLI never contacted TCP peer");
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("accept: {e}"),
            }
        };
        stream.set_read_timeout(Some(WAIT)).unwrap();
        stream.set_write_timeout(Some(WAIT)).unwrap();
        let mut peer = Peer {
            stream,
            wire: Vec::new(),
        };
        assert_eq!(peer.read(), TCP_CONNECT_REQUEST);
        peer.send(&TCP_CONNECT_RESPONSE);
        script(&mut peer);
        peer.wire
    });
    (address, worker, trap)
}

fn no_udp(trap: &UdpSocket) {
    trap.set_nonblocking(true).unwrap();
    assert_eq!(
        trap.recv_from(&mut [0; 2048]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

fn assert_request(frame: &[u8], by_address: bool, tpci: Tpci) {
    assert_eq!(
        &frame[..4],
        [6, 0x10, 4, 0x20],
        "only TUNNELLING_REQUEST, never UDP ACK"
    );
    assert_eq!(&frame[6..8], [4, CHANNEL]);
    let cemi = decode_l_data(&frame[10..]).unwrap();
    assert_eq!(cemi.kind, LDataMessageKind::Request);
    assert_eq!(cemi.source.to_string(), "0.0.0");
    assert_eq!(cemi.transport, tpci);
    if by_address {
        assert_eq!(
            cemi.destination,
            Destination::Individual("1.1.67".parse().unwrap())
        );
        if matches!(tpci, Tpci::NumberedData { .. }) {
            // Exact PID_SERIAL_NUMBER read: no authorisation, key, write, memory or restart APDU.
            assert_eq!(&frame[19..], [0x43, 0xd5, 0, 11, 0x10, 1]);
        } else {
            assert_eq!(cemi.service, ApplicationService::NoApplicationPdu);
        }
    } else {
        assert_eq!(
            cemi.destination,
            Destination::Group(knx_core::GroupAddress::from_raw(0))
        );
        assert_eq!(
            cemi.service,
            ApplicationService::IndividualAddressSerialNumberRead {
                serial_number: OCTETS
            }
        );
        assert_eq!(&frame[19..], [3, 0xdc, 0, 0x83, 0x12, 0x34, 0x56, 0x78]);
    }
}

fn begin_query(peer: &mut Peer, by_address: bool) {
    if by_address {
        let mut connect = peer.read();
        assert_request(&connect, true, Tpci::Connect);
        // Positive L_Data.con is separate from TCP send acceptance.
        connect[10] = 0x2e;
        connect[14..16].copy_from_slice(&[0x11, 0xfa]);
        peer.send(&connect);
        assert_request(&peer.read(), true, Tpci::NumberedData { seq: 0 });
    } else {
        assert_request(&peer.read(), false, Tpci::UnnumberedData);
    }
}

fn indication(apdu: &[u8], by_address: bool) -> Vec<u8> {
    let mut frame = vec![
        6,
        0x10,
        4,
        0x20,
        0,
        0,
        4,
        CHANNEL,
        7,
        0,
        0x29,
        0,
        0xbc,
        if by_address { 0x60 } else { 0xe0 },
        0x11,
        0x43,
        if by_address { 0x11 } else { 0 },
        if by_address { 0xfa } else { 0 },
        (apdu.len() - 1) as u8,
    ];
    frame.extend_from_slice(apdu);
    let size = (frame.len() as u16).to_be_bytes();
    frame[4..6].copy_from_slice(&size);
    frame
}

fn answer(peer: &mut Peer, by_address: bool) {
    if by_address {
        peer.send(&indication(
            &[0x43, 0xd6, 0, 11, 0x10, 1, 0, 0x83, 0x12, 0x34, 0x56, 0x78],
            true,
        ));
        assert_request(&peer.read(), true, Tpci::Ack { seq: 0 });
        assert_request(&peer.read(), true, Tpci::Disconnect);
    } else {
        peer.send(&indication(
            &[3, 0xdd, 0, 0x83, 0x12, 0x34, 0x56, 0x78, 0, 0, 0, 0],
            false,
        ));
    }
}

fn assert_result(output: &Output, by_address: bool) {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert_eq!(
        stdout(output),
        if by_address {
            format!("1.1.67: {SERIAL}\n")
        } else {
            format!("{SERIAL}: 1.1.67\n")
        }
    );
    assert!(
        stderr(output).contains("over TCP, assigned individual address 1.1.250"),
        "{}",
        stderr(output)
    );
}

fn history(path: &Path, state: &str, interrupted: bool, gateway: &str) {
    let log = Arc::new(OneShotLog::for_run(path.to_path_buf()).unwrap());
    let (rows, more) = log.history_page(0, 100).unwrap();
    assert!(!more);
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.activity.kind, "serialLookup");
    assert_eq!(row.activity.state, state);
    assert_eq!(row.interrupted, interrupted);
    assert_eq!(row.activity.finished_at.is_none(), interrupted);
    assert!(row.activity.address.is_none());
    let text = serde_json::to_string(&rows).unwrap();
    for secret in [SERIAL, "1.1.67", gateway, path.to_str().unwrap()] {
        assert!(
            !text.contains(secret),
            "private query data in metadata history: {secret}"
        );
    }
}

#[test]
fn both_queries_require_matching_device_answers_and_observed_management_and_tcp_teardown() {
    for by_address in [false, true] {
        let (addr, worker, trap) = gateway(move |peer| {
            begin_query(peer, by_address);
            answer(peer, by_address);
            peer.finish();
        });
        let output = run_tcp(&addr.to_string(), by_address, &[]);
        worker.join().unwrap();
        assert_result(&output, by_address);
        assert!(
            !stderr(&output).contains("skipped"),
            "zero skips must not invent a warning"
        );
        no_udp(&trap);
    }
}

#[test]
fn ordinary_output_reports_skips_before_success_and_peer_eof_in_both_queries() {
    for by_address in [false, true] {
        for reply in [false, true] {
            let (addr, worker, trap) = gateway(move |peer| {
                begin_query(peer, by_address);
                peer.send(&[6, 0x10, 9, 0x99, 0, 10, 0xde, 0xad, 0xbe, 0xef]);
                let mut large = vec![6, 0x10, 4, 0x20, 5, 0xdc];
                large.resize(1500, 0xee);
                peer.send(&large);
                if reply {
                    answer(peer, by_address);
                    peer.finish();
                } else {
                    peer.close_write();
                }
            });
            let output = run_tcp(&addr.to_string(), by_address, &[]);
            worker.join().unwrap();
            assert_eq!(output.status.success(), reply, "{}", stderr(&output));
            let error = stderr(&output);
            assert!(error.contains("over TCP"), "{error}");
            assert!(
                error.contains("2 TCP frame(s) skipped before disconnecting"),
                "{error}"
            );
            assert!(error.contains("their content was not kept"), "{error}");
            if reply {
                assert_result(&output, by_address);
            } else {
                assert!(stdout(&output).is_empty());
                assert!(
                    error.contains("the connection was broken down during"),
                    "{error}"
                );
            }
            no_udp(&trap);
        }
    }
}

#[test]
fn refused_tcp_and_rejected_handshake_never_fall_back_or_create_an_answer() {
    for by_address in [false, true] {
        let (port, trap) = refused_tcp_with_udp_trap();
        let output = run_tcp(&format!("127.0.0.1:{port}"), by_address, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr(&output).contains("could not connect"));
        assert!(!stderr(&output).contains("over TCP"));
        assert!(stdout(&output).is_empty());
        no_udp(&trap);

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let trap = UdpSocket::bind(address).unwrap();
        let worker = thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + WAIT;
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "CLI never contacted TCP peer");
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("accept: {error}"),
                }
            };
            stream.set_read_timeout(Some(WAIT)).unwrap();
            assert_eq!(read_frame(&mut stream), TCP_CONNECT_REQUEST);
            stream.write_all(&[6, 0x10, 2, 6, 0, 8, 0, 0x24]).unwrap();
            let mut tail = Vec::new();
            stream.read_to_end(&mut tail).unwrap();
            assert!(tail.is_empty());
        });
        let output = run_tcp(&address.to_string(), by_address, &[]);
        worker.join().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr(&output).contains("could not connect"));
        assert!(stdout(&output).is_empty());
        no_udp(&trap);
    }
}

#[test]
fn unanswered_and_nonmatching_queries_preserve_result_history_and_bounded_eof() {
    for by_address in [false, true] {
        for nonmatching in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("history.sqlite");
            let (addr, worker, trap) = gateway(move |peer| {
                begin_query(peer, by_address);
                if by_address {
                    // Transport acknowledgement is not an application answer and prevents a retry.
                    peer.send(&indication(&[0xc2], true));
                    if nonmatching {
                        // Correct device/sequence, wrong property; it must be ACKed but not returned.
                        peer.send(&indication(
                            &[0x43, 0xd6, 0, 12, 0x10, 1, 0, 0x83, 0x12, 0x34, 0x56, 0x78],
                            true,
                        ));
                        assert_request(&peer.read(), true, Tpci::Ack { seq: 0 });
                    }
                    assert_request(&peer.read(), true, Tpci::Disconnect);
                } else if nonmatching {
                    peer.send(&indication(
                        &[3, 0xdd, 0, 0x83, 0x12, 0x34, 0x56, 0x79, 0, 0, 0, 0],
                        false,
                    ));
                }
                peer.finish();
            });
            let endpoint = addr.to_string();
            let output = run_tcp(
                &endpoint,
                by_address,
                &["--activity-history", path.to_str().unwrap()],
            );
            worker.join().unwrap();
            assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
            assert!(stderr(&output).contains("over TCP"));
            if by_address {
                assert!(stdout(&output).is_empty());
                assert!(stderr(&output).contains("no answer"), "{}", stderr(&output));
            } else {
                assert!(stdout(&output).starts_with("no answer (MP §2.4:"));
            }
            history(
                &path,
                if by_address { "failed" } else { "finished" },
                false,
                &endpoint,
            );
            no_udp(&trap);
        }
    }
}

#[test]
fn mid_read_eof_is_failed_history_not_no_match_or_send_success() {
    for by_address in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        let (addr, worker, trap) = gateway(move |peer| {
            begin_query(peer, by_address);
            peer.close_write();
        });
        let endpoint = addr.to_string();
        let output = run_tcp(
            &endpoint,
            by_address,
            &["--activity-history", path.to_str().unwrap()],
        );
        worker.join().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(stdout(&output).is_empty());
        assert!(
            stderr(&output).contains("the connection was broken down during"),
            "{}",
            stderr(&output)
        );
        history(&path, "failed", false, &endpoint);
        no_udp(&trap);
    }
}

#[test]
fn invalid_admission_refuses_before_tcp_udp_history_or_recording() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let trap = UdpSocket::bind(listener.local_addr().unwrap()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let history = dir.path().join("not-created.sqlite");
    let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
    let cases = vec![
        (vec![], "give a serial number"),
        (vec![SERIAL, "--address", "1.1.67"], "not both"),
        (vec!["0083:123"], "serial"),
        (vec!["--address", "1.1.300"], "invalid individual address"),
        (vec!["--address", &excluded], "excluded"),
        (vec![SERIAL, SERIAL], "exactly one serial number"),
        (vec![SERIAL, "--tunnel-transport"], "needs a value"),
        (
            vec![SERIAL, "--tunnel-transport", "TCP"],
            "must be udp or tcp",
        ),
        (
            vec![SERIAL, "--tunnel-transport", "tls"],
            "must be udp or tcp",
        ),
        (vec![SERIAL, "--tunnel-transport", ""], "must be udp or tcp"),
        (
            vec![
                SERIAL,
                "--tunnel-transport",
                "tcp",
                "--tunnel-transport",
                "udp",
            ],
            "duplicate",
        ),
        (
            vec![
                SERIAL,
                "--tunnel-transport",
                "tcp",
                "--tunnel-transport",
                "tcp",
            ],
            "duplicate",
        ),
        (vec![SERIAL, "--gateway", &endpoint], "duplicate --gateway"),
        (
            vec!["--address", "1.1.67", "--address", "1.1.68"],
            "duplicate --address",
        ),
        (vec![SERIAL, "--confirm", "yes"], "unknown flag --confirm"),
        (
            vec![SERIAL, "--key-file", "absent"],
            "unknown flag --key-file",
        ),
        (vec![SERIAL, "--serial", "absent"], "unknown flag --serial"),
        (vec![SERIAL, "--source", "1.1.1"], "unknown flag --source"),
        (vec![SERIAL, "--enable"], "unknown flag --enable"),
        (
            vec![SERIAL, "--activity-history", history.to_str().unwrap()],
            "only once",
        ),
    ];
    for (flags, message) in cases {
        let output = collect(
            knx()
                .args([
                    "device",
                    "find-serial",
                    "--gateway",
                    &endpoint,
                    "--activity-history",
                    history.to_str().unwrap(),
                ])
                .args(flags),
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(
            stderr(&output).contains(message),
            "{message}: {}",
            stderr(&output)
        );
        assert!(!history.exists());
    }
    for endpoint in [
        "192.0.2.1:0",
        "0.0.0.0:3671",
        "255.255.255.255:3671",
        "224.0.0.1:3671",
        "[::1]:3671",
        "localhost:3671",
        "192.0.2.1",
    ] {
        let output = run_tcp(
            endpoint,
            false,
            &["--activity-history", history.to_str().unwrap()],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr(&output).contains("--gateway"));
        assert!(!history.exists());
    }
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    no_udp(&trap);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn tcp_history_and_bundle_preflight_preserve_foreign_evidence_without_contact() {
    for by_address in [false, true] {
        for bundle_refusal in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let foreign = dir.path().join("foreign-evidence");
            std::fs::write(&foreign, b"preserve me").unwrap();
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            listener.set_nonblocking(true).unwrap();
            let endpoint = listener.local_addr().unwrap().to_string();
            let trap = UdpSocket::bind(listener.local_addr().unwrap()).unwrap();
            let output = run_tcp(
                &endpoint,
                by_address,
                &[
                    if bundle_refusal {
                        "--diagnostic-bundle"
                    } else {
                        "--activity-history"
                    },
                    foreign.to_str().unwrap(),
                ],
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(stderr(&output).contains(if bundle_refusal {
                "already exists"
            } else {
                "lookup not started"
            }));
            assert_eq!(std::fs::read(&foreign).unwrap(), b"preserve me");
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            no_udp(&trap);
        }
    }
}

#[test]
fn omitted_and_explicit_udp_keep_the_real_udp_handshake_for_both_queries() {
    for by_address in [false, true] {
        for flags in [vec![], vec!["--tunnel-transport", "udp"]] {
            let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            socket.set_read_timeout(Some(WAIT)).unwrap();
            let endpoint = socket.local_addr().unwrap();
            let tcp = TcpListener::bind(endpoint).unwrap();
            tcp.set_nonblocking(true).unwrap();
            let worker = thread::spawn(move || {
                let mut buf = [0; 2048];
                let (size, client) = socket.recv_from(&mut buf).unwrap();
                assert_eq!(size, 26);
                assert_eq!(&buf[..4], [6, 0x10, 2, 5]);
                assert_eq!(buf[7], 1);
                assert_eq!(buf[15], 1);
                socket
                    .send_to(&[6, 0x10, 2, 6, 0, 8, 0, 0x24], client)
                    .unwrap();
            });
            let output = collect(command(&endpoint.to_string(), by_address).args(flags));
            worker.join().unwrap();
            assert_eq!(output.status.code(), Some(1));
            assert!(stderr(&output).contains("could not connect"));
            assert!(!stderr(&output).contains("skipped"));
            assert_eq!(
                tcp.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
    }
}

#[test]
fn tcp_bundle_retains_actual_route_back_wire_redacts_keys_and_cleans_temporary_recording() {
    for by_address in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let bundle = out.join("report.zip");
        let path = dir.path().join("history.sqlite");
        let (addr, worker, trap) = gateway(move |peer| {
            begin_query(peer, by_address);
            // Unrelated inbound authorisation request challenges existing capture redaction.
            peer.send(&indication(&[3, 0xd1, 0, 0x11, 0x22, 0x33, 0x44], false));
            answer(peer, by_address);
            peer.finish();
        });
        let endpoint = addr.to_string();
        let output = run_tcp(
            &endpoint,
            by_address,
            &[
                "--diagnostic-bundle",
                bundle.to_str().unwrap(),
                "--activity-history",
                path.to_str().unwrap(),
            ],
        );
        let wire = worker.join().unwrap();
        assert_result(&output, by_address);
        history(&path, "finished", false, &endpoint);
        assert!(stderr(&output)
            .contains("may contain device configuration and passwords: keep it private"));
        let left: Vec<_> = std::fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, ["report.zip"]);
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&bundle).unwrap()).unwrap();
        let mut manifest = String::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        let manifest: Value = serde_json::from_str(&manifest).unwrap();
        assert_eq!(manifest["problemType"], "deviceReading");
        let sections = manifest["sections"].as_array().unwrap();
        let operations: Vec<_> = sections
            .iter()
            .filter(|s| s["kind"] == "operation")
            .collect();
        assert_eq!(operations.len(), 1);
        assert_eq!(
            operations[0]["notes"],
            serde_json::json!([knx_app::diagnostic_bundle::PRIVATE_RAW_NOTE])
        );
        let id = operations[0]["label"].as_str().unwrap();
        let mut text = String::new();
        zip.by_name(&format!("operations/{id}/recording.jsonl"))
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        let lines: Vec<Value> = text
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(lines[0]["operation_kind"], "serial-number-address");
        assert_eq!(lines[0]["gateway"], endpoint);
        assert_eq!(lines.last().unwrap()["outcome"], "succeeded");
        assert_eq!(lines.last().unwrap()["records_dropped"], 0);
        assert!(lines
            .iter()
            .any(|l| l["event"] == "connected" && l["channel"] == CHANNEL));
        assert!(
            lines.iter().any(|l| !l["redacted"].is_null()),
            "inbound key was not marked redacted"
        );
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        assert!(!text.contains("11223344"));
        for (direction, sent) in [("sent", true), ("received", false)] {
            let recorded: Vec<_> = lines
                .iter()
                .filter(|l| l["type"] == "datagram" && l["dir"] == direction)
                .map(|l| l["raw"].as_str().unwrap().to_string())
                .collect();
            let expected: Vec<_> = wire
                .iter()
                .filter(|(from, _)| *from == sent)
                .map(|(_, frame)| {
                    let mut frame = frame.clone();
                    if frame.ends_with(&[0x11, 0x22, 0x33, 0x44]) {
                        let n = frame.len();
                        frame[n - 4..].fill(0);
                    }
                    hex(&frame)
                })
                .collect();
            assert_eq!(
                recorded, expected,
                "{direction}: exact retained TCP frames, not UDP or reconstructed events"
            );
        }
        no_udp(&trap);
    }
}

#[test]
fn failed_bundle_classifies_eof_and_no_match_without_retaining_skipped_bodies() {
    for eof in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let bundle = out.join("report.zip");
        let path = dir.path().join("history.sqlite");
        let (addr, worker, trap) = gateway(move |peer| {
            begin_query(peer, false);
            peer.send(&[6, 0x10, 9, 0x99, 0, 10, 0xde, 0xad, 0xbe, 0xef]);
            let mut large = vec![6, 0x10, 4, 0x20, 5, 0xdc];
            large.resize(1500, 0xee);
            peer.send(&large);
            if eof {
                peer.close_write();
            } else {
                peer.finish();
            }
        });
        let endpoint = addr.to_string();
        let output = run_tcp(
            &endpoint,
            false,
            &[
                "--diagnostic-bundle",
                bundle.to_str().unwrap(),
                "--activity-history",
                path.to_str().unwrap(),
            ],
        );
        worker.join().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr(&output).contains("2 TCP frame(s) skipped before disconnecting"));
        history(
            &path,
            if eof { "failed" } else { "finished" },
            false,
            &endpoint,
        );
        let left: Vec<_> = std::fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, ["report.zip"]);
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&bundle).unwrap()).unwrap();
        let names: Vec<_> = zip
            .file_names()
            .map(Result::unwrap)
            .filter(|n| n.ends_with("recording.jsonl"))
            .map(|n| n.into_owned())
            .collect();
        assert_eq!(names.len(), 1);
        let mut text = String::new();
        zip.by_name(&names[0])
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        let lines: Vec<Value> = text
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(lines[0]["operation_kind"], "serial-number-address");
        assert_eq!(lines.last().unwrap()["outcome"], "failed");
        assert_eq!(lines.last().unwrap()["records_dropped"], 0);
        assert_eq!(
            lines
                .iter()
                .filter(|l| l["event"] == "frame-skipped")
                .count(),
            2
        );
        assert!(lines
            .iter()
            .any(|l| l["event"] == "frame-skipped" && l["total_length"] == 1500));
        assert!(!text.contains("deadbeef"));
        assert!(!text.contains("eeeeeeee"));
        no_udp(&trap);
    }
}

#[cfg(unix)]
#[test]
fn sigint_releases_the_observed_tcp_socket_and_keeps_interrupted_private_free_history() {
    for by_address in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        let (ready, started) = std::sync::mpsc::channel();
        let (addr, worker, trap) = gateway(move |peer| {
            begin_query(peer, by_address);
            ready.send(()).unwrap();
            peer.eof();
        });
        let endpoint = addr.to_string();
        let mut child = KillOnDrop(
            command(&endpoint, by_address)
                .args([
                    "--tunnel-transport",
                    "tcp",
                    "--activity-history",
                    path.to_str().unwrap(),
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        started.recv_timeout(WAIT).unwrap();
        assert!(path.exists(), "history must precede the adapter request");
        assert!(Command::new("kill")
            .args(["-INT", &child.0.id().to_string()])
            .status()
            .unwrap()
            .success());
        let status = super::wait_for(&mut child.0);
        worker.join().unwrap();
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            status.signal(),
            Some(2),
            "preserve the existing Ctrl-C process policy"
        );
        history(&path, "unknown", true, &endpoint);
        no_udp(&trap);
    }
}
