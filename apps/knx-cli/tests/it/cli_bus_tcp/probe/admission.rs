//! Refuse invalid probe admission before contact while preserving offline previews and stores.

use super::{
    collect, failed,
    fixture::{tree, Fixture},
    peer::no_udp,
    stderr, stdout,
};
use std::net::{TcpListener, UdpSocket};

struct Trap {
    tcp: TcpListener,
    udp: UdpSocket,
    endpoint: String,
}
impl Trap {
    fn new() -> Self {
        let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = tcp.local_addr().unwrap();
        tcp.set_nonblocking(true).unwrap();
        let udp = UdpSocket::bind(addr).unwrap();
        udp.set_nonblocking(true).unwrap();
        Self {
            tcp,
            udp,
            endpoint: addr.to_string(),
        }
    }
    fn silent(&self) {
        assert_eq!(
            self.tcp.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        no_udp(&self.udp);
    }
}

#[test]
fn transport_flags_and_preview_combinations_refuse_before_input_openers_or_contact() {
    let fixture = Fixture::new();
    let trap = Trap::new();
    for (flags, message) in [
        (vec!["--tunnel-transport"], "needs a value"),
        (vec!["--tunnel-transport", "tls"], "must be udp or tcp"),
        (vec!["--tunnel-transport", "TCP"], "must be udp or tcp"),
        (vec!["--tunnel-transport", "--preview"], "needs a value"),
        (
            vec!["--tunnel-transport", "udp", "--tunnel-transport", "tcp"],
            "duplicate --tunnel-transport",
        ),
        (
            vec!["--preview", "--tunnel-transport", "udp"],
            "cannot be used with --preview",
        ),
        (
            vec!["--tunnel-transport", "tcp", "--preview"],
            "cannot be used with --preview",
        ),
        (
            vec![
                "--preview",
                "--system2-source",
                "--parameter-ref",
                "W_R",
                "--tunnel-transport",
                "tcp",
            ],
            "cannot be used with --preview",
        ),
        (
            vec![
                "--preview",
                "--system2-source",
                "--parameter-ref",
                "W_R",
                "--tunnel-transport",
                "udp",
            ],
            "cannot be used with --preview",
        ),
    ] {
        let output = collect(
            fixture
                .bare_command()
                .args([
                    "device",
                    "byte-order-probe",
                    "1.1.67",
                    "--project",
                    "missing.knxdb",
                    "--gateway",
                    &trap.endpoint,
                ])
                .args(flags),
        );
        failed(&output, message);
        trap.silent();
        fixture.intact();
        assert!(!stderr(&output).contains("project not found"));
    }
}

#[test]
fn invalid_endpoint_target_and_unsupported_read_write_history_flags_never_contact_or_create_state()
{
    let fixture = Fixture::new();
    let trap = Trap::new();
    for endpoint in [
        "localhost:3671",
        "[::1]:3671",
        "0.0.0.0:3671",
        "255.255.255.255:3671",
        "224.0.23.12:3671",
        "127.0.0.1:0",
        "invalid",
    ] {
        let output = collect(&mut fixture.command(endpoint, Some("tcp")));
        failed(&output, "--gateway");
        trap.silent();
        fixture.intact();
    }
    for target in ["0.0.0", "1.1.0", "1.1.256", "bad"] {
        let output = collect(
            fixture
                .bare_command()
                .args(["device", "byte-order-probe", target, "--project"])
                .arg(&fixture.project)
                .arg("--product-db")
                .arg(&fixture.products)
                .args(["--gateway", &trap.endpoint, "--tunnel-transport", "tcp"]),
        );
        assert_eq!(
            output.status.code(),
            Some(1),
            "{target}: {}",
            stderr(&output)
        );
        assert!(!stderr(&output).contains("could not connect"));
        trap.silent();
        fixture.intact();
    }
    for flag in [
        "--activity-history",
        "--serial",
        "--source",
        "--confirm",
        "--key-file",
        "--restart",
        "--force",
        "--confirm-read",
        "--enable",
    ] {
        let history = fixture.dir.path().join("refused-history.sqlite");
        let output = collect(
            fixture
                .command(&trap.endpoint, Some("tcp"))
                .arg(flag)
                .arg(&history),
        );
        failed(&output, &format!("unknown flag {flag}"));
        assert!(!history.exists());
        trap.silent();
        fixture.intact();
    }
    let output = collect(
        fixture
            .bare_command()
            .args(["device", "byte-order-probe", "1.1.67", "--project"])
            .arg(&fixture.project),
    );
    failed(&output, "--gateway");
    trap.silent();
    fixture.intact();
}

#[test]
fn ordinary_preview_without_selector_remains_offline_even_with_legacy_nonunicast_gateway() {
    let fixture = Fixture::new();
    let trap = Trap::new();
    for endpoint in [&trap.endpoint, "0.0.0.0:0"] {
        let output = collect(fixture.command(endpoint, None).arg("--preview"));
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert!(stdout(&output).contains("Preview only"));
        assert!(!stderr(&output).contains("connected to"));
        trap.silent();
        fixture.intact();
    }
}

#[test]
fn project_and_product_refusals_preserve_current_older_and_default_stores_without_contact() {
    let fixture = Fixture::new();
    let trap = Trap::new();
    let missing = fixture.dir.path().join("absent.knxdb");
    let output = collect(
        fixture
            .bare_command()
            .args(["device", "byte-order-probe", "1.1.67", "--project"])
            .arg(&missing)
            .args(["--gateway", &trap.endpoint, "--tunnel-transport", "tcp"]),
    );
    failed(&output, "project not found");
    for target in ["1.1.68", "1.1.250"] {
        let output = collect(
            fixture
                .bare_command()
                .args(["device", "byte-order-probe", target, "--project"])
                .arg(&fixture.project)
                .arg("--product-db")
                .arg(&fixture.products)
                .args(["--gateway", &trap.endpoint, "--tunnel-transport", "tcp"]),
        );
        failed(&output, "nothing to probe");
    }
    let refused = fixture.dir.path().join("refused-inputs");
    std::fs::create_dir(&refused).unwrap();
    let older = refused.join("older.knxdb");
    std::fs::write(
        &older,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/knx-store/tests/fixtures/native13-before-notes-working.sqlite"
        )),
    )
    .unwrap();
    let bad = refused.join("products.sqlite");
    std::fs::write(&bad, b"not sqlite").unwrap();
    let before = tree(&refused);
    for project in [&fixture.project, &older] {
        let output = collect(
            fixture
                .bare_command()
                .args(["device", "byte-order-probe", "1.1.67", "--project"])
                .arg(project)
                .arg("--product-db")
                .arg(&bad)
                .args(["--gateway", &trap.endpoint, "--tunnel-transport", "tcp"]),
        );
        failed(&output, "product");
        assert_eq!(tree(&refused), before);
    }
    let output = collect(
        fixture
            .bare_command()
            .args(["device", "byte-order-probe", "1.1.67", "--project"])
            .arg(&fixture.project)
            .args(["--gateway", &trap.endpoint, "--tunnel-transport", "tcp"]),
    );
    failed(&output, "product");
    trap.silent();
    fixture.intact();
}

