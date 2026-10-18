//! Format-2 diagnostic archives: declared memory evidence, strict reader, format 1 intact.
use knx_app::device_diagnostics::{
    acquire_memory, bundle, bundle_export, pending, prepare, prepare_memory, read_bundle,
    read_export, write_new_export, Export, MEMORY_PROFILE,
};
use knx_core::{DeviceId, DeviceInstance, Language, Project, SourceRef};
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use std::io::{Cursor, Read, Write};
use std::sync::atomic::AtomicBool;

/// A synthetic System-2 program in the exact `project_tool_metadata` shape:
/// a PID78 comparison and two absolute segments (30 + 12 octets).
const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1"
ApplicationVersion="1" MaskVersion="MV-0025" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="OBJ" Address="1024" Size="8" /></Code>
<ComObjectTable CodeSegment="OBJ" Offset="0" />
<LoadProcedures><LoadProcedure><LdCtrlConnect />
<LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="00010000005B00000000" />
<LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="278" Size="30" Access="51" MemType="3" SegFlags="0" />
<LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="15360" Size="12" Access="51" MemType="3" SegFlags="0" />
<LdCtrlTaskCtrl2 LsmIdx="3" Callback="0" Address="1024" Seg0="0" Seg1="0" />
<LdCtrlTaskSegment LsmIdx="5" Address="1024" /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001">
<Hardware><Hardware Id="H" Name="Synthetic" SerialNumber="synthetic" VersionNumber="1">
<Products><Product Id="M-0001_H-1_P-1" /></Products><Hardware2Programs><Hardware2Program Id="H_HP" MediumTypes="MT-0">
<ApplicationProgramRef RefId="M-0001_A-0001-01-0000" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn products(program: &str) -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let db = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&db, "synthetic.xml", program.as_bytes()).unwrap();
    knx_productdb::ingest_file(&db, "Hardware.xml", HARDWARE).unwrap();
    (dir, db)
}

fn project() -> Project {
    project_with("H_HP")
}

fn project_with(program_ref: &str) -> Project {
    let mut p = Project::new(Language("en".into()));
    p.devices.insert(DeviceInstance {
        id: DeviceId(7),
        source: SourceRef {
            path: "/home/alice/SECRET".into(),
            ets_id: "SECRET_SOURCE".into(),
        },
        name: "SECRET_ROOM".into(),
        description: None,
        address: Some("1.1.25".parse().unwrap()),
        product_ref: "M-0001_H-1_P-1".into(),
        program_ref: program_ref.into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    p
}

fn device() -> SimulatedDevice {
    let d = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        ..Default::default()
    });
    d.preset_property(0, 12, &[0x00, 0x01]);
    d.preset_property(0, 78, &[0, 1, 0, 0, 0, 0x5B]);
    d.preset_memory(0x0116, &(1..=30).collect::<Vec<u8>>());
    d.preset_memory(0x3C00, &[0xEE; 12]);
    d
}

async fn export() -> Export {
    let (_dir, db) = products(PROGRAM);
    let plan = prepare_memory(&project(), DeviceId(7), &db).unwrap();
    acquire_memory(
        &device(),
        &plan,
        Default::default(),
        "synthetic-test",
        &AtomicBool::new(false),
        |_| {},
    )
    .await
}

/// Rewrites an archive member by member, with `edit` deciding each member.
fn rewrite(original: &[u8], mut edit: impl FnMut(&str, Vec<u8>) -> Option<Vec<u8>>) -> Vec<u8> {
    let mut input = zip::ZipArchive::new(Cursor::new(original)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(vec![]));
    for i in 0..input.len() {
        let mut f = input.by_index(i).unwrap();
        let name = f.name().to_string();
        let mut data = vec![];
        f.read_to_end(&mut data).unwrap();
        if let Some(data) = edit(&name, data) {
            output
                .start_file(&name, zip::write::SimpleFileOptions::default())
                .unwrap();
            output.write_all(&data).unwrap();
        }
    }
    output.finish().unwrap().into_inner()
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    format!("{:x}", sha2::Sha256::digest(bytes))
}

