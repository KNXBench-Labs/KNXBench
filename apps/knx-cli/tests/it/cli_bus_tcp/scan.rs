//! Exercise actual CLI scans, device evidence, endpoint admission and cleanup on loopback peers.

use super::{
    assert_no_datagram, knx, read_frame, refused_tcp_with_udp_trap, stderr, stdout, wait_for,
    KillOnDrop, CHANNEL, DISCONNECT_RESPONSE, TCP_CONNECT_REQUEST, TCP_CONNECT_RESPONSE, WAIT,
};
use knx_net::{cemi::decode_l_data, ApplicationService, Destination, LDataMessageKind, Tpci};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket},
    process::{Output, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

fn run_scan(gateway: &str, flags: &[&str]) -> Output {
    let mut child = KillOnDrop(
        knx()
            .args(["bus", "scan", "--gateway", gateway])
            .args(flags)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    // Scan output in these bounded fixtures is smaller than a pipe buffer.
    let status = wait_for(&mut child.0);
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_end(&mut stderr)
        .unwrap();
    Output {
        status,
        stdout,
        stderr,
    }
}

fn accept_before_deadline(listener: TcpListener) -> TcpStream {
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + WAIT;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_read_timeout(Some(WAIT)).unwrap();
                stream.set_write_timeout(Some(WAIT)).unwrap();
                return stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "CLI never connected");
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("accept: {error}"),
        }
    }
}

fn tcp_scan_gateway(
    rest: impl FnOnce(&mut TcpStream) + Send + 'static,
) -> (SocketAddr, JoinHandle<()>, UdpSocket) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let trap = UdpSocket::bind(address).unwrap();
    trap.set_read_timeout(Some(Duration::from_millis(150)))
        .unwrap();
    let worker = thread::spawn(move || {
        let mut stream = accept_before_deadline(listener);
        assert_eq!(read_frame(&mut stream), TCP_CONNECT_REQUEST);
        stream.write_all(&TCP_CONNECT_RESPONSE).unwrap();
        rest(&mut stream);
    });
    (address, worker, trap)
}

fn assert_probe(frame: &[u8], device: u8, tpci: Tpci) {
    assert_eq!(
        &frame[..4],
        [6, 0x10, 4, 0x20],
        "TUNNELLING_REQUEST, never TCP ACK"
    );
    assert_eq!(&frame[6..8], [4, CHANNEL]);
    let cemi = decode_l_data(&frame[10..]).unwrap();
    assert_eq!(cemi.kind, LDataMessageKind::Request);
    // Tunnel requests leave source assignment to the gateway (the existing scan path).
    assert_eq!(cemi.source.to_string(), "0.0.0");
    assert_eq!(
        cemi.destination,
        Destination::Individual(format!("1.1.{device}").parse().unwrap())
    );
    assert_eq!(cemi.transport, tpci);
    assert_eq!(
        cemi.service,
        match tpci {
            Tpci::NumberedData { .. } =>
                ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
            _ => ApplicationService::NoApplicationPdu,
        }
    );
}

/// A literal L_Data.ind from the probed device with DD0 mask 07b0, not a socket receipt.
fn descriptor_reply(device: u8, sequence: u8) -> Vec<u8> {
    vec![
        6, 0x10, 4, 0x20, 0, 23, 4, CHANNEL, sequence, 0, 0x29, 0, 0xbc, 0x60, 0x11, device, 0x11,
        0xfa, 3, 0x43, 0x40, 7, 0xb0,
    ]
}

fn finish_tcp_scan(stream: &mut TcpStream) {
    let frame = read_frame(stream);
    assert_eq!(&frame[2..4], [2, 9], "inner DISCONNECT_REQUEST");
    assert_eq!(frame[6], CHANNEL);
    let _ = stream.write_all(&DISCONNECT_RESPONSE);
    // Unlike a best-effort disconnect request alone, EOF proves observed socket release.
    let mut byte = [0];
    assert_eq!(
        stream.read(&mut byte).unwrap(),
        0,
        "TCP socket did not close cleanly"
    );
}