#[test]
fn ambiguous_device_unknown_program_and_unsupported_live_program_refuse_without_contact() {
    use knx_core::DeviceId;
    let fixture = Fixture::new();
    let trap = Trap::new();
    for case in 0..3 {
        let dir = fixture.dir.path().join(format!("planning-{case}"));
        std::fs::create_dir(&dir).unwrap();
        let project_path = dir.join("project.knxdb");
        let products_path = dir.join("products.sqlite");
        let saved = knx_store::open_existing_read_only(&fixture.project).unwrap();
        let mut project = knx_store::load_project(&saved.conn).unwrap();
        drop(saved);
        if case == 0 {
            let mut duplicate = project.devices.get(DeviceId(1)).unwrap().clone();
            duplicate.id = DeviceId(2);
            duplicate.source.ets_id = "DEV-2".into();
            project.devices.insert(duplicate);
            project.installations[0]
                .topology
                .unassigned
                .push(DeviceId(2));
        } else if case == 1 {
            project.devices.get_mut(DeviceId(1)).unwrap().program_ref = "not-a-program".into();
        }
        let conn = knx_store::open_and_migrate(&project_path).unwrap();
        knx_store::save_project(&conn, &project).unwrap();
        drop(conn);
        let db = knx_productdb::open_and_migrate(&products_path).unwrap();
        let xml = if case == 2 {
            super::fixture::PROGRAM.replace("MV-0701", "MV-0025")
        } else {
            super::fixture::PROGRAM.into()
        };
        knx_productdb::ingest_file(&db, "Synthetic.xml", xml.as_bytes()).unwrap();
        knx_productdb::ingest_file(&db, "Hardware.xml", super::fixture::HARDWARE).unwrap();
        drop(db);
        let before = tree(&dir);
        let output = collect(
            fixture
                .bare_command()
                .args(["device", "byte-order-probe", "1.1.67", "--project"])
                .arg(&project_path)
                .arg("--product-db")
                .arg(&products_path)
                .args(["--gateway", &trap.endpoint, "--tunnel-transport", "tcp"]),
        );
        failed(&output, "nothing to probe");
        assert_eq!(tree(&dir), before);
        trap.silent();
        fixture.intact();
    }
}

