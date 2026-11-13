//! Drive `device system2-project` observation over TCP with synthetic saved inputs and a simulator.

use super::{
    probe::{
        collect,
        fixture::tree,
        peer::{gateway, no_retry},
    },
    service_control::{connected, exit, rejected, udp_controls},
    stderr, stdout,
};
use knx_app::system2_validation_preparation::{
    admit_current_project_hardware, prepare_retained_project_hardware_source, RecoveryTrack,
};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice};
use std::{
    net::{Ipv4Addr, TcpListener, UdpSocket},
    path::PathBuf,
    process::Command,
    sync::Arc,
};

#[allow(dead_code)]
#[path = "../../../../../crates/knx-app/tests/it/system2_validation_preparation/fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "../../../../../crates/knx-app/tests/it/system2_validation_preparation/old_device.rs"]
mod old_device;

const PHRASE: &str = "I confirm read-only System-2 observation to 1.2.52";

struct Inputs {
    dir: tempfile::TempDir,
    project: PathBuf,
    products: PathBuf,
    device: Arc<SimulatedDevice>,
    confirmation: String,
    before: Vec<(String, Vec<u8>)>,
}

/// Fictional source inputs saved through the production APIs, then reopened by the CLI.
fn inputs() -> Inputs {
    let (_source_dir, conn, project, _profile) = fixture::configured_inputs();
    let dir = tempfile::tempdir().unwrap();
    let saved = dir.path().join("inputs");
    std::fs::create_dir(&saved).unwrap();
    let project_path = saved.join("project.knxdb");
    let products = saved.join("products.sqlite");
    let native = knx_store::open_and_migrate(&project_path).unwrap();
    knx_store::save_project(&native, &project).unwrap();
    drop(native);
    conn.execute("VACUUM INTO ?1", [products.to_str().unwrap()])
        .unwrap();
    let source = prepare_retained_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        "cli-tcp-observation",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    let preview = source.preview_summary().unwrap();
    assert_eq!(
        format!(
            "I confirm read-only System-2 observation to {}",
            preview.address
        ),
        PHRASE
    );
    let mut identity = source.preview().clone();
    identity.binding.serial = [0, 1, 2, 3, 4, 5];
    let device = old_device::synthetic_old_device(&identity, Default::default());
    old_device::configure_old_device(&device);
    Inputs {
        before: tree(&saved),
        confirmation: preview.confirmation_phrase,
        dir,
        project: project_path,
        products,
        device: Arc::new(device),
    }
}

impl Inputs {
    fn bare(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_knx"));
        for (key, leaf) in [
            ("HOME", "home"),
            ("XDG_DATA_HOME", "data"),
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_RUNTIME_DIR", "runtime"),
        ] {
            let path = self.dir.path().join(leaf);
            std::fs::create_dir_all(&path).unwrap();
            cmd.env(key, path);
        }
        cmd
    }
    fn output(&self) -> PathBuf {
        self.dir.path().join("out").join("prepared.json")
    }
    fn command(&self, gateway: &str, transport: Option<&str>) -> Command {
        let mut cmd = self.bare();
        cmd.args(["device", "system2-project", "--project"])
            .arg(&self.project)
            .args(["--device-id", "1", "--product-db"])
            .arg(&self.products)
            .args([
                "--operation-id",
                "cli-tcp-observation",
                "--source-mode",
                "retained-sources",
                "--gateway",
                gateway,
                "--observe-confirm",
                PHRASE,
                "--output",
            ])
            .arg(self.output());
        if let Some(transport) = transport {
            cmd.args(["--tunnel-transport", transport]);
        }
        cmd
    }
    fn intact(&self) {
        assert_eq!(tree(&self.dir.path().join("inputs")), self.before);
        assert!(!self.output().exists(), "no bundle may exist");
        let state = self.dir.path().join("data");
        assert!(tree(&state).is_empty(), "no per-user state/memory admitted");
    }
    fn assert_read_only(&self) {
        for seen in self.device.seen() {
            assert!(
                matches!(
                    seen,
                    Seen::Connect
                        | Seen::Disconnect
                        | Seen::PropertyRead { .. }
                        | Seen::MemoryRead { .. }
                ) || matches!(&seen, Seen::Other(name) if name.ends_with("Read")),
                "observation must only read: {seen:?}"
            );
        }
    }
}

