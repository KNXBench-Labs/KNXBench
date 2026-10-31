//! DIAG-03: built CLI bundle lifetime and raw-tunnel withholding on loopback.
//!
//! The built `knx` binary runs `device find-serial` against
//! `knx_net::commissioning::simulator_gateway` on 127.0.0.1. The bundle's
//! recording must hold exactly the datagrams the gateway saw, per direction,
//! with the keep-private note (owner decision 31 October 2026: the flag is the
//! consent for this one bundle); the temporary recording directory must be
//! gone afterwards. No real gateway, bus or device is involved.

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

fn knx(dir: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_knx"));
    command
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_RUNTIME_DIR", dir.join("runtime"));
    command
}

#[test]
fn a_find_serial_run_cleans_up_and_exports_its_recording_byte_exactly() {
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
    // The consent text must not promise less than the ZIP holds.
    assert!(
        stderr.contains("may contain device configuration and passwords: keep it private"),
        "{stderr}"
    );
    assert!(!stderr.contains("never included"), "{stderr}");

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
    let seen = seen.lock().unwrap().clone();
    assert!(seen.iter().any(|(from_tool, _)| *from_tool));
    assert!(seen.iter().any(|(from_tool, _)| !*from_tool));
    assert!(!archive.file_names().any(|name| {
        name.ends_with("system2-private.json") || name.ends_with("system2-report.json")
    }));

    // Exactly one operation section: the raw recording, as the wire saw it.
    let sections = manifest["sections"].as_array().unwrap();
    let kinds: Vec<&str> = sections
        .iter()
        .map(|s| s["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["environment", "operation"]);
    assert!(
        sections[0]["notes"].as_array().unwrap().is_empty(),
        "{}",
        sections[0]
    );
    let operation = &sections[1];
    assert_eq!(
        operation["notes"],
        serde_json::json!([knx_app::diagnostic_bundle::PRIVATE_RAW_NOTE])
    );
    let id = operation["label"].as_str().unwrap();
    let mut text = String::new();
    archive
        .by_name(&format!("operations/{id}/recording.jsonl"))
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    let lines: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines[0]["operation_kind"], "serial-number-address");
    assert_eq!(lines[0]["writer"]["program"], "knx-cli");
    assert_eq!(lines[0]["gateway"], endpoint);
    let hex = |bytes: &[u8]| -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() };
    for (dir, from_tool) in [("sent", true), ("received", false)] {
        let recorded: Vec<&str> = lines
            .iter()
            .filter(|l| l["type"] == "datagram" && l["dir"] == dir)
            .map(|l| l["raw"].as_str().unwrap())
            .collect();
        let wire: Vec<String> = seen
            .iter()
            .filter(|(from, _)| *from == from_tool)
            .map(|(_, bytes)| hex(bytes))
            .collect();
        assert!(!recorded.is_empty());
        assert_eq!(recorded, wire, "{dir}");
    }
    let end = lines.last().unwrap();
    assert_eq!(end["type"], "end");
    assert_eq!(end["outcome"], "succeeded");
    assert_eq!(end["records_dropped"], 0);
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