#[test]
fn the_profile_is_derived_from_product_data_with_required_identity() {
    let (_dir, db) = products(PROGRAM);
    let plan = prepare_memory(&project(), DeviceId(7), &db).unwrap();
    assert_eq!(plan.base().expected.mask, Some(0x0025));
    assert_eq!(plan.base().expected.manufacturer, Some(0x0001));
    assert_eq!(plan.profile().program_id(), "M-0001_A-0001-01-0000");
    assert_eq!(plan.profile().ranges().total_octets(), 42);
    assert_eq!(plan.profile().range_positions(), [2, 3]);
    // Not the System-2 shape: refused before any adapter exists.
    let (_dir, other) = products(&PROGRAM.replace("MV-0025", "MV-0701"));
    let error = prepare_memory(&project(), DeviceId(7), &other).unwrap_err();
    assert!(error.contains("diagnostics-profile-unavailable"), "{error}");
    // No product mapping for the device: refused.
    let error = prepare_memory(&project_with("H_OTHER"), DeviceId(7), &db).unwrap_err();
    assert!(error.contains("diagnostics-program-unresolved"), "{error}");
}

#[tokio::test]
async fn a_format_2_export_round_trips_with_memory_evidence_and_coverage() {
    let export = export().await;
    let report = &export.report;
    assert_eq!(report.format, 2);
    assert_eq!(report.profile, MEMORY_PROFILE);
    assert_eq!(report.memory_status, "requested");
    assert!(report.simulated && !report.written && report.complete);
    assert_eq!(report.stop_reason, None);
    let memory = export.memory.as_ref().unwrap();
    assert_eq!(memory.program_id, "M-0001_A-0001-01-0000");
    assert_eq!(memory.chunk_octets, 12);
    assert_eq!(memory.chunks.len(), 4);
    assert_eq!(memory.chunks[0].bytes, "0102030405060708090a0b0c");
    assert_eq!(memory.ranges[0].procedure_position, 2);
    assert_eq!(memory.ranges[0].coverage.read, 30);
    assert_eq!(memory.ranges[1].coverage.read, 12);
    assert_eq!(
        memory.hardware_type_comparison.inline_data,
        "00010000005b00000000"
    );
    let bytes = bundle_export(&export).unwrap();
    assert_eq!(read_export(&bytes).unwrap(), export);
    // A format-2 archive is never silently reduced to its identity part.
    assert!(read_bundle(&bytes).is_err());
    let names: Vec<String> = zip::ZipArchive::new(Cursor::new(&bytes))
        .unwrap()
        .file_names()
        .map(str::to_string)
        .collect();
    assert_eq!(names.len(), 5);
    assert!(names.iter().any(|n| n == "memory.json"));
    let text = |name: &str| {
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut s = String::new();
        zip.by_name(name).unwrap().read_to_string(&mut s).unwrap();
        s
    };
    for name in ["README.txt", "manifest.json"] {
        let content = text(name);
        for phrase in [
            "not proven runtime addresses",
            "not a backup",
            "not a restore basis",
            "keep private",
        ] {
            assert!(content.contains(phrase), "{name} lacks {phrase}: {content}");
        }
    }
    for secret in ["SECRET", "/home/", "1.1.25"] {
        for name in ["report.json", "memory.json", "manifest.json"] {
            assert!(!text(name).contains(secret), "{name} leaked {secret}");
        }
    }
}

#[tokio::test]
async fn the_writer_is_create_only_owner_only_and_rereads() {
    let export = export().await;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory-evidence.zip");
    write_new_export(&path, &export).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let written = std::fs::read(&path).unwrap();
    assert_eq!(read_export(&written).unwrap(), export);
    assert!(
        write_new_export(&path, &export).is_err(),
        "never overwrites"
    );
    assert_eq!(std::fs::read(&path).unwrap(), written);
}