#[test]
fn tcp_descriptor_scan_single_and_range_preserve_exclusions_and_cleanup() {
    for (target, devices) in [
        (vec!["--device", "1.1.5"], vec![5]),
        (
            vec![
                "--line",
                "1.1",
                "--range",
                "1.1.5-1.1.7",
                "--exclude",
                "1.1.6",
            ],
            vec![5, 7],
        ),
    ] {
        let expected = devices.clone();
        let (address, worker, trap) = tcp_scan_gateway(move |stream| {
            for device in expected {
                assert_probe(&read_frame(stream), device, Tpci::Connect);
                assert_probe(&read_frame(stream), device, Tpci::NumberedData { seq: 0 });
                stream.write_all(&descriptor_reply(device, 7)).unwrap();
                assert_probe(&read_frame(stream), device, Tpci::Disconnect);
            }
            finish_tcp_scan(stream);
        });
        let mut flags = vec![
            "--tunnel-transport",
            "tcp",
            "--timeout-ms",
            "1000",
            "--pause-ms",
            "0",
        ];
        flags.extend(target);
        let output = run_scan(&address.to_string(), &flags);
        worker.join().expect("TCP scan peer failed");
        assert!(output.status.success(), "{}", stderr(&output));
        assert!(stderr(&output).contains("over TCP"), "{}", stderr(&output));
        assert!(!stderr(&output).contains("skipped"), "{}", stderr(&output));
        let text = stdout(&output);
        for device in &devices {
            assert!(
                text.contains(&format!("1.1.{device}  occupied (mask 0x07b0)")),
                "{text}"
            );
        }
        assert!(
            text.contains(&format!(
                "{} probed: {} occupied, 0 occupied-silent",
                devices.len(),
                devices.len()
            )),
            "{text}"
        );
        if devices.len() == 2 {
            assert!(!text.contains("1.1.6  "), "{text}");
            assert!(text.contains("1 excluded"), "{text}");
        }
        assert_no_datagram(&trap);
    }
}

#[test]
fn tcp_scan_reports_skipped_frames_before_success_or_failure_cleanup() {
    for reply in [true, false] {
        let (address, worker, trap) = tcp_scan_gateway(move |stream| {
            assert_probe(&read_frame(stream), 5, Tpci::Connect);
            assert_probe(&read_frame(stream), 5, Tpci::NumberedData { seq: 0 });
            let mut unknown = vec![6, 0x10, 9, 0x99, 0, 16];
            unknown.extend_from_slice(&[0xee; 10]);
            let mut oversize = vec![6, 0x10, 4, 0x20];
            oversize.extend_from_slice(&1500u16.to_be_bytes());
            oversize.resize(1500, 0);
            stream.write_all(&unknown).unwrap();
            stream.write_all(&oversize).unwrap();
            if reply {
                stream.write_all(&descriptor_reply(5, 7)).unwrap();
                assert_probe(&read_frame(stream), 5, Tpci::Disconnect);
                finish_tcp_scan(stream);
            }
            // Otherwise EOF follows the fully sent skips with a device reply outstanding.
        });
        let output = run_scan(
            &address.to_string(),
            &[
                "--device",
                "1.1.5",
                "--tunnel-transport",
                "tcp",
                "--timeout-ms",
                "1000",
            ],
        );
        worker.join().unwrap();
        assert_eq!(output.status.success(), reply, "{}", stderr(&output));
        let errors = stderr(&output);
        let warnings: Vec<_> = errors
            .lines()
            .filter(|line| line.contains("skipped"))
            .collect();
        assert_eq!(warnings.len(), 1, "{errors}");
        assert!(
            warnings[0].starts_with("warning: 2 TCP frame(s) skipped before disconnecting"),
            "{errors}"
        );
        assert!(warnings[0].contains("their content was not kept"));
        assert!(!warnings[0].contains("lost") && !warnings[0].contains("dropped"));
        if reply {
            assert!(stdout(&output).contains("1.1.5  occupied (mask 0x07b0)"));
        } else {
            assert!(errors.contains("scan transport failed after 0 address(es)"));
            assert!(!stdout(&output).contains("1.1.5  occupied"));
        }
        assert_no_datagram(&trap);
    }
}

#[test]
fn tcp_socket_acceptance_alone_is_vacant_not_device_evidence() {
    let (address, worker, trap) = tcp_scan_gateway(|stream| {
        assert_probe(&read_frame(stream), 5, Tpci::Connect);
        assert_probe(&read_frame(stream), 5, Tpci::NumberedData { seq: 0 });
        // No indication or L_Data.con: a successful TCP send is not presence evidence.
        assert_probe(&read_frame(stream), 5, Tpci::Disconnect);
        finish_tcp_scan(stream);
    });
    let output = run_scan(
        &address.to_string(),
        &[
            "--device",
            "1.1.5",
            "--tunnel-transport",
            "tcp",
            "--timeout-ms",
            "150",
        ],
    );
    worker.join().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("1.1.5  vacant"), "{text}");
    assert!(
        text.contains("1 probed: 0 occupied, 0 occupied-silent, 0 busy, 1 vacant"),
        "{text}"
    );
    assert_no_datagram(&trap);
}

