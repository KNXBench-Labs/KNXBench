//! Pin strict transport parsing, no-I/O admission and durable backup refusal through the CLI.

use super::*;
use std::net::{Ipv4Addr, TcpListener, UdpSocket};

fn replace(f: &Fixture, cmd: &Command, value: &std::ffi::OsStr, with: &str) -> Command {
    let mut result = f.inputs.bare();
    let mut found = 0;
    for arg in cmd.get_args() {
        if arg == value {
            result.arg(with);
            found += 1;
        } else {
            result.arg(arg);
        }
    }
    assert_eq!(found, 1);
    result
}

#[test]
fn strict_transport_preview_simulator_and_gateway_refuse_before_input_or_output_io() {
    let f = inputs();
    let never = f.dir.path().join("never");
    let mut cases = vec![
        (vec!["--tunnel-transport"], "needs a value"),
        (vec!["--tunnel-transport", "--report", "x"], "needs a value"),
        (vec!["--tunnel-transport", ""], "must be udp or tcp"),
        (vec!["--tunnel-transport", "TCP"], "must be udp or tcp"),
        (vec!["--tunnel-transport", "tls"], "must be udp or tcp"),
        (
            vec!["--tunnel-transport", "tcp", "--tunnel-transport", "udp"],
            "duplicate",
        ),
        (
            vec!["--tunnel-transport", "udp", "--tunnel-transport", "udp"],
            "duplicate",
        ),
        (vec!["--tunnel-transport", "tcp"], "offline preview"),
        (vec!["--tunnel-transport", "udp"], "offline preview"),
        (
            vec!["--recover", "x", "--tunnel-transport", "tcp"],
            "offline preview",
        ),
        (
            vec!["--simulate", "--tunnel-transport", "tcp"],
            "--simulate",
        ),
        (
            vec!["--simulate", "--tunnel-transport", "udp"],
            "--simulate",
        ),
        (
            vec!["--gateway", "127.0.0.1:9", "--tunnel-transport", "tcp"],
            "offline preview",
        ),
    ];
    for addr in [
        "bad",
        "0.0.0.0:3671",
        "224.0.23.12:3671",
        "255.255.255.255:3671",
        "127.0.0.1:0",
        "[::1]:3671",
    ] {
        cases.push((vec!["--gateway", addr], "--gateway"));
    }
    for (extra, expected) in cases {
        let out = collect(
            f.bare()
                .args(["device", "system2-validation", "--bundle"])
                .arg(&never)
                .arg("--activity-history")
                .arg(&never)
                .args(extra),
        );
        exit(&out, 1);
        assert!(stderr(&out).contains(expected), "{}", stderr(&out));
        assert!(!never.exists());
        assert!(stdout(&out).is_empty(), "bundle not read before refusal");
    }
}

