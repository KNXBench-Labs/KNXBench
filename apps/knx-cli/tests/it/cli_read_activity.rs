//! Read-only CLI lifecycle checks using an unanswered loopback UDP peer only.

use knx_app::commissioning_activity::OneShotLog;
use std::net::UdpSocket;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn explicit_unavailable_read_history_is_refused_before_adapter_without_replacing_evidence() {
    for directory_history in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let history = directory.path().join("unavailable-history");
        let original = b"foreign evidence must remain unchanged";
        if directory_history {
            std::fs::create_dir(&history).unwrap();
        } else {
            std::fs::write(&history, original).unwrap();
        }
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args(["device", "find-serial", "--address", "1.1.67", "--gateway"])
            .arg(peer.local_addr().unwrap().to_string())
            .arg("--activity-history")
            .arg(&history)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .contains("activity history unavailable; lookup not started"));
        if directory_history {
            assert_eq!(std::fs::read_dir(&history).unwrap().count(), 0);
        } else {
            assert_eq!(std::fs::read(&history).unwrap(), original);
        }
        let mut packet = [0; 2048];
        assert!(
            peer.recv_from(&mut packet).is_err(),
            "refused lookup opened an adapter"
        );
    }
}

#[test]
fn duplicate_read_history_flags_are_refused_before_history_or_adapter() {
    let directory = tempfile::tempdir().unwrap();
    let history = directory.path().join("history.sqlite");
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_millis(150)))
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "find-serial", "--address", "1.1.67", "--gateway"])
        .arg(peer.local_addr().unwrap().to_string())
        .arg("--activity-history")
        .arg(&history)
        .arg("--activity-history")
        .arg(&history)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("give --activity-history only once"));
    assert!(!history.exists());
    let mut packet = [0; 2048];
    assert!(peer.recv_from(&mut packet).is_err());
}

#[test]
fn find_serial_usage_names_optional_history_and_its_refusal_boundary() {
    let output = Command::new(env!("CARGO_BIN_EXE_knx")).output().unwrap();
    let usage = String::from_utf8(output.stderr).unwrap();
    let section = usage
        .split("knx device ")
        .find(|section| section.starts_with("find-serial "))
        .unwrap();
    assert!(section.contains("[--activity-history <path>]"));
    assert!(section.contains("unavailable explicit history refuses lookup before a tunnel"));
}

#[test]
fn explicit_service_control_read_history_failure_is_refused_before_adapter() {
    use knx_net::core::services::{CONNECT_RESPONSE, E_CONNECTION_TYPE};
    let directory = tempfile::tempdir().unwrap();
    let history = directory.path().join("foreign-evidence");
    let original = b"foreign evidence must remain unchanged";
    std::fs::write(&history, original).unwrap();
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "service-control", "1.1.67", "--gateway"])
        .arg(peer.local_addr().unwrap().to_string())
        .arg("--activity-history")
        .arg(&history)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut packet = [0; 2048];
    let received = peer.recv_from(&mut packet);
    if let Ok((_, client)) = received {
        let refusal = knx_net::frame::encode_frame(CONNECT_RESPONSE, &[0, E_CONNECTION_TYPE]);
        peer.send_to(&refusal, client).unwrap();
    }
    // Reap even if the pre-adapter assertion is about to fail.
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert_eq!(std::fs::read(&history).unwrap(), original);
    assert!(
        received.is_err(),
        "explicit service-control read history failure must refuse before adapter"
    );
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("activity history unavailable; read not started"));
}

#[test]
fn refused_loopback_connection_persists_a_terminal_read_failure() {
    use knx_net::core::services::{CONNECT_RESPONSE, E_CONNECTION_TYPE};
    let directory = tempfile::tempdir().unwrap();
    let history = directory.path().join("history.sqlite");
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let gateway = peer.local_addr().unwrap().to_string();
    let responder = std::thread::spawn(move || {
        let mut packet = [0; 2048];
        let (_, client) = peer.recv_from(&mut packet).unwrap();
        let refusal = knx_net::frame::encode_frame(CONNECT_RESPONSE, &[0, E_CONNECTION_TYPE]);
        peer.send_to(&refusal, client).unwrap();
    });
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "device",
            "find-serial",
            "--address",
            "1.1.67",
            "--gateway",
            &gateway,
            "--activity-history",
        ])
        .arg(&history)
        .output()
        .unwrap();
    responder.join().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("could not connect"));
    let reader = Arc::new(OneShotLog::for_run(history).unwrap());
    let (rows, more) = reader.history_page(0, 100).unwrap();
    assert!(!more);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].activity.kind, "serialLookup");
    assert_eq!(rows[0].activity.state, "failed");
    assert!(rows[0].activity.finished_at.is_some());
    assert!(!rows[0].interrupted);
    assert!(rows[0].activity.address.is_none());
}