#[tokio::test]
async fn the_strict_reader_refuses_missing_extra_and_tampered_memory_evidence() {
    let original = bundle_export(&export().await).unwrap();
    let missing = rewrite(&original, |name, data| {
        (name != "memory.json").then_some(data)
    });
    assert!(
        read_export(&missing).is_err(),
        "accepted missing memory.json"
    );
    let mut extra = rewrite(&original, |_, data| Some(data));
    {
        let mut zip = zip::ZipWriter::new_append(Cursor::new(&mut extra)).unwrap();
        zip.start_file("dump.bin", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&[0; 4]).unwrap();
        zip.finish().unwrap();
    }
    assert!(read_export(&extra).is_err(), "accepted an extra member");
    let tampered = rewrite(&original, |name, mut data| {
        if name == "memory.json" {
            data.push(b' ');
        }
        Some(data)
    });
    assert!(read_export(&tampered).is_err(), "accepted a hash mismatch");
    // Even with a matching hash, a chunk claimed read with other bytes, or a
    // coverage that does not follow from the chunks, is refused.
    for (from, to) in [
        ("0102030405060708090a0b0c", "0102030405060708090a0b"),
        ("\"read\": 30", "\"read\": 29"),
        ("\"status\": \"read\"", "\"status\": \"not-read\""),
    ] {
        let mut memory = vec![];
        let edited = rewrite(&original, |name, data| {
            if name == "memory.json" {
                let text = String::from_utf8(data).unwrap();
                assert!(text.contains(from), "fixture lacks {from}");
                memory = text.replacen(from, to, 1).into_bytes();
                Some(memory.clone())
            } else {
                Some(data)
            }
        });
        let fixed = rewrite(&edited, |name, data| {
            if name == "manifest.json" {
                let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
                v["files"]["memory.json"]["bytes"] = memory.len().into();
                v["files"]["memory.json"]["sha256"] = sha256(&memory).into();
                Some(serde_json::to_vec_pretty(&v).unwrap())
            } else {
                Some(data)
            }
        });
        assert!(read_export(&fixed).is_err(), "accepted {from} -> {to}");
    }
    // A format-2 archive relabelled as format 1 is refused.
    let relabelled = rewrite(&original, |name, data| {
        if name == "manifest.json" {
            let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
            v["format"] = 1.into();
            Some(serde_json::to_vec_pretty(&v).unwrap())
        } else {
            Some(data)
        }
    });
    assert!(read_export(&relabelled).is_err());
}

#[test]
fn format_1_archives_keep_four_members_and_no_memory_notice() {
    let plan = prepare(&project(), DeviceId(7), None).unwrap();
    let report = pending(&plan, "synthetic", true);
    let bytes = bundle(&report).unwrap();
    assert_eq!(zip::ZipArchive::new(Cursor::new(&bytes)).unwrap().len(), 4);
    let export = read_export(&bytes).unwrap();
    assert_eq!(export.memory, None);
    assert_eq!(read_bundle(&bytes).unwrap(), report);
    assert_eq!(
        bundle_export(&export).unwrap(),
        bytes,
        "byte-identical rewrite"
    );
    let mut manifest = String::new();
    zip::ZipArchive::new(Cursor::new(&bytes))
        .unwrap()
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    assert!(!manifest.contains("notice"));
}

/// The two declared segments of [`PROGRAM`], as they appear in it.
const SEGMENTS: &str = r#"<LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="278" Size="30" Access="51" MemType="3" SegFlags="0" />
<LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="15360" Size="12" Access="51" MemType="3" SegFlags="0" />"#;

