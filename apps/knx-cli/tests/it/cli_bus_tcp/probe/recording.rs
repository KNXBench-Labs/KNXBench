//! Verify consented probe recordings retain actual wire with redaction and sampled-skip metadata.

use super::{
    collect, failed,
    fixture::Fixture,
    peer::{gateway, no_udp, Wire},
    result, stderr,
};
use knx_net::{ApplicationService, Tpci};
use serde_json::Value;
use std::{io::Read, path::Path};

fn recording(bundle: &Path) -> (Value, String, Vec<Value>) {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(bundle).unwrap()).unwrap();
    let mut manifest = String::new();
    zip.by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let manifest: Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(manifest["problemType"], "deviceReading");
    let sections: Vec<_> = manifest["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["kind"] == "operation")
        .collect();
    assert_eq!(sections.len(), 1);
    assert_eq!(
        sections[0]["notes"],
        serde_json::json!([knx_app::diagnostic_bundle::PRIVATE_RAW_NOTE])
    );
    let id = sections[0]["label"].as_str().unwrap();
    let mut text = String::new();
    zip.by_name(&format!("operations/{id}/recording.jsonl"))
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    let lines = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    (manifest, text, lines)
}

fn exact_wire(lines: &[Value], wire: &Wire) {
    for (direction, from_tool) in [("sent", true), ("received", false)] {
        let recorded: Vec<_> = lines
            .iter()
            .filter(|l| l["type"] == "datagram" && l["dir"] == direction)
            .map(|l| l["raw"].as_str().unwrap().to_string())
            .collect();
        let expected: Vec<_> = wire
            .iter()
            .filter(|(sent, frame)| {
                *sent == from_tool && frame.len() < 1500 && frame[2..4] != [9, 0x99]
            })
            .map(|(_, frame)| {
                let mut bytes = frame.clone();
                if bytes.ends_with(&[0x11, 0x22, 0x33, 0x44]) {
                    let length = bytes.len();
                    bytes[length - 4..].fill(0);
                }
                bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            })
            .collect();
        assert_eq!(
            recorded, expected,
            "actual {direction} TCP frames, not reconstructed telemetry"
        );
    }
}

#[test]
fn tcp_bundle_records_byte_order_probe_identity_raw_reads_redacted_keys_and_skip_metadata() {
    let fixture = Fixture::new();
    let out = fixture.dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let bundle = out.join("report.zip");
    let (addr, worker, trap) = gateway(250, |peer| {
        peer.identity();
        peer.memory_request();
        peer.skips();
        peer.indicate(
            Tpci::UnnumberedData,
            ApplicationService::AuthorizeRequest {
                key: [0x11, 0x22, 0x33, 0x44],
            },
        );
        peer.memory_answer(vec![0x2c, 1, 2, 1]);
        peer.control(7);
        peer.management_end();
        peer.finish();
    });
    let endpoint = addr.to_string();
    let output = collect(
        fixture
            .command(&endpoint, Some("tcp"))
            .arg("--diagnostic-bundle")
            .arg(&bundle),
    );
    let wire = worker.join().unwrap();
    result(
        &output,
        "2 low octet first, 0 high octet first, 0 neither",
        "1 agree, 0 disagree",
        "TCP",
    );
    assert!(
        stderr(&output).contains("may contain device configuration and passwords: keep it private")
    );
    let (_, text, lines) = recording(&bundle);
    assert_eq!(lines[0]["operation_kind"], "byte-order-probe");
    assert_eq!(lines[0]["gateway"], endpoint);
    assert_eq!(lines.last().unwrap()["outcome"], "succeeded");
    assert_eq!(lines.last().unwrap()["records_dropped"], 0);
    assert!(lines
        .iter()
        .any(|l| l["event"] == "connected" && l["channel"] == super::super::CHANNEL));
    assert!(lines.iter().any(|l| !l["redacted"].is_null()));
    assert!(!text.contains("11223344"));
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
    assert!(
        text.contains("2c010201"),
        "consented raw device data is intentionally retained, not metadata history"
    );
    exact_wire(&lines, &wire);
    assert_eq!(
        std::fs::read_dir(&out)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>(),
        ["report.zip"]
    );
    no_udp(&trap);
    fixture.intact();
}

#[test]
fn settled_read_refusal_and_eof_finalize_failed_recordings_without_temporary_directories() {
    for eof in [false, true] {
        let fixture = Fixture::new();
        let out = fixture.dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let bundle = out.join("report.zip");
        let (addr, worker, trap) = gateway(250, move |peer| {
            peer.identity();
            peer.memory_request();
            peer.skips();
            if eof {
                peer.close_write();
            } else {
                peer.memory_answer(vec![]);
                peer.management_end();
                peer.finish();
            }
        });
        let output = collect(
            fixture
                .command(&addr.to_string(), Some("tcp"))
                .arg("--diagnostic-bundle")
                .arg(&bundle),
        );
        let wire = worker.join().unwrap();
        failed(&output, "not probed:");
        assert!(stderr(&output).contains("2 TCP frame(s) skipped before disconnecting"));
        let (_, text, lines) = recording(&bundle);
        assert_eq!(lines[0]["operation_kind"], "byte-order-probe");
        assert_eq!(lines.last().unwrap()["outcome"], "failed");
        assert_eq!(lines.last().unwrap()["records_dropped"], 0);
        assert_eq!(
            lines
                .iter()
                .filter(|l| l["event"] == "frame-skipped")
                .count(),
            2
        );
        assert!(!text.contains("deadbeef"));
        assert!(!text.contains("eeeeeeee"));
        exact_wire(&lines, &wire);
        assert_eq!(
            std::fs::read_dir(&out)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect::<Vec<_>>(),
            ["report.zip"]
        );
        no_udp(&trap);
        fixture.intact();
    }
}

#[test]
fn existing_bundle_destination_refuses_before_contact_and_preserves_its_bytes() {
    let fixture = Fixture::new();
    let tcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    tcp.set_nonblocking(true).unwrap();
    let addr = tcp.local_addr().unwrap();
    let udp = std::net::UdpSocket::bind(addr).unwrap();
    udp.set_nonblocking(true).unwrap();
    let bundle = fixture.dir.path().join("existing.zip");
    std::fs::write(&bundle, b"preserve existing private report").unwrap();
    let output = collect(
        fixture
            .command(&addr.to_string(), Some("tcp"))
            .arg("--diagnostic-bundle")
            .arg(&bundle),
    );
    failed(&output, "exists");
    assert_eq!(
        std::fs::read(&bundle).unwrap(),
        b"preserve existing private report"
    );
    assert_eq!(
        tcp.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    no_udp(&udp);
    fixture.intact();
}