#[test]
fn tcp_observation_writes_a_reopenable_prepared_bundle_and_discloses_skips() {
    for skips in [false, true] {
        let f = inputs();
        let worker_device = Arc::clone(&f.device);
        let (addr, worker, trap) = gateway(250, move |peer| {
            peer.simulate(&worker_device, skips, |_| {});
        });
        std::fs::create_dir(f.dir.path().join("out")).unwrap();
        let before = f.device.system2_snapshot();
        let output = collect(&mut f.command(&addr.to_string(), Some("tcp")));
        worker.join().unwrap();
        exit(&output, 0);
        connected(&output, &addr.to_string(), skips);
        no_retry(&trap);
        assert!(
            stdout(&output).contains("NO DEVICE WRITE"),
            "{}",
            stdout(&output)
        );
        assert_eq!(
            f.device.system2_snapshot(),
            before,
            "observation is read-only"
        );
        f.assert_read_only();
        let bundle = knx_app::property_validation::read_bundle(&f.output()).unwrap();
        assert!(!bundle.binding.simulated);
        assert_eq!(bundle.binding.target, "1.2.52");
        assert_eq!(bundle.binding.serial, [0, 1, 2, 3, 4, 5]);
        assert_eq!(bundle.consent_phrase(), f.confirmation);
        let project =
            knx_store::load_project(&knx_store::open_existing_read_only(&f.project).unwrap().conn)
                .unwrap();
        let products = knx_productdb::open_read_only(&f.products).unwrap();
        admit_current_project_hardware(&products.conn, &project, knx_core::DeviceId(1), &bundle)
            .expect("reopened bundle admits against the unchanged saved inputs");
        assert_eq!(tree(&f.dir.path().join("inputs")), f.before);
    }
}

#[test]
fn tcp_eof_fails_without_a_bundle_discloses_skips_and_never_retries() {
    let f = inputs();
    let (addr, worker, trap) = gateway(250, |peer| {
        peer.target("1.2.52");
        peer.begin(false);
        peer.property_request(0, 11);
        peer.skips();
        peer.close_write();
    });
    std::fs::create_dir(f.dir.path().join("out")).unwrap();
    let output = collect(&mut f.command(&addr.to_string(), Some("tcp")));
    worker.join().unwrap();
    exit(&output, 1);
    connected(&output, &addr.to_string(), true);
    assert!(
        !stdout(&output).contains("retained locally"),
        "{}",
        stdout(&output)
    );
    no_retry(&trap);
    f.intact();
}

#[test]
fn tcp_rejection_never_falls_back_and_udp_controls_are_handshake_only() {
    let f = inputs();
    rejected(|addr| f.command(addr, Some("tcp")));
    udp_controls(|addr, transport| f.command(addr, transport));
    f.intact();
}