#[tokio::test]
async fn the_largest_admitted_profile_fits_the_archive_limits() {
    // Eight declared ranges of 256 octets: the 2048-octet profile limit,
    // derived from product data and read from a device showing exactly the
    // asserted identity, so the archive is a real, consistent acquisition.
    let segments: String = (0..8u32)
        .map(|i| {
            format!(
                r#"<LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="{}" Size="256" Access="51" MemType="3" SegFlags="0" />"#,
                0x1000 + 0x400 * i
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(PROGRAM.contains(SEGMENTS), "fixture drifted");
    let (_dir, db) = products(&PROGRAM.replace(SEGMENTS, &segments));
    let plan = prepare_memory(&project(), DeviceId(7), &db).unwrap();
    assert_eq!(plan.profile().ranges().total_octets(), 2048);
    let device = device();
    for i in 0..8u32 {
        device.preset_memory(0x1000 + 0x400 * i, &[0xFF; 256]);
    }
    let export = acquire_memory(
        &device,
        &plan,
        Default::default(),
        "synthetic",
        &AtomicBool::new(false),
        |_| {},
    )
    .await;
    assert_eq!(export.report.stop_reason, None);
    let memory = export.memory.as_ref().unwrap();
    assert!(memory.chunks.iter().all(|c| c.status == "read"));
    assert!(memory.ranges.iter().all(|r| r.coverage.read == 256));
    let bytes = bundle_export(&export).unwrap();
    assert_eq!(read_export(&bytes).unwrap(), export);
}

/// Re-encodes `report.json`, `properties.json` (always the report's items)
/// and `memory.json` after `edit`, and rewrites the manifest's sizes and
/// hashes to match, so only the reader's semantic checks can refuse it.
fn rehashed(
    original: &[u8],
    edit: impl FnOnce(&mut serde_json::Value, &mut serde_json::Value),
) -> Vec<u8> {
    let member = |name: &str| {
        let mut zip = zip::ZipArchive::new(Cursor::new(original)).unwrap();
        let mut data = vec![];
        zip.by_name(name).unwrap().read_to_end(&mut data).unwrap();
        serde_json::from_slice::<serde_json::Value>(&data).unwrap()
    };
    let (mut report, mut memory) = (member("report.json"), member("memory.json"));
    edit(&mut report, &mut memory);
    let encoded = [
        ("report.json", serde_json::to_vec_pretty(&report).unwrap()),
        (
            "properties.json",
            serde_json::to_vec_pretty(&report["items"]).unwrap(),
        ),
        ("memory.json", serde_json::to_vec_pretty(&memory).unwrap()),
    ];
    let edited = rewrite(original, |name, data| {
        Some(
            encoded
                .iter()
                .find(|(n, _)| *n == name)
                .map_or(data, |(_, v)| v.clone()),
        )
    });
    rewrite(&edited, |name, data| {
        if name != "manifest.json" {
            return Some(data);
        }
        let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
        for (n, bytes) in &encoded {
            v["files"][n]["bytes"] = bytes.len().into();
            v["files"][n]["sha256"] = sha256(bytes).into();
        }
        Some(serde_json::to_vec_pretty(&v).unwrap())
    })
}

type Edit = fn(&mut serde_json::Value, &mut serde_json::Value);

/// Every named edit that the reader accepted; each edit breaks one rule.
fn accepted(original: &[u8], cases: &[(&'static str, Edit)]) -> Vec<&'static str> {
    // The unedited re-encoding is accepted, so a refusal below is the rule's.
    read_export(&rehashed(original, |_, _| {})).expect("re-encoding alone is accepted");
    cases
        .iter()
        .filter(|(_, edit)| read_export(&rehashed(original, edit)).is_ok())
        .map(|(name, _)| *name)
        .collect()
}

#[tokio::test]
async fn memory_evidence_is_refused_unless_the_recorded_identity_admits_it() {
    let original = bundle_export(&export().await).unwrap();
    let cases: &[(&'static str, Edit)] = &[
        ("descriptor differs from the asserted mask", |r, _| {
            r["items"][0]["bytes"] = serde_json::json!([7, 5]);
        }),
        ("asserted mask contradicts the descriptor", |r, _| {
            r["assertedMask"] = 0x0705.into();
        }),
        ("PID12 differs from the asserted manufacturer", |r, _| {
            r["items"][1]["bytes"] = serde_json::json!([0, 2]);
        }),
        ("asserted manufacturer contradicts PID12", |r, _| {
            r["assertedManufacturer"] = 2.into();
        }),
        ("PID78 does not match the recorded inlineData", |r, _| {
            r["items"][2]["bytes"] = serde_json::json!([0, 1, 0, 0, 0, 0x5C]);
        }),
        ("recorded inlineData does not match PID78", |_, m| {
            m["hardwareTypeComparison"]["inlineData"] = "00010000005c00000000".into();
        }),
        ("PID12 returned two elements", |r, _| {
            r["items"][1]["returnedElements"] = 2.into();
        }),
        ("PID78 returned no element count", |r, _| {
            r["items"][2]["returnedElements"] = serde_json::Value::Null;
        }),
        ("the descriptor carries an element count", |r, _| {
            r["items"][0]["returnedElements"] = 1.into();
        }),
        ("a read chunk lost its attempt time", |_, m| {
            m["chunks"][0]["attemptedAt"] = serde_json::Value::Null;
        }),
        ("a read chunk lost its completion time", |_, m| {
            m["chunks"][0]["completedAt"] = serde_json::Value::Null;
        }),
        ("a chunk completed before it was attempted", |_, m| {
            m["chunks"][0]["completedAt"] = "2000-01-01T00:00:00+00:00".into();
        }),
        ("an unread chunk carries a completion time", |r, m| {
            // Otherwise a consistent cancellation before the last chunk.
            let last = &mut m["chunks"][3];
            last["status"] = "not-read".into();
            last["bytes"] = "".into();
            last["receivedOctets"] = 0.into();
            last["attemptedAt"] = serde_json::Value::Null;
            m["ranges"][1]["coverage"]["read"] = 0.into();
            m["ranges"][1]["coverage"]["notRead"] = 12.into();
            r["stopReason"] = "cancelled".into();
        }),
    ];
    assert_eq!(accepted(&original, cases), Vec::<&str>::new());
}

/// Sets chunk `index` (zero-based) of the successful fixture to `status`
/// with `bytes` and `error`, and moves its octets in the range coverage.
fn set_chunk(
    m: &mut serde_json::Value,
    index: usize,
    status: &str,
    bytes: &str,
    error: Option<&str>,
) {
    let chunk = &mut m["chunks"][index];
    let range = chunk["range"].as_u64().unwrap() as usize - 1;
    let octets = chunk["requestedOctets"].as_u64().unwrap();
    chunk["status"] = status.into();
    chunk["bytes"] = bytes.into();
    chunk["receivedOctets"] = (bytes.len() / 2).into();
    chunk["error"] = error.into();
    if status == "not-read" {
        chunk["attemptedAt"] = serde_json::Value::Null;
        chunk["completedAt"] = serde_json::Value::Null;
    }
    let coverage = &mut m["ranges"][range]["coverage"];
    coverage["read"] = (coverage["read"].as_u64().unwrap() - octets).into();
    let key = match status {
        "refused-ambiguous" => "refusedAmbiguous",
        "not-read" => "notRead",
        other => other,
    };
    coverage[key] = (coverage[key].as_u64().unwrap() + octets).into();
}

#[tokio::test]
async fn chunk_sequences_the_collector_cannot_produce_are_refused() {
    // Fixture chunks: range 1 = chunks 0, 1, 2 (12 + 12 + 6), range 2 = chunk 3.
    let original = bundle_export(&export().await).unwrap();
    let cases: &[(&'static str, Edit)] = &[
        ("a read after a refusal in the same range", |_, m| {
            set_chunk(
                m,
                0,
                "refused-ambiguous",
                "",
                Some("access-denied-or-memory-unavailable"),
            );
        }),
        ("a read after a partial answer in the same range", |_, m| {
            set_chunk(
                m,
                0,
                "partial",
                "0102030405060708090a0b",
                Some("unexpected-length"),
            );
        }),
        ("a partial answer without any octet", |_, m| {
            set_chunk(m, 2, "partial", "", Some("unexpected-length"));
        }),
        ("reads after an error", |r, m| {
            set_chunk(m, 1, "error", "", Some("timeout"));
            r["stopReason"] = "timeout".into();
        }),
        ("an error the stop reason does not name", |r, m| {
            set_chunk(m, 3, "error", "", Some("timeout"));
            r["stopReason"] = "cancelled".into();
        }),
        ("an error without a stop reason", |_, m| {
            set_chunk(m, 3, "error", "", Some("timeout"));
        }),
        ("a read after an unread chunk", |r, m| {
            set_chunk(m, 1, "not-read", "", None);
            r["stopReason"] = "cancelled".into();
        }),
        ("an unread chunk without a stop reason", |_, m| {
            set_chunk(m, 3, "not-read", "", None);
        }),
        ("a stop reason with every chunk read", |r, _| {
            r["stopReason"] = "timeout".into();
        }),
    ];
    assert_eq!(accepted(&original, cases), Vec::<&str>::new());
    // What the CLI adds after an unconfirmed tunnel teardown stays readable.
    let unconfirmed = rehashed(&original, |r, _| {
        r["stopReason"] = "tunnel-disconnect-unconfirmed".into();
    });
    read_export(&unconfirmed).expect("an unconfirmed tunnel teardown is no contradiction");
}

/// Test-only memory faults, injected between the simulator and the client.
#[derive(Clone, Copy)]
enum Fault {
    None,
    /// Answer one octet short at this address.
    ShortAt(u16),
    /// Drop the answer at this address.
    SilentAt(u16),
}
struct Faulty {
    device: SimulatedDevice,
    tx: tokio::sync::broadcast::Sender<knx_net::TunnelEvent>,
    fault: Fault,
}
impl Faulty {
    fn new(device: SimulatedDevice, fault: Fault) -> Self {
        Self {
            device,
            tx: tokio::sync::broadcast::channel(256).0,
            fault,
        }
    }
}
impl knx_net::ManagementTransport for Faulty {
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        self.device.assigned_address()
    }
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<knx_net::TunnelEvent> {
        self.tx.subscribe()
    }
    fn target_kind(&self) -> knx_core::commissioning::mutation::TargetKind {
        knx_core::commissioning::mutation::TargetKind::Simulator
    }
    async fn send_frame(
        &self,
        destination: knx_net::Destination,
        tpci: knx_net::Tpci,
        service: knx_net::ApplicationService,
    ) -> Result<(), knx_net::BusError> {
        let mut rx = self.device.subscribe();
        self.device.send_frame(destination, tpci, service).await?;
        while let Ok(mut event) = rx.try_recv() {
            if let knx_net::TunnelEvent::Telegram(ref mut frame) = event {
                if let knx_net::ApplicationService::MemoryResponse { address, data } =
                    &mut frame.service
                {
                    match self.fault {
                        Fault::SilentAt(at) if *address == at => continue,
                        Fault::ShortAt(at) if *address == at => {
                            data.pop();
                        }
                        _ => {}
                    }
                }
            }
            let _ = self.tx.send(event);
        }
        Ok(())
    }
}

/// Collector → bundle → create-only write → independent reread, identical.
async fn written_and_reread(transport: &Faulty, cancel_after_first_chunk: bool) -> Export {
    let (_dir, db) = products(PROGRAM);
    let plan = prepare_memory(&project(), DeviceId(7), &db).unwrap();
    let cancel = AtomicBool::new(false);
    let export = acquire_memory(
        transport,
        &plan,
        knx_net::SessionTiming {
            response_timeout: std::time::Duration::from_millis(10),
            connection_timeout: std::time::Duration::from_millis(10),
            ..Default::default()
        },
        "synthetic-test",
        &cancel,
        |e| {
            let first = &e.memory.as_ref().unwrap().chunks[0];
            if cancel_after_first_chunk && first.completed_at.is_some() {
                cancel.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        },
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("evidence.zip");
    write_new_export(&path, &export).unwrap();
    assert_eq!(read_export(&std::fs::read(&path).unwrap()).unwrap(), export);
    export
}

fn statuses(export: &Export) -> Vec<&str> {
    let chunks = &export.memory.as_ref().unwrap().chunks;
    chunks.iter().map(|c| c.status.as_str()).collect()
}

#[tokio::test]
async fn real_refused_partial_interrupted_and_mismatched_acquisitions_round_trip() {
    let refusing = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        protected_memory: Some((0x0122, 0x0123)),
        ..Default::default()
    });
    refusing.preset_property(0, 12, &[0x00, 0x01]);
    refusing.preset_property(0, 78, &[0, 1, 0, 0, 0, 0x5B]);
    refusing.preset_memory(0x0116, &(1..=30).collect::<Vec<u8>>());
    refusing.preset_memory(0x3C00, &[0xEE; 12]);
    let e = written_and_reread(&Faulty::new(refusing, Fault::None), false).await;
    assert_eq!(
        statuses(&e),
        ["read", "refused-ambiguous", "not-read", "read"]
    );
    assert_eq!(e.report.stop_reason, None);

    let e = written_and_reread(&Faulty::new(device(), Fault::ShortAt(0x0116)), false).await;
    assert_eq!(statuses(&e), ["partial", "not-read", "not-read", "read"]);
    assert_eq!(e.memory.as_ref().unwrap().chunks[0].received_octets, 11);
    assert_eq!(e.report.stop_reason, None);

    let e = written_and_reread(&Faulty::new(device(), Fault::SilentAt(0x0122)), false).await;
    assert_eq!(statuses(&e), ["read", "error", "not-read", "not-read"]);
    assert_eq!(e.report.stop_reason.as_deref(), Some("timeout"));

    let e = written_and_reread(&Faulty::new(device(), Fault::None), true).await;
    assert_eq!(statuses(&e), ["read", "not-read", "not-read", "not-read"]);
    assert_eq!(e.report.stop_reason.as_deref(), Some("cancelled"));

    let other = device();
    other.preset_property(0, 12, &[0x00, 0x02]);
    let e = written_and_reread(&Faulty::new(other, Fault::None), false).await;
    assert_eq!(statuses(&e), ["not-read"; 4]);
    assert_eq!(e.report.stop_reason.as_deref(), Some("identity-mismatch"));
}

/// Puts a `decoy` member with other content before every genuine member,
/// then renames it to `name` in the raw bytes (local header and central
/// directory), so the archive holds two `name` entries, the later genuine.
fn duplicated(original: &[u8], name: &str, decoy: &str, content: &[u8]) -> Vec<u8> {
    assert_eq!(name.len(), decoy.len());
    let mut input = zip::ZipArchive::new(Cursor::new(original)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(vec![]));
    output
        .start_file(decoy, zip::write::SimpleFileOptions::default())
        .unwrap();
    output.write_all(content).unwrap();
    for i in 0..input.len() {
        let mut f = input.by_index(i).unwrap();
        let member = f.name().to_string();
        let mut data = vec![];
        f.read_to_end(&mut data).unwrap();
        output
            .start_file(member, zip::write::SimpleFileOptions::default())
            .unwrap();
        output.write_all(&data).unwrap();
    }
    let mut bytes = output.finish().unwrap().into_inner();
    let at: Vec<usize> = bytes
        .windows(decoy.len())
        .enumerate()
        .filter(|(_, w)| *w == decoy.as_bytes())
        .map(|(i, _)| i)
        .collect();
    assert_eq!(at.len(), 2, "decoy name once in each header");
    for i in at {
        bytes[i..i + name.len()].copy_from_slice(name.as_bytes());
    }
    bytes
}

#[tokio::test]
async fn a_duplicated_memory_member_is_refused_even_if_one_copy_matches() {
    let original = bundle_export(&export().await).unwrap();
    let forged = duplicated(
        &original,
        "memory.json",
        "memorz.json",
        b"{\"forged\":true}",
    );
    assert_eq!(
        read_export(&forged).expect_err("accepted two memory.json entries"),
        "diagnostics-duplicate-member"
    );
}

#[test]
fn a_duplicated_format_1_report_member_is_refused() {
    let plan = prepare(&project(), DeviceId(7), None).unwrap();
    let original = bundle(&pending(&plan, "synthetic", true)).unwrap();
    let forged = duplicated(
        &original,
        "report.json",
        "reporz.json",
        b"{\"forged\":true}",
    );
    assert_eq!(
        read_bundle(&forged).expect_err("accepted two report.json entries"),
        "diagnostics-duplicate-member"
    );
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}
fn u16_at(bytes: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()))
}

/// The competing-EOCD construction: an outer end record whose comment ends
/// exactly at the end of the archive points to a directory of unique names;
/// its comment holds an inner end record, followed by padding, that points
/// to the directory of [`duplicated`] with two `name` entries.
/// A reader trusting the outer record sees unique names, while a reader
/// that accepts a comment ending before the end of the file picks the inner.
fn competing_directories(original: &[u8], name: &str, decoy: &str) -> Vec<u8> {
    let inner = duplicated(original, name, decoy, b"{\"forged\":true}");
    let inner_end = inner.len() - 22;
    assert_eq!(u16_at(&inner, inner_end + 20), 0, "writer adds no comment");
    let (size, start) = (
        u32_at(&inner, inner_end + 12),
        u32_at(&inner, inner_end + 16),
    );
    let mut headers: Vec<(&[u8], &[u8])> = vec![];
    let mut at = start;
    while at < start + size {
        let n = u16_at(&inner, at + 28);
        let next = at + 46 + n + u16_at(&inner, at + 30) + u16_at(&inner, at + 32);
        headers.push((&inner[at + 46..at + 46 + n], &inner[at..next]));
        at = next;
    }
    // One header per name, in first-seen order, the last copy winning: the
    // view `ZipArchive` presents of the inner directory.
    let headers_len = headers.len();
    let mut unique: Vec<(&[u8], &[u8])> = vec![];
    for (n, header) in headers {
        match unique.iter_mut().find(|(seen, _)| *seen == n) {
            Some(slot) => slot.1 = header,
            None => unique.push((n, header)),
        }
    }
    let count = unique.len() as u16;
    assert_eq!(headers_len, usize::from(count) + 1, "exactly one duplicate");
    let mut bytes = inner[..inner_end].to_vec();
    let outer_start = bytes.len();
    for (_, header) in &unique {
        bytes.extend_from_slice(header);
    }
    let outer_size = bytes.len() - outer_start;
    let mut comment = inner[inner_end..].to_vec();
    comment.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(b"PK\x05\x06\0\0\0\0");
    bytes.extend_from_slice(&[count.to_le_bytes(), count.to_le_bytes()].concat());
    bytes.extend_from_slice(&(outer_size as u32).to_le_bytes());
    bytes.extend_from_slice(&(outer_start as u32).to_le_bytes());
    bytes.extend_from_slice(&(comment.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&comment);
    // The premise: `ZipArchive` really reads the inner, duplicated directory.
    let zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    assert_eq!(zip.central_directory_start(), start as u64);
    assert_eq!(zip.len(), usize::from(count));
    bytes
}

#[tokio::test]
async fn a_competing_end_record_cannot_hide_a_duplicated_memory_member() {
    let original = bundle_export(&export().await).unwrap();
    let forged = competing_directories(&original, "memory.json", "memorz.json");
    assert_eq!(
        read_export(&forged).expect_err("accepted a competing end record"),
        "diagnostics-invalid-zip"
    );
}

#[test]
fn a_competing_end_record_cannot_hide_a_duplicated_format_1_report() {
    let plan = prepare(&project(), DeviceId(7), None).unwrap();
    let original = bundle(&pending(&plan, "synthetic", true)).unwrap();
    let forged = competing_directories(&original, "report.json", "reporz.json");
    assert_eq!(
        read_bundle(&forged).expect_err("accepted a competing end record"),
        "diagnostics-invalid-zip"
    );
}

#[tokio::test]
async fn the_fixed_archive_format_carries_no_zip_comment() {
    let mut bytes = bundle_export(&export().await).unwrap();
    let end = bytes.len() - 22;
    bytes[end + 20..end + 22].copy_from_slice(&5u16.to_le_bytes());
    bytes.extend_from_slice(b"hello");
    assert_eq!(
        zip::ZipArchive::new(Cursor::new(&bytes)).unwrap().comment(),
        b"hello"
    );
    assert_eq!(
        read_export(&bytes).expect_err("accepted a ZIP comment"),
        "diagnostics-invalid-zip"
    );
}