#[test]
fn a_completed_lookup_with_no_match_is_finished_not_a_transport_failure() {
    use knx_net::core::{hpai::Hpai, services};
    use knx_net::{frame, tunnelling};
    let directory = tempfile::tempdir().unwrap();
    let history = directory.path().join("history.sqlite");
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(6))).unwrap();
    let gateway = peer.local_addr().unwrap();
    let responder = std::thread::spawn(move || {
        let channel = 0x15;
        let mut packet = [0; 2048];
        let (size, client) = peer.recv_from(&mut packet).unwrap();
        let (header, _) = frame::decode_frame(&packet[..size]).unwrap();
        assert_eq!(header.service_type, services::CONNECT_REQUEST);
        // Same verified handshake shape as knx-net's loopback client tests.
        let mut body = vec![channel, services::E_NO_ERROR];
        body.extend_from_slice(
            &Hpai {
                addr: std::net::Ipv4Addr::LOCALHOST,
                port: gateway.port(),
            }
            .encode(),
        );
        body.extend_from_slice(&[4, tunnelling::TUNNEL_CONNECTION, 0x11, 0x01]);
        peer.send_to(
            &frame::encode_frame(services::CONNECT_RESPONSE, &body),
            client,
        )
        .unwrap();
        let mut seen = vec![services::CONNECT_REQUEST];
        loop {
            let (size, source) = peer.recv_from(&mut packet).unwrap();
            assert_eq!(source, client);
            let (header, body) = frame::decode_frame(&packet[..size]).unwrap();
            seen.push(header.service_type);
            match header.service_type {
                tunnelling::TUNNELLING_REQUEST => {
                    let request = tunnelling::decode_tunnelling_request(body).unwrap();
                    assert_eq!(request.channel_id, channel);
                    let ack = tunnelling::encode_tunnelling_ack(
                        channel,
                        request.sequence_counter,
                        tunnelling::E_NO_ERROR,
                    );
                    peer.send_to(
                        &frame::encode_frame(tunnelling::TUNNELLING_ACK, &ack),
                        client,
                    )
                    .unwrap();
                    // No simulated bus device answers the serial-number query.
                }
                services::DISCONNECT_REQUEST => {
                    assert_eq!(body.first(), Some(&channel));
                    let reply = services::encode_disconnect_response(channel, services::E_NO_ERROR);
                    peer.send_to(
                        &frame::encode_frame(services::DISCONNECT_RESPONSE, &reply),
                        client,
                    )
                    .unwrap();
                    return seen;
                }
                other => panic!("unexpected loopback service {other:#06x}"),
            }
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "find-serial", "0083:12345678", "--gateway"])
        .arg(gateway.to_string())
        .arg("--activity-history")
        .arg(&history)
        .output()
        .unwrap();
    let seen = responder.join().unwrap();
    assert_eq!(
        seen,
        [
            services::CONNECT_REQUEST,
            tunnelling::TUNNELLING_REQUEST,
            services::DISCONNECT_REQUEST,
        ]
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "legacy no-match exit must remain"
    );
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("no answer"));
    let reader = Arc::new(OneShotLog::for_run(history).unwrap());
    let (rows, more) = reader.history_page(0, 100).unwrap();
    assert!(!more);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].activity.kind, "serialLookup");
    assert_eq!(
        rows[0].activity.state, "finished",
        "a completed no-match lookup is not a transport failure"
    );
    assert!(rows[0].activity.finished_at.is_some());
    assert!(!rows[0].interrupted);
    assert!(rows[0].activity.address.is_none());
}

#[test]
fn find_serial_records_before_adapter_and_preserves_interruption_without_payload() {
    for by_address in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let history = directory.path().join("history.sqlite");
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let gateway = peer.local_addr().unwrap().to_string();
        let mut command = Command::new(env!("CARGO_BIN_EXE_knx"));
        command.args(["device", "find-serial"]);
        if by_address {
            command.args(["--address", "1.1.67"]);
        } else {
            command.arg("0083:12345678");
        }
        let mut child = command
            .args(["--gateway", &gateway, "--activity-history"])
            .arg(&history)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut packet = [0; 2048];
        let received = peer.recv_from(&mut packet);
        let published_before_request = history.exists();
        // Always reap before assertions, including on refusal/timeout.
        let _ = child.kill();
        child.wait().unwrap();
        assert!(received.is_ok(), "the loopback adapter was never reached");
        assert!(
            published_before_request,
            "find-serial must persist its read before the first adapter request"
        );
        let reader = Arc::new(OneShotLog::for_run(history.clone()).unwrap());
        let (rows, more) = reader.history_page(0, 100).unwrap();
        assert!(!more);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.activity.kind, "serialLookup");
        assert_eq!(row.activity.state, "unknown");
        assert!(row.activity.address.is_none());
        assert!(row.interrupted);
        assert!(row.activity.finished_at.is_none());
        let metadata = serde_json::to_string(&rows).unwrap();
        for private in ["0083:12345678", gateway.as_str(), history.to_str().unwrap()] {
            assert!(
                !metadata.contains(private),
                "private request data entered history"
            );
        }
    }
}