#[test]
fn consent_and_overwrite_gates_hold_before_any_connection() {
    let f = inputs();
    let tcp = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    tcp.set_nonblocking(true).unwrap();
    let addr = tcp.local_addr().unwrap();
    let udp = UdpSocket::bind(addr).unwrap();
    udp.set_nonblocking(true).unwrap();
    let wrong = f.command(&addr.to_string(), Some("tcp"));
    let args: Vec<_> = wrong.get_args().map(|a| a.to_os_string()).collect();
    let mut wrong = f.bare();
    let at = args.iter().position(|a| a == PHRASE).unwrap();
    wrong.args(&args[..at]).arg("wrong").args(&args[at + 1..]);
    let out = collect(&mut wrong);
    exit(&out, 1);
    assert!(stderr(&out).contains("exact read-only target observation confirmation required"));
    std::fs::create_dir(f.dir.path().join("out")).unwrap();
    std::fs::write(f.output(), b"keep me").unwrap();
    let out = collect(&mut f.command(&addr.to_string(), Some("tcp")));
    exit(&out, 1);
    assert!(stderr(&out).contains("output exists"), "{}", stderr(&out));
    assert_eq!(std::fs::read(f.output()).unwrap(), b"keep me");
    assert_eq!(
        tcp.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    super::probe::peer::no_udp(&udp);
    assert!(f.device.seen().is_empty());
}

#[test]
fn strict_parsing_preview_and_invalid_gateway_do_no_io() {
    let f = inputs();
    let tcp = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    tcp.set_nonblocking(true).unwrap();
    let addr = tcp.local_addr().unwrap().to_string();
    let missing = f.dir.path().join("missing.knxdb");
    let out_file = f.dir.path().join("never.json");
    let base = |extra: &[&str]| {
        let mut cmd = f.bare();
        cmd.args(["device", "system2-project", "--project"])
            .arg(&missing)
            .args(["--device-id", "1", "--product-db"])
            .arg(f.dir.path().join("missing.sqlite"))
            .args(["--operation-id", "x", "--source-mode", "retained-sources"])
            .args(extra);
        collect(&mut cmd)
    };
    let confirmed = ["--gateway", addr.as_str(), "--observe-confirm", PHRASE];
    let mut cases: Vec<(Vec<&str>, &str)> = vec![
        (vec!["--tunnel-transport"], "needs a value"),
        (vec!["--tunnel-transport", "--output", "x"], "needs a value"),
        (vec!["--tunnel-transport", "tls"], "must be udp or tcp"),
        (vec!["--tunnel-transport", "TCP"], "must be udp or tcp"),
        (vec!["--tunnel-transport", ""], "must be udp or tcp"),
        (
            vec!["--tunnel-transport", "tcp", "--tunnel-transport", "tcp"],
            "duplicate --tunnel-transport",
        ),
        (
            vec!["--tunnel-transport", "tcp", "--tunnel-transport", "udp"],
            "duplicate --tunnel-transport",
        ),
        (vec!["--tunnel-transport", "tcp"], "offline preview"),
        (vec!["--tunnel-transport", "udp"], "offline preview"),
        (
            vec!["--gateway", &addr, "--tunnel-transport", "tcp"],
            "offline preview",
        ),
    ];
    for gateway in [
        "bad",
        "0.0.0.0:3671",
        "224.0.23.12:3671",
        "255.255.255.255:3671",
        "127.0.0.1:0",
        "[::1]:3671",
    ] {
        cases.push((
            vec![
                "--gateway",
                gateway,
                "--observe-confirm",
                PHRASE,
                "--tunnel-transport",
                "tcp",
            ],
            "--gateway",
        ));
    }
    for (extra, expected) in cases {
        let out = base(&extra);
        exit(&out, 1);
        assert!(
            stderr(&out).contains(expected),
            "{extra:?}: {}",
            stderr(&out)
        );
        assert!(
            !stderr(&out).contains("missing.knxdb"),
            "{extra:?}: {}",
            stderr(&out)
        );
        assert!(!stdout(&out).contains("OFFLINE PREVIEW"), "{extra:?}");
    }
    // Control: the same inputs without a transport still reach the (missing) project.
    let mut args = confirmed.to_vec();
    args.extend(["--output", out_file.to_str().unwrap()]);
    let out = base(&args);
    exit(&out, 1);
    assert!(!stderr(&out).contains("must be udp or tcp"));
    assert!(!out_file.exists() && !missing.exists());
    assert_eq!(
        tcp.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    f.intact();
}

#[test]
fn offline_preview_without_transport_still_connects_nothing() {
    let f = inputs();
    let out = collect(
        f.bare()
            .args(["device", "system2-project", "--project"])
            .arg(&f.project)
            .args(["--device-id", "1", "--product-db"])
            .arg(&f.products)
            .args(["--operation-id", "x", "--source-mode", "retained-sources"]),
    );
    exit(&out, 0);
    assert!(stdout(&out).contains("OFFLINE PREVIEW ONLY"));
    assert!(f.device.seen().is_empty());
    f.intact();
}