#[test]
fn tcp_connect_confirmation_without_descriptor_remains_occupied_silent() {
    let (address, worker, trap) = tcp_scan_gateway(|stream| {
        let mut confirmation = read_frame(stream);
        assert_probe(&confirmation, 5, Tpci::Connect);
        assert_probe(&read_frame(stream), 5, Tpci::NumberedData { seq: 0 });
        confirmation[10] = 0x2e; // Positive L_Data.con for this T_Connect, not an indication.
        confirmation[12] &= !1;
        confirmation[14..16].copy_from_slice(&[0x11, 0xfa]); // Gateway-assigned source.
        stream.write_all(&confirmation).unwrap();
        assert_probe(&read_frame(stream), 5, Tpci::Disconnect);
        finish_tcp_scan(stream);
    });
    let output = run_scan(
        &address.to_string(),
        &[
            "--device",
            "1.1.5",
            "--tunnel-transport",
            "tcp",
            "--timeout-ms",
            "500",
        ],
    );
    worker.join().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("1.1.5  occupied-silent"), "{text}");
    assert!(
        text.contains("1 probed: 0 occupied, 1 occupied-silent"),
        "{text}"
    );
    assert_no_datagram(&trap);
}

#[test]
fn tcp_scan_peer_close_during_probe_is_failure_without_udp_fallback() {
    let (address, worker, trap) = tcp_scan_gateway(|stream| {
        assert_probe(&read_frame(stream), 5, Tpci::Connect);
        assert_probe(&read_frame(stream), 5, Tpci::NumberedData { seq: 0 });
        // Return and close with the reply outstanding.
    });
    let output = run_scan(
        &address.to_string(),
        &[
            "--device",
            "1.1.5",
            "--tunnel-transport",
            "tcp",
            "--timeout-ms",
            "1000",
        ],
    );
    worker.join().unwrap();
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("scan transport failed after 0 address(es)"),
        "{}",
        stderr(&output)
    );
    assert!(!stdout(&output).contains("1.1.5  occupied"));
    assert_no_datagram(&trap);
}

#[test]
fn tcp_scan_refusal_and_handshake_eof_never_fall_back() {
    let (port, trap) = refused_tcp_with_udp_trap();
    let output = run_scan(
        &format!("127.0.0.1:{port}"),
        &["--device", "1.1.5", "--tunnel-transport", "tcp"],
    );
    assert!(!output.status.success());
    assert!(stderr(&output).contains("could not connect to"));
    assert!(stdout(&output).is_empty());
    assert_no_datagram(&trap);

    for reject in [false, true] {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let trap = UdpSocket::bind(address).unwrap();
        trap.set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        let worker = thread::spawn(move || {
            let mut stream = accept_before_deadline(listener);
            assert_eq!(read_frame(&mut stream), TCP_CONNECT_REQUEST);
            if reject {
                stream.write_all(&[6, 0x10, 2, 6, 0, 8, 0, 0x24]).unwrap();
                assert_eq!(
                    stream.read(&mut [0]).unwrap(),
                    0,
                    "refused stream must close"
                );
            }
        });
        let output = run_scan(
            &address.to_string(),
            &["--device", "1.1.5", "--tunnel-transport", "tcp"],
        );
        worker.join().unwrap();
        assert!(!output.status.success());
        assert!(
            stderr(&output).contains("could not connect to"),
            "{}",
            stderr(&output)
        );
        assert!(stdout(&output).is_empty());
        assert_no_datagram(&trap);
    }
}

#[test]
fn scan_invalid_transport_targets_and_dry_runs_open_neither_transport() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let trap = UdpSocket::bind(address).unwrap();
    trap.set_read_timeout(Some(Duration::from_millis(150)))
        .unwrap();
    for (extra, expected) in [
        (vec!["--tunnel-transport", "tls"], "must be udp or tcp"),
        (vec!["--tunnel-transport", "TCP"], "must be udp or tcp"),
        (vec!["--tunnel-transport", ""], "must be udp or tcp"),
        (vec!["--tunnel-transport"], "needs a value"),
        (vec!["--tunnel-transport", "--dry-run"], "needs a value"),
        (
            vec!["--tunnel-transport", "tcp", "--tunnel-transport", "tcp"],
            "duplicate --tunnel-transport",
        ),
        (
            vec!["--tunnel-transport", "udp", "--tunnel-transport", "tcp"],
            "duplicate --tunnel-transport",
        ),
        (
            vec!["--tunnel-transport", "tcp", "--serial", "absent-serial"],
            "unrecognized argument: --serial",
        ),
        (
            vec!["--tunnel-transport", "tcp", "--exclude", "broken"],
            "invalid address in --exclude",
        ),
        (
            vec!["--tunnel-transport", "tcp", "--range", "1.1.5-1.1.6"],
            "cannot be combined with --device",
        ),
    ] {
        let mut flags = vec!["--device", "1.1.5"];
        flags.extend(extra);
        let output = run_scan(&address.to_string(), &flags);
        assert!(!output.status.success(), "{flags:?}");
        assert!(
            stderr(&output).contains(expected),
            "{flags:?}: {}",
            stderr(&output)
        );
        assert!(!stderr(&output).contains("could not connect"));
    }
    for transport in ["tcp", "udp"] {
        for target in [
            vec!["--device", "1.1.5"],
            vec![
                "--line",
                "1.1",
                "--range",
                "1.1.5-1.1.7",
                "--exclude",
                "1.1.6",
            ],
        ] {
            let mut flags = vec!["--tunnel-transport", transport, "--dry-run"];
            flags.extend(target);
            let output = run_scan(&address.to_string(), &flags);
            assert!(output.status.success(), "{}", stderr(&output));
            assert!(stdout(&output).contains("dry run:"));
            assert!(!stderr(&output).contains("connected"));
        }
    }
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_no_datagram(&trap);
}