#[test]
fn tcp_refusal_and_rejected_handshake_do_not_fallback_or_replay_and_keep_inputs() {
    let fixture = Fixture::new();
    let (port, udp) = super::super::refused_tcp_with_udp_trap();
    let output = collect(&mut fixture.command(&format!("127.0.0.1:{port}"), Some("tcp")));
    failed(&output, "could not connect");
    udp.set_nonblocking(true).unwrap();
    no_udp(&udp);
    let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = tcp.local_addr().unwrap();
    tcp.set_nonblocking(true).unwrap();
    let udp = UdpSocket::bind(addr).unwrap();
    udp.set_nonblocking(true).unwrap();
    let worker = std::thread::spawn(move || {
        use std::io::{Read, Write};
        let until = std::time::Instant::now() + super::WAIT;
        let mut stream = loop {
            match tcp.accept() {
                Ok((s, _)) => break s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(std::time::Instant::now() < until);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("accept: {e}"),
            }
        };
        stream.set_read_timeout(Some(super::WAIT)).unwrap();
        stream.set_write_timeout(Some(super::WAIT)).unwrap();
        assert_eq!(
            super::super::read_frame(&mut stream),
            super::super::TCP_CONNECT_REQUEST
        );
        stream.write_all(&[6, 0x10, 2, 6, 0, 8, 0, 0x24]).unwrap();
        let mut tail = Vec::new();
        stream.read_to_end(&mut tail).unwrap();
        assert!(tail.is_empty());
        assert_eq!(
            tcp.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    });
    let worker = super::Joined::new(worker);
    let output = collect(&mut fixture.command(&addr.to_string(), Some("tcp")));
    worker.join().unwrap();
    failed(&output, "could not connect");
    no_udp(&udp);
    fixture.intact();
}

#[test]
fn compare_and_diagnostics_do_not_gain_the_probe_transport_flag() {
    let fixture = Fixture::new();
    let trap = Trap::new();
    for command in ["compare", "diagnostics"] {
        let mut cmd = fixture.bare_command();
        cmd.args(["device", command, "--tunnel-transport", "tcp"]);
        let output = collect(&mut cmd);
        failed(&output, "--tunnel-transport");
        assert!(stderr(&output)
            .lines()
            .next()
            .unwrap()
            .contains("--tunnel-transport"));
        trap.silent();
        fixture.intact();
    }
}