#[test]
fn wrong_phrases_and_overwrite_refuse_without_socket_history_or_operation() {
    let f = Fixture::new();
    let tcp = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    tcp.set_nonblocking(true).unwrap();
    let udp = UdpSocket::bind(tcp.local_addr().unwrap()).unwrap();
    udp.set_nonblocking(true).unwrap();
    let cmd = f.command(&tcp.local_addr().unwrap().to_string(), Some("tcp"), false);
    for phrase in [
        f.bundle.consent_phrase(),
        f.bundle.acceptance_phrase(),
        knx_core::commissioning::mutation::required_confirmation_phrase(
            f.inputs.device.address(),
            knx_core::WriteScope::Download,
        ),
        knx_net::commissioning::hardware_release::required_release_phrase(
            f.inputs.device.address(),
        ),
    ] {
        let out = collect(&mut replace(&f, &cmd, phrase.as_ref(), "wrong"));
        exit(&out, 1);
        assert!(stderr(&out).contains("exact"), "{}", stderr(&out));
        assert!(!f.operation().exists());
        assert!(!f.history().exists());
    }
    let report = f.inputs.dir.path().join("keep-report.json");
    fs::write(&report, b"immutable report").unwrap();
    let out = collect(
        f.command(&tcp.local_addr().unwrap().to_string(), Some("tcp"), false)
            .arg("--report")
            .arg(&report),
    );
    exit(&out, 1);
    assert!(stderr(&out).contains("report destination already exists"));
    assert_eq!(fs::read(&report).unwrap(), b"immutable report");
    assert!(!f.operation().exists());
    assert!(!f.history().exists());
    assert_eq!(
        tcp.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    super::super::probe::peer::no_udp(&udp);
}

#[test]
fn tcp_connection_refusal_never_falls_back_and_udp_controls_are_handshake_only() {
    for (recover, transport) in [
        (false, Some("tcp")),
        (false, None),
        (false, Some("udp")),
        (true, Some("tcp")),
        (true, None),
        (true, Some("udp")),
    ] {
        let f = Fixture::new();
        if recover {
            let (out, _) = f.run(false, false, true);
            exit(&out, 1);
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let _entered = runtime.enter();
        let socket = tokio::net::TcpSocket::new_v4().unwrap();
        socket.bind((Ipv4Addr::LOCALHOST, 0).into()).unwrap();
        let addr = socket.local_addr().unwrap();
        let udp = UdpSocket::bind(addr).unwrap();
        if transport == Some("tcp") {
            udp.set_nonblocking(true).unwrap();
            let out = collect(&mut f.command(&addr.to_string(), transport, recover));
            exit(&out, 1);
            assert!(stderr(&out).contains("gateway refused/unavailable"));
            super::super::probe::peer::no_udp(&udp);
        } else {
            udp.set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let worker = std::thread::spawn(move || {
                let mut bytes = [0; 2048];
                let (n, client) = udp.recv_from(&mut bytes).unwrap();
                assert_eq!(n, 26);
                assert_eq!(&bytes[..4], &[6, 0x10, 2, 5]);
                assert_eq!(bytes[7], 1, "UDP HPAI");
                udp.send_to(&[6, 0x10, 2, 6, 0, 8, 0, 0x24], client)
                    .unwrap();
            });
            let out = collect(&mut f.command(&addr.to_string(), transport, recover));
            worker.join().unwrap();
            exit(&out, 1);
            assert!(stderr(&out).contains("gateway refused/unavailable"));
            assert!(!stderr(&out).contains("TCP frame(s) skipped"));
        }
        assert_eq!(f.store().recovery().is_some(), recover);
        assert_eq!(f.store().mutation_possible(), recover);
        assert_eq!(f.rows().last().unwrap()["state"], "notSent");
    }
}

#[test]
fn backup_persistence_failure_refuses_every_write_including_setup() {
    let f = Fixture::new();
    let device = Arc::clone(&f.inputs.device);
    let old = device.system2_snapshot();
    let operation = f.operation();
    let (addr, worker, trap) = gateway(250, move |p| {
        let mut blocked = false;
        p.simulate(&device, false, |frame| {
            assert!(
                !is_mutation(&frame.service),
                "no setup/write before durable backup"
            );
            if !blocked && matches!(frame.service, A::MemoryRead { .. }) {
                let head: serde_json::Value =
                    serde_json::from_slice(&fs::read(operation.join("HEAD")).unwrap()).unwrap();
                let next = head["sequence"].as_u64().unwrap() + 1;
                fs::create_dir(operation.join(format!("checkpoint-{next:08}.json"))).unwrap();
                blocked = true;
            }
        });
        assert!(blocked, "backup read reached persistence injection");
    });
    let out = collect(&mut f.command(&addr.to_string(), Some("tcp"), false));
    worker.join().unwrap();
    exit(&out, 1);
    no_retry(&trap);
    assert_eq!(f.inputs.device.system2_snapshot(), old);
    assert_eq!(f.rows()[0]["state"], "notSent");
    assert!(!stdout(&out).contains("HARDWARE outcome: Finished"));
}

#[test]
fn recovery_wrong_source_binding_refuses_before_any_connection_and_preserves_original() {
    let f = Fixture::new();
    let (out, _) = f.run(false, false, true);
    exit(&out, 1);
    let before = tree(&f.operation());
    let old = f.inputs.device.system2_snapshot();
    let tcp = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    tcp.set_nonblocking(true).unwrap();
    let udp = UdpSocket::bind(tcp.local_addr().unwrap()).unwrap();
    udp.set_nonblocking(true).unwrap();
    let mut changed = f.bundle.clone();
    changed.binding.source_sha256 = "c".repeat(64);
    fs::write(f.inputs.output(), serde_json::to_vec(&changed).unwrap()).unwrap();
    let out = collect(&mut f.command(&tcp.local_addr().unwrap().to_string(), Some("tcp"), true));
    exit(&out, 1);
    assert!(stderr(&out).contains("project preparation record or retained digest differs"));
    assert_eq!(
        tcp.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    super::super::probe::peer::no_udp(&udp);
    assert_eq!(
        tree(&f.operation()),
        before,
        "source refusal must not edit original operation"
    );
    assert_eq!(f.inputs.device.system2_snapshot(), old);
}