#[test]
fn scan_refuses_non_unicast_zero_port_and_invalid_ipv4_before_dry_run() {
    for gateway in [
        "0.0.0.0:3671",
        "255.255.255.255:3671",
        "224.0.23.12:3671",
        "127.0.0.1:0",
        "[::1]:3671",
        "localhost:3671",
    ] {
        let output = run_scan(
            gateway,
            &[
                "--device",
                "1.1.5",
                "--tunnel-transport",
                "tcp",
                "--dry-run",
            ],
        );
        assert!(!output.status.success(), "{gateway}");
        assert!(stderr(&output).contains("--gateway"));
        assert!(stdout(&output).is_empty());
    }
}

#[test]
fn scan_default_and_explicit_udp_receive_descriptor_and_never_connect_tcp() {
    for extra in [vec![], vec!["--tunnel-transport", "udp"]] {
        let gateway = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        gateway.set_read_timeout(Some(WAIT)).unwrap();
        let address = gateway.local_addr().unwrap();
        let tcp_trap = TcpListener::bind(address).unwrap();
        tcp_trap.set_nonblocking(true).unwrap();
        let worker = thread::spawn(move || {
            let mut buffer = [0; 128];
            let (len, client) = gateway.recv_from(&mut buffer).unwrap();
            assert_eq!(len, 26);
            assert_eq!(&buffer[2..4], [2, 5]);
            assert_eq!(buffer[7], 1, "UDP control HPAI");
            assert_eq!(buffer[15], 1, "UDP data HPAI");
            let mut response = TCP_CONNECT_RESPONSE;
            response[9] = 1;
            response[10..14].copy_from_slice(&Ipv4Addr::LOCALHOST.octets());
            response[14..16].copy_from_slice(&address.port().to_be_bytes());
            gateway.send_to(&response, client).unwrap();
            for (sequence, tpci) in [
                (0, Tpci::Connect),
                (1, Tpci::NumberedData { seq: 0 }),
                (2, Tpci::Disconnect),
            ] {
                let (len, sender) = gateway.recv_from(&mut buffer).unwrap();
                assert_eq!(sender, client);
                assert_probe(&buffer[..len], 5, tpci);
                assert_eq!(buffer[8], sequence);
                gateway
                    .send_to(&[6, 0x10, 4, 0x21, 0, 10, 4, CHANNEL, sequence, 0], client)
                    .unwrap();
                if sequence == 1 {
                    gateway.send_to(&descriptor_reply(5, 0), client).unwrap();
                    let (len, _) = gateway.recv_from(&mut buffer).unwrap();
                    assert_eq!(&buffer[..len], [6, 0x10, 4, 0x21, 0, 10, 4, CHANNEL, 0, 0]);
                }
            }
            let (len, _) = gateway.recv_from(&mut buffer).unwrap();
            assert_eq!(len, 16);
            assert_eq!(&buffer[2..4], [2, 9]);
            assert_eq!(buffer[6], CHANNEL);
            gateway.send_to(&DISCONNECT_RESPONSE, client).unwrap();
        });
        let mut flags = vec!["--device", "1.1.5", "--timeout-ms", "1000"];
        flags.extend(extra);
        let output = run_scan(&address.to_string(), &flags);
        worker.join().expect("UDP peer failed");
        assert!(output.status.success(), "{}", stderr(&output));
        assert!(
            stdout(&output).contains("1.1.5  occupied (mask 0x07b0)"),
            "{}",
            stdout(&output)
        );
        assert!(stderr(&output).contains("over UDP"), "{}", stderr(&output));
        assert_eq!(
            tcp_trap.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}
