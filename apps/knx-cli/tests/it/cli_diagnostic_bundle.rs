//! DIAG-03: `--diagnostic-bundle` records a real CLI tunnel byte for byte into one ZIP.
//!
//! The built `knx` binary runs `device find-serial` against
//! `knx_net::commissioning::simulator_gateway` on 127.0.0.1. The bundle's
//! recording must hold exactly the datagrams the gateway saw, per direction;
//! the temporary recording directory must be gone afterwards. No real
//! gateway, bus or device is involved.

use std::collections::BTreeMap;
use std::io::Read;
use std::net::{SocketAddr, UdpSocket as StdSocket};
use std::process::Command;
use std::sync::{Arc, Mutex};

use knx_core::IndividualAddress;
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use knx_net::commissioning::simulator_gateway::{serve, GatewayConfig, GatewayEvent};
use serde_json::Value;

const SERIAL_OCTETS: [u8; 6] = [0x00, 0x83, 0x12, 0x34, 0x56, 0x78];

type Seen = Arc<Mutex<Vec<(bool, Vec<u8>)>>>;

/// Serves a simulated device on a loopback port; returns its endpoint and log.
fn gateway() -> (String, Seen) {
    let probe = StdSocket::bind("127.0.0.1:0").unwrap();
    let address = probe.local_addr().unwrap();
    let SocketAddr::V4(advertised) = address else {
        unreachable!("bound to IPv4")
    };
    let seen: Seen = Arc::default();
    let log = Arc::clone(&seen);
    let (ready, started) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            probe.set_nonblocking(true).unwrap();
            let socket = tokio::net::UdpSocket::from_std(probe).unwrap();
            let device = SimulatedDevice::with_config(SimulatorConfig {
                serial_number: Some(SERIAL_OCTETS),
                programming_mode: false,
                ..Default::default()
            });
            let mut config = GatewayConfig::new(advertised, IndividualAddress::from_raw(0x11fe));
            config.properties = BTreeMap::new();
            config.observer = Some(Arc::new(move |event: GatewayEvent<'_>| {
                if let GatewayEvent::Datagram { from_tool, bytes } = event {
                    log.lock().unwrap().push((from_tool, bytes.to_vec()));
                }
            }));
            ready.send(()).unwrap();
            let _ = serve(Arc::new(device), socket, config).await;
        });
    });
    started.recv().unwrap();
    (address.to_string(), seen)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn knx(dir: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_knx"));
    command
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_RUNTIME_DIR", dir.join("runtime"));
    command
}

#[test]
fn a_find_serial_run_leaves_one_bundle_with_the_exact_tunnel() {
    let dir = tempfile::tempdir().unwrap();
    let (endpoint, seen) = gateway();
    let bundle = dir.path().join("out").join("report.zip");
    std::fs::create_dir_all(bundle.parent().unwrap()).unwrap();
    let output = knx(dir.path())
        .args([
            "device",
            "find-serial",
            "0083:12345678",
            "--diagnostic-bundle",
            bundle.to_str().unwrap(),
            "--gateway",
            &endpoint,
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("diagnostic bundle written to"), "{stderr}");

    // Only the bundle remains beside it; the recording directory is gone.
    let left: Vec<_> = std::fs::read_dir(bundle.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(left, ["report.zip"]);

    let mut archive = zip::ZipArchive::new(std::fs::File::open(&bundle).unwrap()).unwrap();
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let manifest: Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(manifest["problemType"], "deviceReading");
    assert_eq!(manifest["writer"]["program"], "knx-cli");
    let recording_name = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .find(|name| name.ends_with("/recording.jsonl"))
        .expect("a recording in the bundle");
    let mut recording = String::new();
    archive
        .by_name(&recording_name)
        .unwrap()
        .read_to_string(&mut recording)
        .unwrap();
    let lines: Vec<Value> = recording
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.first().unwrap()["type"], "header");
    assert_eq!(lines.last().unwrap()["type"], "end");
    let recorded = |direction: &str| -> Vec<String> {
        lines
            .iter()
            .filter(|l| l["type"] == "datagram" && l["dir"] == direction)
            .map(|l| l["raw"].as_str().unwrap().to_string())
            .collect()
    };
    let seen = seen.lock().unwrap().clone();
    let observed = |from_tool: bool| -> Vec<String> {
        seen.iter()
            .filter(|(tool, _)| *tool == from_tool)
            .map(|(_, bytes)| hex(bytes))
            .collect()
    };
    assert!(!observed(true).is_empty());
    assert_eq!(recorded("sent"), observed(true), "tool → gateway");
    assert_eq!(recorded("received"), observed(false), "gateway → tool");
    assert_eq!(lines.last().unwrap()["records_dropped"], 0);
}

#[test]
fn an_existing_bundle_file_is_refused_before_any_connection() {
    let dir = tempfile::tempdir().unwrap();
    let (endpoint, seen) = gateway();
    let bundle = dir.path().join("taken.zip");
    std::fs::write(&bundle, b"keep me").unwrap();
    let output = knx(dir.path())
        .args([
            "device",
            "find-serial",
            "0083:12345678",
            "--gateway",
            &endpoint,
            "--diagnostic-bundle",
            bundle.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
    assert_eq!(std::fs::read(&bundle).unwrap(), b"keep me");
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn the_flag_is_refused_outside_device_and_bus_commands() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("x.zip");
    let output = knx(dir.path())
        .args(["--version", "--diagnostic-bundle", bundle.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!bundle.exists());
}
